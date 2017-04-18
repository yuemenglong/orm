use entity::Entity;
use cond::Cond;
use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;
use std::rc::Rc;
use std::cell::RefCell;

// Select::from<E>().wher(Cond::by_id()).join("b")

#[derive(Clone)]
pub struct Select {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    cond: Option<Cond>,
    joins: Vec<(String, String, String, Rc<RefCell<Select>>)>, // (field, field, select)
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
    pub fn to_sql(&self) {}
    // select [A.a, B.b] from [A_t as A] join [B_t as B on A.a_id = B.id] where A.id > 10
    pub fn get_tables(&self) -> Vec<String> {
        let a_meta = self.meta;
        let a_table = &a_meta.table_name;
        let a_entity = &a_meta.entity_name;
        let mut vec = self.get_join_tables(a_entity);
        let table_sql = format!("{} as {}", a_table, a_entity);
        vec.insert(0, table_sql);
        vec
    }
    pub fn get_join_tables(&self, alias: &str) -> Vec<String> {
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
                let mut vec = rc.borrow().get_join_tables(&b_alias);
                vec.insert(0, join_sql);
                vec
            })
            .collect::<Vec<_>>()
    }
}
