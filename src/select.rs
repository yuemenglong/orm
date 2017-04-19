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

pub struct MultiSelect {
    first: (String, Rc<RefCell<Select>>), //(a_alias)
    joins: Vec<(String, String, String, Rc<RefCell<Select>>)>, // (a_field, b_field, b_alias)
}

#[derive(Clone)]
pub struct Select {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    cond: Option<Cond>,
    joins: Vec<(String, String, String, Rc<RefCell<Select>>)>, // (a_field, b_field, a_b_field, select)
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
        if field_meta.is_refer_pointer() {
            let a_field = field_meta.get_pointer_id();
            let b_field = "id".to_string();
            a.joins.push((a_field, b_field, a_b_field, rc.clone()));
            return rc;
        } else if field_meta.is_refer_one_one() {
            let a_field = "id".to_string();
            let b_field = field_meta.get_one_one_id();
            a.joins.push((a_field, b_field, a_b_field, rc.clone()));
            return rc;
        } else if field_meta.is_refer_one_many() {
            let a_field = "id".to_string();
            let b_field = field_meta.get_one_many_id();
            a.joins.push((a_field, b_field, a_b_field, rc.clone()));
            return rc;
        } else if field_meta.is_refer_many_many() {
            let a_field = "id".to_string();
            let b_field = field_meta.get_many_many_id();
            a.joins.push((a_field, b_field, a_b_field.clone(), rc.clone()));
            let a_field = field_meta.get_many_many_refer_id();
            let b_field = "id".to_string();
            let ret = rc.borrow_mut().join_on(&a_field, &b_field, &a_b_field, b_meta);
            return ret;
        } else {
            panic!("Join Must Set Refer Field, {}", field);
        }
    }

    pub fn join_on(&mut self,
                   a_field: &str,
                   b_field: &str,
                   a_b_field: &str,
                   b_meta: &'static EntityMeta)
                   -> Rc<RefCell<Select>> {
        let a = self;
        if a.meta.field_map.get(a_field).is_none() || b_meta.field_map.get(b_field).is_none() {
            panic!("Join Invalid Field, [{}], [{}]", a_field, b_field);
        }
        let rc = Rc::new(RefCell::new(Select::from_meta(b_meta, a.orm_meta)));
        a.joins.push((a_field.to_string(), b_field.to_string(), a_b_field.to_string(), rc.clone()));
        rc
    }
    pub fn inner_query(&self, conn: &mut PooledConn) -> Result<Vec<EntityInnerPointer>, Error> {
        let sql = self.get_sql();
        let params = self.get_params();
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
            let rc = self.inner_pick(alias, &mut row, &mut map);
            if rc.is_some() {
                acc.as_mut().unwrap().push(rc.unwrap().clone());
            }
            return acc;
        });
        ret
    }
    pub fn inner_pick(&self,
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
        for &(_, _, ref a_b_field, ref select_rc) in self.joins.iter() {
            let field_meta = a_meta.field_map.get(a_b_field).unwrap();
            let b_alias = format!("{}_{}", alias, a_b_field);
            let b_rc = select_rc.borrow().inner_pick(&b_alias, row, map);
            if field_meta.is_refer_pointer() {
                a_rc.borrow_mut().pointer_map.insert(a_b_field.clone(), b_rc);
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
        let a_table = &a_meta.table_name;
        let a_entity = &a_meta.entity_name;
        self.inner_get_params(a_entity)
    }
    pub fn get_conds(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let a_entity = &a_meta.entity_name;
        let mut vec = self.inner_get_conds(a_entity);
        if vec.len() == 0 {
            vec.push("1 = 1".to_string());
        }
        vec
    }
    pub fn get_tables(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let a_entity = &a_meta.entity_name;
        let mut vec = self.inner_get_tables(a_entity);
        let sql = format!("{} as {}", a_table, a_entity);
        vec.insert(0, sql);
        vec
    }
    pub fn get_columns(&self) -> Vec<Vec<String>> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let a_entity = &a_meta.entity_name;
        self.inner_get_columns(a_entity)
    }
    fn inner_get_params(&self, alias: &str) -> Vec<(String, Value)> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let mut vec = self.joins
            .iter()
            .flat_map(|&(_, _, ref a_b_field, ref rc)| {
                let b_meta = rc.borrow().meta;
                let b_table = &b_meta.table_name;
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
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let mut vec = self.joins
            .iter()
            .flat_map(|&(_, _, ref a_b_field, ref rc)| {
                let b_meta = rc.borrow().meta;
                let b_table = &b_meta.table_name;
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
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let mut vec = self.joins
            .iter()
            .flat_map(|&(_, _, ref a_b_field, ref rc)| {
                let b_meta = rc.borrow().meta;
                let b_table = &b_meta.table_name;
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
    fn inner_get_tables(&self, alias: &str) -> Vec<String> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        self.joins
            .iter()
            .flat_map(|&(ref a_field, ref b_field, ref a_b_field, ref rc)| {
                let b_meta = rc.borrow().meta;
                let b_table = &b_meta.table_name;
                let a_column = a_meta.field_map.get(a_field).unwrap().get_column_name();
                let b_column = b_meta.field_map.get(b_field).unwrap().get_column_name();
                let b_alias = format!("{}_{}", alias, a_b_field);
                let join_sql = format!("{} AS {} ON {}.{} = {}.{}",
                                       b_table,
                                       b_alias,
                                       alias,
                                       a_column,
                                       b_alias,
                                       b_column);
                let mut vec = rc.borrow().inner_get_tables(&b_alias);
                vec.insert(0, join_sql);
                vec
            })
            .collect::<Vec<_>>()
    }
}
