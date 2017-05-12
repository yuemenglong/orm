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
use mysql::conn::GenericConnection;
use mysql::Row;
use mysql::Error;

use std::marker::PhantomData;


// SelectImpl::from<E1>().join::<E2>().on(JoinCond::by_eq("id", "id"))
// select E1.id , E1_E2.id from E1 as E1 join E2 as E1_E2

#[derive(Debug)]
pub struct Select<E> {
    phantom: PhantomData<E>,
    imp: SelectImpl,
}

#[derive(Debug)]
pub struct SelectImpl {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    alias: String,
    cond: Option<Cond>,
    withs: Vec<(String, SelectImpl)>,
    joins: Vec<Join>,
}

impl<E> Select<E>
    where E: Entity
{
    pub fn new() -> Self {
        Select::<E> {
            phantom: PhantomData,
            imp: SelectImpl::from_meta(E::meta(), E::orm_meta()),
        }
    }
    pub fn wher(&mut self, cond: &Cond) -> &mut SelectImpl {
        self.imp.wher(cond)
    }

    pub fn with(&mut self, field: &str) -> &mut SelectImpl {
        self.imp.with(field)
    }
    pub fn join<Et>(&mut self, cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.imp.join::<Et>(cond)
    }
    pub fn left_join<Et>(&mut self, cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.imp.left_join::<Et>(cond)
    }
    pub fn right_join<Et>(&mut self, cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.imp.right_join::<Et>(cond)
    }
    pub fn outer_join<Et>(&mut self, cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.imp.outer_join::<Et>(cond)
    }
    pub fn query<C>(&self, conn: &mut C) -> Result<Vec<E>, Error>
        where C: GenericConnection
    {
        self.imp.query_inner(conn).map(|vec| vec.into_iter().map(E::from_inner).collect())
    }
    pub fn query_ex<C>(&self, conn: &mut C) -> Result<Vec<Vec<E>>, Error>
        where C: GenericConnection
    {
        self.imp
            .query_inner_ex(conn)
            .map(|tuple| {
                tuple.into_iter().map(|vec| vec.into_iter().map(E::from_inner).collect()).collect()
            })
    }
}

