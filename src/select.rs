use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;
use cond::Cond;
use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use mysql::Value;
use mysql::PooledConn;
use mysql::Row;
use mysql::Error;

// Select::from<E>().wher(Cond::by_id()).join("b")

#[derive(Clone)]
pub struct Select {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    cond: Option<Cond>,
    joins: Vec<(String, Rc<RefCell<Select>>)>, // (a_field, b_field, a_b_field, select)
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
        for (_, ref mut om_vec) in rc.borrow_mut().one_many_map.iter_mut() {
            dup_filter(om_vec);
        }
        for (_, ref mut mm_vec) in rc.borrow_mut().many_many_map.iter_mut() {
            dup_filter_pair(mm_vec);
        }
    }
}
fn dup_filter_pair(vec: &mut Vec<(Option<EntityInnerPointer>, EntityInnerPointer)>) {
    let copy = vec.clone();
    vec.clear();
    let mut map = HashMap::new();
    for (mid, rc) in copy {
        let id = rc.borrow().get_id_u64().unwrap();
        if !map.contains_key(&id) {
            vec.push((mid.clone(), rc.clone()));
        }
        map.entry(id).or_insert(rc.clone());
    }
    for &(_, ref rc) in vec.iter() {
        for (_, ref mut om_vec) in rc.borrow_mut().one_many_map.iter_mut() {
            dup_filter(om_vec);
        }
        for (_, ref mut mm_vec) in rc.borrow_mut().many_many_map.iter_mut() {
            dup_filter_pair(mm_vec);
        }
    }
}

