#[macro_use]
use macros;

use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;
use cond::Cond;
use cond::JoinCond;
use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;
use value::FieldValue;

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use mysql::Value;
use mysql::prelude::GenericConnection;
use mysql::Row;
use mysql::Error;


// Select::from<E1>().join::<E2>().on(JoinCond::by_eq("id", "id"))
// select E1.id , E1_E2.id from E1 as E1 join E2 as E1_E2

pub struct Select {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    alias: String,
    cond: Option<Cond>,
    withs: Vec<(String, Select)>,
    joins: Vec<Join>, // ("INNER", )
}

pub struct Join {
    kind: JoinKind,
    cond: JoinCond,
    select: Select,
}

impl Join {
    fn new(kind: JoinKind, cond: JoinCond, select: Select) -> Self {
        Join {
            kind: kind,
            cond: cond,
            select: select,
        }
    }
    pub fn on(&mut self, cond: &JoinCond) {
        self.cond = cond.clone();
    }
    pub fn wher(&mut self, cond: &Cond) {
        self.select.wher(cond)
    }

    pub fn with(&mut self, field: &str) -> &mut Select {
        self.select.with(field)
    }
}

enum JoinKind {
    Inner,
    Outer,
    Left,
    Right,
}

fn dup_filter(vec: &mut Vec<EntityInnerPointer>) {
    let copy = vec.clone();
    vec.clear();
    let mut map = HashMap::new();
    for rc in copy {
        let id = rc.borrow().get_id_u64().unwrap();
        if !map.contains_key(&id) {
            vec.push(rc.clone());
        }
        map.entry(id).or_insert(rc.clone());
    }
    for rc in vec.iter() {
        for field_meta in rc.borrow().meta.get_one_one_fields() {
            rc.borrow_mut()
                .field_map
                .get_mut(&field_meta.get_field_name())
                .map(|v| dup_filter(v.as_vec_mut()));
        }
        // for (_, ref mut om_vec) in rc.borrow_mut().one_many_map.iter_mut() {
        //     dup_filter(om_vec);
        // }
        // for (_, ref mut mm_vec) in rc.borrow_mut().many_many_map.iter_mut() {
        //     dup_filter_pair(mm_vec);
        // }
    }
}
// fn dup_filter_pair(vec: &mut Vec<(Option<EntityInnerPointer>, EntityInnerPointer)>) {
//     let copy = vec.clone();
//     vec.clear();
//     let mut map = HashMap::new();
//     for (mid, rc) in copy {
//         let id = rc.borrow().get_id_u64().unwrap();
//         if !map.contains_key(&id) {
//             vec.push((mid.clone(), rc.clone()));
//         }
//         map.entry(id).or_insert(rc.clone());
//     }
//     for &(_, ref rc) in vec.iter() {
//         for (_, ref mut om_vec) in rc.borrow_mut().one_many_map.iter_mut() {
//             dup_filter(om_vec);
//         }
//         for (_, ref mut mm_vec) in rc.borrow_mut().many_many_map.iter_mut() {
//             dup_filter_pair(mm_vec);
//         }
//     }
// }

impl Select {
    pub fn from<E>() -> Self
        where E: Entity
    {
        Self::from_meta(E::meta(), E::orm_meta())
    }
    pub fn from_meta(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> Self {
        Select {
            meta: meta,
            orm_meta: orm_meta,
            alias: meta.entity_name.to_lowercase(),
            cond: None,
            withs: Vec::new(),
            joins: Vec::new(),
        }
    }
    fn from_alias(meta: &'static EntityMeta, orm_meta: &'static OrmMeta, alias: String) -> Self {
        Select {
            meta: meta,
            orm_meta: orm_meta,
            alias: alias,
            cond: None,
            withs: Vec::new(),
            joins: Vec::new(),
        }
    }

    pub fn wher(&mut self, cond: &Cond) {
        self.cond = Some(cond.clone());
    }

    pub fn with(&mut self, field: &str) -> &mut Select {
        let a = self;
        let field_meta = a.meta.field_map.get(field).expect(&expect!());
        let b_entity = field_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();

        let alias = format!("{}_{}", &a.alias, field);
        let select = Select::from_alias(b_meta, a.orm_meta, alias);
        a.withs.push((field.to_string(), select));
        &mut a.withs.last_mut().unwrap().1
    }
    pub fn join<E>(&mut self, cond: &JoinCond) -> &mut Join
        where E: Entity
    {
        let alias = format!("{}_{}", self.alias, E::meta().entity_name);
        let select = Select::from_alias(E::meta(), E::orm_meta(), alias);
        let kind = JoinKind::Inner;
        let join = Join::new(kind, cond.clone(), select);
        self.joins.push(join);
        self.joins.last_mut().unwrap()
    }
}

impl Select {
    pub fn query<E, C>(&self, conn: &mut C) -> Result<Vec<E>, Error>
        where E: Entity,
              C: GenericConnection
    {
        self.query_inner(conn).map(|vec| vec.into_iter().map(E::from_inner).collect::<_>())
    }