impl SelectImpl {
    pub fn from_meta(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> Self {
        SelectImpl {
            meta: meta,
            orm_meta: orm_meta,
            alias: meta.alias.clone(),
            cond: None,
            withs: Vec::new(),
            joins: Vec::new(),
        }
    }
    fn from_alias(meta: &'static EntityMeta, orm_meta: &'static OrmMeta, alias: String) -> Self {
        SelectImpl {
            meta: meta,
            orm_meta: orm_meta,
            alias: alias,
            cond: None,
            withs: Vec::new(),
            joins: Vec::new(),
        }
    }

    pub fn wher(&mut self, cond: &Cond) -> &mut Self {
        self.cond = Some(cond.clone());
        self
    }

    pub fn with(&mut self, field: &str) -> &mut Self {
        let a = self;
        let field_meta = a.meta.field_map.get(field).expect(&expect!());
        let b_entity = field_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();

        let alias = format!("{}_{}", &a.alias, field);
        let select = SelectImpl::from_alias(b_meta, a.orm_meta, alias);
        a.withs.push((field.to_string(), select));
        &mut a.withs.last_mut().unwrap().1
    }
    pub fn join<E>(&mut self, cond: &JoinCond) -> &mut Join
        where E: Entity
    {
        self.join_impl::<E>(cond, JoinKind::Inner)
    }
    pub fn outer_join<E>(&mut self, cond: &JoinCond) -> &mut Join
        where E: Entity
    {
        self.join_impl::<E>(cond, JoinKind::Outer)
    }
    pub fn left_join<E>(&mut self, cond: &JoinCond) -> &mut Join
        where E: Entity
    {
        self.join_impl::<E>(cond, JoinKind::Left)
    }
    pub fn right_join<E>(&mut self, cond: &JoinCond) -> &mut Join
        where E: Entity
    {
        self.join_impl::<E>(cond, JoinKind::Right)
    }
    fn join_impl<E>(&mut self, cond: &JoinCond, kind: JoinKind) -> &mut Join
        where E: Entity
    {
        let alias = format!("{}_{}", self.alias, E::meta().alias.clone());
        let select = SelectImpl::from_alias(E::meta(), E::orm_meta(), alias);
        let join = Join::new(kind, cond.clone(), select);
        self.joins.push(join);
        self.joins.last_mut().unwrap()
    }
    fn flat_select(&self) -> Vec<&SelectImpl> {
        let mut subs =
            self.joins.iter().flat_map(|join| join.select.flat_select()).collect::<Vec<_>>();
        let mut ret = vec![self];
        ret.append(&mut subs);
        ret
    }
}

impl SelectImpl {
    pub fn query_inner<C>(&self, conn: &mut C) -> Result<Vec<EntityInnerPointer>, Error>
        where C: GenericConnection
    {
        self.query_inner_ex(conn).map(|mut vec| vec.remove(0))
    }
    pub fn query_inner_ex<C>(&self, conn: &mut C) -> Result<Vec<Vec<EntityInnerPointer>>, Error>
        where C: GenericConnection
    {
        let sql = self.get_sql();
        let params = self.get_params();
        log!("{}", sql);
        log!("\t{:?}", params);
        let res = match params.len() {
            0 => conn.prep_exec(sql, ()),
            _ => conn.prep_exec(sql, params),
        };
        let a_meta = self.meta;
        let alias = &a_meta.alias;
        if res.is_err() {
            return Err(res.err().unwrap());
        }
        let mut map = HashMap::new();
        let query_result = res.unwrap();
        let mut selects = self.flat_select();
        let ret = selects.iter().map(|_| Vec::new()).collect::<Vec<_>>();
        query_result.into_iter()
            .fold(Ok(ret), |mut acc, mut item| {
                if acc.is_err() {
                    return acc;
                }
                if item.is_err() {
                    return Err(item.err().unwrap());
                }
                let mut acc = acc.unwrap();
                let mut row = item.as_mut().unwrap();
                // 循环每个select读取
                for (i, select) in selects.iter().enumerate() {
                    let rc = select.pick_inner(&mut row, &mut map);
                    rc.map(|rc| acc.get_mut(i).unwrap().push(rc));
                }
                return Ok(acc);
            })
            // 过滤重复数据
            .map(|tuple| {
                tuple.into_iter()
                    .map(|mut vec| {
                        dup_filter(&mut vec);
                        vec
                    })
                    .collect::<Vec<_>>()
            })

        // ret.map(|mut vec| {
        //     dup_filter(&mut vec);
        //     vec
        // })
        // Ok(Vec::new())
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
        let conds = self.get_conds().join("\n\tAND ");
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
        let a_entity = &a_meta.entity;
        let a_table = &a_meta.table;
        let self_table = format!("{} as {}", a_table, &self.alias);
        let mut tables = self.inner_get_tables();
        tables.insert(0, self_table);
        tables
    }
    pub fn get_columns(&self) -> Vec<Vec<String>> {
        self.inner_get_columns()
    }