impl Select {
    pub fn from<E>() -> Self
        where E: Entity
    {
        Select {
            meta: E::meta(),
            orm_meta: E::orm_meta(),
            cond: None,
            joins: Vec::new(),
        }
    }
    pub fn from_meta(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> Self {
        Select {
            meta: meta,
            orm_meta: orm_meta,
            cond: None,
            joins: Vec::new(),
        }
    }
    pub fn wher(&mut self, cond: &Cond) -> &Self {
        self.cond = Some(cond.clone());
        self
    }

    pub fn join(&mut self, field: &str) -> Rc<RefCell<Select>> {
        let a = self;
        let field_meta =
            a.meta.field_map.get(field).expect(&format!("Join Field Not Exists: {}", field));
        let b_entity = field_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
        let rc = Rc::new(RefCell::new(Select::from_meta(b_meta, a.orm_meta)));

        let a_b_field = field_meta.get_field_name();
        a.joins.push((a_b_field, rc.clone()));
        return rc;
    }

    pub fn query<E>(&self, conn: &mut PooledConn) -> Result<Vec<E>, Error>
        where E: Entity
    {
        self.query_inner(conn).map(|vec| vec.into_iter().map(E::from_inner).collect::<_>())
    }

    pub fn query_inner(&self, conn: &mut PooledConn) -> Result<Vec<EntityInnerPointer>, Error> {
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
            let rc = self.pick_inner(alias, &mut row, &mut map);
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
    pub fn pick_inner(&self,
                      alias: &str,
                      row: &mut Row,
                      map: &mut HashMap<String, EntityInnerPointer>)
                      -> Option<EntityInnerPointer> {
        let a_meta = self.meta;
        let a_rc = EntityInner::new_pointer(self.meta, self.orm_meta);
        a_rc.borrow_mut().set_values(row, alias);
        if a_rc.borrow().get_id_u64().is_none() {
            return None;
        }
        let key = format!("{}_{}", alias, a_rc.borrow().get_id_u64().unwrap());
        map.entry(key.clone()).or_insert(a_rc.clone());
        let a_rc = map.get(&key).unwrap().clone();
        for &(ref a_b_field, ref select_rc) in self.joins.iter() {
            let field_meta = a_meta.field_map.get(a_b_field).unwrap();
            let b_alias = format!("{}_{}", alias, a_b_field);
            let b_rc = select_rc.borrow().pick_inner(&b_alias, row, map);
            if field_meta.is_refer_pointer() {
                a_rc.borrow_mut().pointer_map.insert(a_b_field.clone(), b_rc);
            } else if field_meta.is_refer_one_one() {
                a_rc.borrow_mut().one_one_map.insert(a_b_field.clone(), b_rc);
            } else if field_meta.is_refer_one_many() {
                a_rc.borrow_mut().one_many_map.entry(a_b_field.clone()).or_insert(Vec::new());
                if b_rc.is_some() {
                    a_rc.borrow_mut().one_many_map.get_mut(a_b_field).unwrap().push(b_rc.unwrap());
                }
            } else if field_meta.is_refer_many_many() {
                let mid_alias = format!("{}__{}", alias, a_b_field);
                let mid_rc = select_rc.borrow().pick_inner(&mid_alias, row, map);
                a_rc.borrow_mut().many_many_map.entry(a_b_field.clone()).or_insert(Vec::new());
                if b_rc.is_some() {
                    a_rc.borrow_mut()
                        .many_many_map
                        .get_mut(a_b_field)
                        .unwrap()
                        .push((mid_rc, b_rc.unwrap()));
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
        let tables = self.get_tables().join("\n\tLEFT JOIN\n\t");
        let conds = self.get_conds().join("\n\tAND\n\t");
        format!("SELECT\n\t{}\nFROM\n\t{}\nWHERE\n\t{}",
                columns,
                tables,
                conds)
    }
    pub fn get_params(&self) -> Vec<(String, Value)> {
        let a_meta = self.meta;
        let a_entity = &a_meta.entity_name;
        self.inner_get_params(a_entity)
    }
    pub fn get_conds(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_entity = &a_meta.entity_name;
        let mut vec = self.inner_get_conds(a_entity);
        if vec.len() == 0 {
            vec.push("1 = 1".to_string());
        }
        vec
    }
    pub fn get_tables(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_entity = &a_meta.entity_name;
        let a_table = &a_meta.table_name;
        let mut vec = self.inner_get_tables(a_entity);
        let sql = format!("{} as {}", a_table, a_entity);
        vec.insert(0, sql);
        vec
    }
    pub fn get_columns(&self) -> Vec<Vec<String>> {
        let a_meta = self.meta;
        let a_entity = &a_meta.entity_name;
        self.inner_get_columns(a_entity)
    }
    fn inner_get_params(&self, alias: &str) -> Vec<(String, Value)> {
        let vec = self.joins
            .iter()
            .flat_map(|&(ref a_b_field, ref rc)| {
                let b_alias = format!("{}_{}", alias, a_b_field);
                rc.borrow().inner_get_params(&b_alias)
            })
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
    fn inner_get_conds(&self, alias: &str) -> Vec<String> {
        let mut vec = self.joins
            .iter()
            .flat_map(|&(ref a_b_field, ref rc)| {
                let b_alias = format!("{}_{}", alias, a_b_field);
                rc.borrow().inner_get_conds(&b_alias)
            })
            .collect::<Vec<_>>();
        if self.cond.is_some() {
            let cond = self.cond.as_ref().unwrap();
            vec.insert(0, cond.to_sql(alias));
        }
        vec
    }
    fn inner_get_columns(&self, alias: &str) -> Vec<Vec<String>> {
        let mut vec = self.joins
            .iter()
            .flat_map(|&(ref a_b_field, ref rc)| {
                let b_alias = format!("{}_{}", alias, a_b_field);
                rc.borrow().inner_get_columns(&b_alias)
            })
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
    fn get_join_field(field_meta: &FieldMeta) -> (String, String) {
        if field_meta.is_refer_pointer() {
            let a_field = field_meta.get_pointer_id();
            let b_field = "id".to_string();
            return (a_field, b_field);
        } else if field_meta.is_refer_one_one() {
            let a_field = "id".to_string();
            let b_field = field_meta.get_one_one_id();
            return (a_field, b_field);
        } else if field_meta.is_refer_one_many() {
            let a_field = "id".to_string();
            let b_field = field_meta.get_one_many_id();
            return (a_field, b_field);
        } else {
            unreachable!();
        }
    }
    fn get_join_field_many_many(field_meta: &FieldMeta) -> (String, String, String, String) {
        if field_meta.is_refer_many_many() {
            let a_field = "id".to_string();
            let mid_a_field = field_meta.get_many_many_id();
            let mid_b_field = field_meta.get_many_many_refer_id();
            let b_field = "id".to_string();
            return (a_field, mid_a_field, mid_b_field, b_field);
        } else {
            unreachable!();
        }
    }
    fn inner_get_tables(&self, alias: &str) -> Vec<String> {
        let a_meta = self.meta;
        self.joins
            .iter()
            .flat_map(|&(ref a_b_field, ref rc)| {
                let a_b_meta = a_meta.field_map.get(a_b_field).unwrap();
                let b_alias = format!("{}_{}", alias, a_b_field);
                let b_meta = rc.borrow().meta;
                let b_table = &b_meta.table_name;
                let mut vec = rc.borrow().inner_get_tables(&b_alias);
                if !a_b_meta.is_refer_many_many() {
                    let (a_field, b_field) = Self::get_join_field(a_b_meta);
                    let a_column = a_meta.field_map.get(&a_field).unwrap().get_column_name();
                    let b_column = b_meta.field_map.get(&b_field).unwrap().get_column_name();
                    let join_sql = format!("{} AS {} ON {}.{} = {}.{}",
                                           b_table,
                                           b_alias,
                                           alias,
                                           a_column,
                                           b_alias,
                                           b_column);
                    vec.insert(0, join_sql);
                    vec
                } else {
                    let mid_entity = a_b_meta.get_many_many_middle_entity();
                    let mid_meta = self.orm_meta.entity_map.get(&mid_entity).unwrap();
                    let mid_table = &mid_meta.table_name;
                    let mid_alias = format!("{}__{}", alias, a_b_field);
                    let (a_field, a_mid_field, b_mid_field, b_field) =
                        Self::get_join_field_many_many(a_b_meta);
                    let a_column = a_meta.field_map.get(&a_field).unwrap().get_column_name();
                    let a_mid_column =
                        mid_meta.field_map.get(&a_mid_field).unwrap().get_column_name();
                    let b_mid_column =
                        mid_meta.field_map.get(&b_mid_field).unwrap().get_column_name();
                    let b_column = b_meta.field_map.get(&b_field).unwrap().get_column_name();
                    let a_join_mid = format!("{} AS {} ON {}.{} = {}.{}",
                                             mid_table,
                                             mid_alias,
                                             alias,
                                             a_column,
                                             mid_alias,
                                             a_mid_column);
                    let mid_join_b = format!("{} AS {} ON {}.{} = {}.{}",
                                             b_table,
                                             b_alias,
                                             mid_alias,
                                             b_mid_column,
                                             b_alias,
                                             b_column);
                    vec.insert(0, mid_join_b);
                    vec.insert(0, a_join_mid);
                    vec
                }
            })
            .collect::<Vec<_>>()
    }
}