    pub fn query_inner<C>(&self, conn: &mut C) -> Result<Vec<EntityInnerPointer>, Error>
        where C: GenericConnection
    {
        let sql = self.get_sql();
        let params = self.get_params();
        println!("{}", sql);
        println!("\t{:?}", params);
        let res = conn.prep_exec(sql, params);
        let a_meta = self.meta;
        let alias = &a_meta.entity_name;
        if res.is_err() {
            return Err(res.err().unwrap());
        }
        let query_result = res.unwrap();
        let mut map = HashMap::new();
        let ret = query_result.into_iter().fold(Ok(Vec::new()), |mut acc, mut item| {
            if acc.is_err() {
                return acc;
            }
            if item.is_err() {
                return Err(item.err().unwrap());
            }
            let mut row = item.as_mut().unwrap();
            let rc = self.pick_inner(&mut row, &mut map);
            if rc.is_some() {
                acc.as_mut().unwrap().push(rc.unwrap().clone());
            }
            return acc;
        });
        ret.map(|mut vec| {
            dup_filter(&mut vec);
            vec
        })
    }
    fn pick_self(&self,
                 row: &mut Row,
                 map: &mut HashMap<String, EntityInnerPointer>)
                 -> Option<EntityInnerPointer> {
        let a_rc = EntityInner::new_pointer(self.meta, self.orm_meta);
        for field_meta in self.meta.get_non_refer_fields() {
            let field = field_meta.get_field_name();
            let key = format!("{}${}", self.alias, field);
            row.get::<Value, &str>(&key).map(|value| {
                let field_value = FieldValue::from(value);
                a_rc.borrow_mut().field_map.insert(field, field_value);
                // self.set_value(&field, Some(value));
            });
        }
        if a_rc.borrow().get_id_u64().is_none() {
            return None;
        }
        // 写入map防止重复对象
        let key = format!("{}_{}", self.alias, a_rc.borrow().get_id_u64().unwrap());
        map.entry(key.clone()).or_insert(a_rc.clone());
        let a_rc = map.get(&key).unwrap().clone();
        Some(a_rc)
    }
    fn pick_inner(&self,
                  row: &mut Row,
                  map: &mut HashMap<String, EntityInnerPointer>)
                  -> Option<EntityInnerPointer> {
        let alias = &self.alias;
        let a_meta = self.meta;
        let a_rc = self.pick_self(row, map);
        if a_rc.is_none() {
            return None;
        }
        let a_rc = a_rc.unwrap();

        for &(ref a_b_field, ref select) in self.withs.iter() {
            let field_meta = a_meta.field_map.get(a_b_field).unwrap();
            let b_rc = select.pick_inner(row, map);
            match field_meta {
                &FieldMeta::Id { .. } |
                &FieldMeta::Integer { .. } |
                &FieldMeta::String { .. } => unreachable!(),
                &FieldMeta::Refer { .. } |
                &FieldMeta::Pointer { .. } |
                &FieldMeta::OneToOne { .. } => {
                    a_rc.borrow_mut()
                        .field_map
                        .insert(a_b_field.to_string(), FieldValue::from(b_rc));
                }
                &FieldMeta::OneToMany { .. } => {
                    // 保证数据存在
                    a_rc.borrow_mut()
                        .field_map
                        .entry(a_b_field.to_string())
                        .or_insert(FieldValue::from(Vec::new()));
                    if b_rc.is_some() {
                        a_rc.borrow_mut()
                            .field_map
                            .get_mut(a_b_field)
                            .unwrap()
                            .as_vec_mut()
                            .push(b_rc.unwrap());
                    }
                }
            }
        }
        Some(a_rc)
    }
    // select [A.a as A_a, B.b as B_b] from [A_t as A] join [B_t as B on A.a_id = B.id] where A.id > 10
    pub fn get_sql(&self) -> String {
        let columns = self.get_columns()
            .into_iter()
            .map(|vec| vec.join(",\n\t"))
            .collect::<Vec<_>>()
            .join(",\n\n\t");
        let tables = self.get_tables().join("\n\t");
        let conds = self.get_conds().join("\n\tAND\n\t");
        format!("SELECT\n\t{}\nFROM\n\t{}\nWHERE\n\t{}",
                columns,
                tables,
                conds)
    }
    pub fn get_params(&self) -> Vec<(String, Value)> {
        self.inner_get_params()
    }
    pub fn get_conds(&self) -> Vec<String> {
        let mut vec = self.inner_get_conds();
        if vec.len() == 0 {
            vec.push("1 = 1".to_string());
        }
        vec
    }
    pub fn get_tables(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_entity = &a_meta.entity_name;
        let a_table = &a_meta.table_name;
        let mut vec = self.inner_get_tables();
        let sql = format!("{} as {}", a_table, a_entity);
        vec.insert(0, sql);
        vec
    }
    pub fn get_columns(&self) -> Vec<Vec<String>> {
        self.inner_get_columns()
    }