    fn inner_get_params(&self) -> Vec<(String, Value)> {
        let alias = &self.alias;
        let mut with_params = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_params())
            .collect::<Vec<_>>();
        let mut join_params = self.joins
            .iter()
            .flat_map(|join| {
                join.on_cond.as_ref().map_or(Vec::new(), |cond| cond.to_params(&join.select.alias))
            })
            .collect::<Vec<_>>();
        let mut join_select_params = self.joins
            .iter()
            .flat_map(|join| join.select.inner_get_params())
            .collect::<Vec<_>>();
        let mut ret = self.cond.as_ref().map_or(Vec::new(), |cond| cond.to_params(alias));
        ret.append(&mut with_params);
        ret.append(&mut join_params);
        ret.append(&mut join_select_params);
        ret
    }
    fn inner_get_conds(&self) -> Vec<String> {
        let alias = &self.alias;
        let mut with_conds = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_conds())
            .collect::<Vec<_>>();
        let mut join_select_cond = self.joins
            .iter()
            .flat_map(|join| join.select.inner_get_conds())
            .collect::<Vec<_>>();
        let mut ret = self.cond.as_ref().map_or(Vec::new(), |cond| vec![cond.to_sql(alias)]);
        ret.append(&mut with_conds);
        ret.append(&mut join_select_cond);
        ret
    }
    fn inner_get_columns(&self) -> Vec<Vec<String>> {
        let alias = &self.alias;
        let self_columns = self.meta
            .get_non_refer_fields()
            .into_iter()
            .map(|field_meta| {
                let column = field_meta.get_column_name();
                let field = field_meta.get_field_name();
                format!("{}.{} AS {}${}", alias, column, alias, field)
            })
            .collect::<Vec<_>>();
        let mut with_columns = self.withs
            .iter()
            .flat_map(|&(_, ref select)| select.inner_get_columns())
            .collect::<Vec<_>>();
        let mut join_columns =
            self.joins.iter().flat_map(|join| join.select.inner_get_columns()).collect::<Vec<_>>();
        let mut ret = vec![self_columns];
        ret.append(&mut with_columns);
        ret.append(&mut join_columns);
        ret
    }
    fn inner_get_tables(&self) -> Vec<String> {
        let alias = &self.alias;
        let a_meta = self.meta;
        let mut with_tables = self.withs
            .iter()
            .flat_map(|&(ref a_b_field, ref select)| {
                let a_b_meta = a_meta.field_map.get(a_b_field).unwrap();
                let b_alias = format!("{}_{}", alias, a_b_field);
                let b_meta = select.meta;
                let b_table = &b_meta.table;
                let mut vec = select.inner_get_tables();
                let (a_field, b_field) = a_b_meta.get_refer_lr();
                let a_column = a_meta.field_map.get(&a_field).unwrap().get_column_name();
                let b_column = b_meta.field_map.get(&b_field).unwrap().get_column_name();
                let join_table = format!("LEFT JOIN {} AS {} ON {}.{} = {}.{}",
                                         b_table,
                                         b_alias,
                                         alias,
                                         a_column,
                                         b_alias,
                                         b_column);
                vec.insert(0, join_table);
                vec
            })
            .collect::<Vec<_>>();
        let mut join_tables = self.joins
            .iter()
            .flat_map(|join| {
                let b_meta = join.select.meta;
                let b_entity = &b_meta.entity;
                let b_table = &b_meta.table;
                let b_alias = &join.select.alias;
                let join_cond = join.join_cond.to_sql(alias, &b_alias);
                let on_cond = join.on_cond.as_ref().map_or("".to_string(), |cond| {
                    format!(" AND {}", cond.to_sql(&b_alias))
                });
                let cond = vec![join_cond, on_cond].join("");
                let join_kind = join.kind.to_sql();
                let join_table = format!("{} {} AS {} ON {}", join_kind, b_table, b_alias, cond);
                let mut ret = vec![join_table];
                let mut subs = join.select.inner_get_tables();
                ret.append(&mut subs);
                ret
            })
            .collect::<Vec<_>>();
        let mut ret: Vec<String> = Vec::new();
        ret.append(&mut with_tables);
        ret.append(&mut join_tables);
        ret
    }
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
        let fields = {
            rc.borrow()
                .meta
                .get_one_many_fields()
                .into_iter()
                .map(FieldMeta::get_field_name)
                .collect::<Vec<_>>()
        };
        for field_name in fields.into_iter() {
            rc.borrow_mut()
                .field_map
                .get_mut(&field_name)
                .map(|v| dup_filter(v.as_vec_mut()));
        }
    }
}


#[derive(Debug)]
pub struct Join {
    kind: JoinKind,
    join_cond: JoinCond,
    on_cond: Option<Cond>,
    select: SelectImpl,
}

impl Join {
    fn new(kind: JoinKind, join_cond: JoinCond, select: SelectImpl) -> Self {
        Join {
            kind: kind,
            join_cond: join_cond,
            on_cond: None,
            select: select,
        }
    }
    pub fn on(&mut self, cond: &Cond) -> &mut Self {
        self.on_cond = Some(cond.clone());
        self
    }
}
impl Join {
    pub fn wher(&mut self, cond: &Cond) -> &mut SelectImpl {
        self.select.wher(cond)
    }
    pub fn with(&mut self, field: &str) -> &mut SelectImpl {
        self.select.with(field)
    }
    pub fn join<Et>(&mut self, join_cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.select.join::<Et>(join_cond)
    }
    pub fn left_join<Et>(&mut self, join_cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.select.left_join::<Et>(join_cond)
    }
    pub fn right_join<Et>(&mut self, join_cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.select.right_join::<Et>(join_cond)
    }
    pub fn outer_join<Et>(&mut self, join_cond: &JoinCond) -> &mut Join
        where Et: Entity
    {
        self.select.outer_join::<Et>(join_cond)
    }
}

#[derive(Debug)]
enum JoinKind {
    Inner,
    Outer,
    Left,
    Right,
}

impl JoinKind {
    pub fn to_sql(&self) -> String {
        match self {
            &JoinKind::Inner => "INNER JOIN".to_string(),
            &JoinKind::Outer => "OUTER JOIN".to_string(),
            &JoinKind::Left => "LEFT JOIN".to_string(),
            &JoinKind::Right => "RIGHT JOIN".to_string(),
        }
    }
}