    fn inner_get_params(&self) -> Vec<(String, Value)> {
        let alias = &self.alias;
        let vec = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_params())
            .collect::<Vec<_>>();
        if self.cond.is_some() {
            let cond = self.cond.as_ref().unwrap();
            let mut ret = cond.to_params(alias);
            ret.extend(vec);
            return ret;
            // vec.insert(0, cond.to_params(alias));
        } else {
            return vec;
        }
    }
    fn inner_get_conds(&self) -> Vec<String> {
        let alias = &self.alias;
        let mut vec = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_conds())
            .collect::<Vec<_>>();
        if self.cond.is_some() {
            let cond = self.cond.as_ref().unwrap();
            vec.insert(0, cond.to_sql(alias));
        }
        vec
    }
    fn inner_get_columns(&self) -> Vec<Vec<String>> {
        let alias = &self.alias;
        let mut vec = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_columns())
            .collect::<Vec<_>>();
        let self_columns = self.meta
            .get_non_refer_fields()
            .into_iter()
            .map(|field_meta| {
                let column = field_meta.get_column_name();
                let field = field_meta.get_field_name();
                format!("{}.{} AS {}${}", alias, column, alias, field)
            })
            .collect::<Vec<_>>();
        vec.insert(0, self_columns);
        vec
    }
    fn inner_get_tables(&self) -> Vec<String> {
        let alias = &self.alias;
        let a_meta = self.meta;
        let with_tables = self.withs
            .iter()
            .flat_map(|&(ref a_b_field, ref select)| {
                let a_b_meta = a_meta.field_map.get(a_b_field).unwrap();
                let b_alias = format!("{}_{}", alias, a_b_field);
                let b_meta = select.meta;
                let b_table = &b_meta.table_name;
                let mut vec = select.inner_get_tables();
                let (a_field, b_field) = a_b_meta.get_refer_lr();
                let a_column = a_meta.field_map.get(&a_field).unwrap().get_column_name();
                let b_column = b_meta.field_map.get(&b_field).unwrap().get_column_name();
                let join_sql = format!("LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                                       b_table,
                                       b_alias,
                                       alias,
                                       a_column,
                                       b_alias,
                                       b_column);
                vec.insert(0, join_sql);
                vec
            })
            .collect::<Vec<_>>();
        let join_tables = self.joins
            .iter()
            .map(|join| {
                let b_meta = join.select.meta;
                let b_entity = &b_meta.entity_name;
                let b_table = &b_meta.table_name;
                let b_alias = format!("{}_{}", alias, b_entity);
                let join_cond = join.cond.to_sql(alias, &b_alias);
                let join_sql = format!("LEFT JOIN {} AS {} ON {}", b_table, b_alias, join_cond);
                join_sql
            })
            .collect::<Vec<_>>();
        with_tables
    }
}
