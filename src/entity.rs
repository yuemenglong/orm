#[macro_use]
use macros;

use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

use mysql::Value;
use mysql::Error;
use mysql::value;
use mysql::prelude::FromValue;
use mysql::Row;
use mysql::prelude::GenericConnection;

use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;
use meta::Cascade;
use value::FieldValue;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

pub struct EntityInner {
    pub orm_meta: &'static OrmMeta,
    pub meta: &'static EntityMeta,
    pub field_map: HashMap<String, FieldValue>,

    pub cascade: Option<Cascade>, /* pub session: Option<Session>, // pub cache: Vec<(String, EntityInnerPointer)>, */
}

// 和字段编辑相关
impl EntityInner {
    pub fn new(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> EntityInner {
        EntityInner {
            orm_meta: orm_meta,
            meta: meta,
            field_map: HashMap::new(),
            cascade: None, // session: None, // cache: Vec::new(),
        }
    }
    pub fn default(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> EntityInner {
        let field_map: HashMap<String, FieldValue> = meta.field_vec
            .iter()
            .map(|field| {
                let field_meta = meta.field_map.get(field).expect(expect!().as_ref());
                let default_value = FieldValue::default(field_meta);
                (field.to_string(), default_value)
            })
            .collect::<HashMap<_, _>>();

        EntityInner {
            orm_meta: orm_meta,
            meta: meta,
            field_map: field_map,
            cascade: None, // session: None, // cache: Vec::new(),
        }
    }
    pub fn new_pointer(meta: &'static EntityMeta,
                       orm_meta: &'static OrmMeta)
                       -> EntityInnerPointer {
        Rc::new(RefCell::new(EntityInner::new(meta, orm_meta)))
    }
    pub fn default_pointer(meta: &'static EntityMeta,
                           orm_meta: &'static OrmMeta)
                           -> EntityInnerPointer {
        Rc::new(RefCell::new(EntityInner::default(meta, orm_meta)))
    }

    pub fn get_addr(&self) -> u64 {
        self as *const EntityInner as u64
    }
    pub fn get_id_value(&self) -> Value {
        self.field_map.get("id").map_or(Value::NULL, |value| value.as_value())
    }
}

// Value
impl EntityInner {
    pub fn get_value<V>(&self, field: &str) -> V
        where V: FromValue
    {
        let v = self.field_map.get(field).expect(expect!().as_ref()).as_value();
        value::from_value(v)
    }
    pub fn set_value<V>(&mut self, field: &str, value: V)
        where Value: From<V>
    {
        let v = Value::from(value);
        let field_value = FieldValue::from(v);
        self.field_map.insert(field.to_string(), field_value);
    }
    pub fn set_value_null(&mut self, field: &str) {
        let field_value = FieldValue::from(Value::NULL);
        self.field_map.insert(field.to_string(), field_value);
    }
    pub fn is_value_null(&self, field: &str) -> bool {
        self.field_map.get(field).map_or(false, |v| v.as_value() == Value::NULL)
    }
}

// Entity
impl EntityInner {
    pub fn get_entity(&self, field: &str) -> Option<EntityInnerPointer> {
        let opt = self.field_map.get(field);
        if opt.is_some() {
            return opt.unwrap().as_entity();
        }
        unreachable!();
    }
    pub fn set_entity(&mut self, field: &str, opt: Option<EntityInnerPointer>) {
        match self.meta.field_map.get(field).expect(expect!().as_ref()) {
            &FieldMeta::Refer { .. } => {}
            &FieldMeta::Pointer { .. } => self.set_entity_pointer(field, opt.clone()),
            &FieldMeta::OneToOne { .. } => self.set_entity_one_one(field, opt.clone()),
            _ => unreachable!(),
        }
        // a.b = b;
        let a = self;
        let field_value = FieldValue::from(opt);
        a.field_map.insert(field.to_string(), field_value);

        // let field_value = FieldValue::from(b_rc);
        // self.field_map.insert(field, Some(field_value));
    }
    fn set_entity_pointer(&mut self, field: &str, opt: Option<EntityInnerPointer>) {
        let a = self;
        let field_meta = a.meta.field_map.get(field).expect(expect!().as_ref());
        let left = field_meta.get_refer_left();
        let right = field_meta.get_refer_right();

        if opt.is_none() {
            // a.b_id = null;
            a.field_map.insert(left, FieldValue::from(None));
        } else {
            // a.b_id = b.id;

        }
    }
    fn set_entity_one_one(&mut self, field: &str, opt: Option<EntityInnerPointer>) {
        let a = self;
        let field_meta = a.meta.field_map.get(field).expect(expect!().as_ref());
        let left = field_meta.get_refer_left();
        let right = field_meta.get_refer_right();

        let old_b = a.get_entity(field);
        if opt.is_some() {
            // b.a_id = a_id
            let b_rc = opt.unwrap();
            let b_id = b_rc.borrow().field_map.get(&right).map_or(Value::NULL, |v| v.as_value());
            a.field_map.insert(left, FieldValue::from(b_id));
        }
        if old_b.is_some() {
            // old_b.a_id = NULL;
            let old_b = old_b.unwrap();
            old_b.borrow_mut().field_map.insert(right.clone(), FieldValue::from(Value::NULL));
        }
    }
}

// 和session相关
// impl EntityInner {
// fn need_lazy_load(&self) -> bool {
//     // 以下都是没有查到的情况
//     if self.session.is_none() {
//         // 没有session，属于临时对象，不进行懒加载
//         return false;
//     }
//     // 以下为有session，即非临时对象的情况
//     let session = self.session.as_ref().unwrap();
//     if session.status() == SessionStatus::Closed {
//         // 游离态,抛异常
//         panic!("Can't Call Set/Get In Detached Status");
//     }
//     if session.status() == SessionStatus::Normal {
//         // 最常见的情况，正常的lazy load的情况
//         return true;
//     }
//     // 未考虑到的情况
//     unreachable!();
// }
// fn push_cache(&self, rc: EntityInnerPointer) {
//     // a和b有一个是临时态都不需要做这项操作
//     if self.session.is_none() || rc.borrow().session.is_none() {
//         return;
//     }
//     let session = self.session.as_ref().unwrap();
//     match session.status() {
//         SessionStatus::Closed => unreachable!(), // 异常情况
//         SessionStatus::Select => unreachable!(), // 目前的情况不应该出现
//         SessionStatus::Normal => session.push_cache(rc), // 在session内进行操作
//         SessionStatus::Insert => session.push_cache(rc), // 操作完成后的级联更新
//         SessionStatus::Update => session.push_cache(rc), // 操作完成后的级联更新
//         SessionStatus::Delete => session.push_cache(rc), // 操作完成后的级联更新
//     }
// }
// fn ensure_session_not_closed(&self) {
//     // 游离态
//     if self.session.is_some() &&
//        self.session.as_ref().unwrap().status() == SessionStatus::Closed {
//         panic!("Session Is Closed");
//     }
// }
// pub fn set_session(&mut self, session: Session) {
// self.session = Some(session);
// }
// pub fn set_session_recur(&mut self, session: Session) {
//     self.set_session(session.clone());
//     for (_, rc) in self.pointer_map.iter() {
//         if rc.is_some() {
//             rc.as_ref().unwrap().borrow_mut().set_session_recur(session.clone());
//         }
//     }
//     for (_, rc) in self.one_one_map.iter() {
//         if rc.is_some() {
//             rc.as_ref().unwrap().borrow_mut().set_session_recur(session.clone());
//         }
//     }
//     for (_, vec) in self.one_many_map.iter() {
//         for rc in vec.iter() {
//             rc.borrow_mut().set_session_recur(session.clone());
//         }
//     }
//     for (_, vec) in self.many_many_map.iter() {
//         for &(_, ref rc) in vec.iter() {
//             rc.borrow_mut().set_session_recur(session.clone());
//         }
//     }
// }
// pub fn clear_session(&mut self) {
//     self.session = None;
// }
// }

// 和数据库操作相关
impl EntityInner {
    pub fn get_values(&self) -> Vec<Value> {
        // 不包括id
        self.meta
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                self.field_map
                    .get(&field.get_field_name())
                    .map_or(Value::NULL,
                            |field_value: &FieldValue| field_value.as_value())
            })
            .collect::<Vec<_>>()
    }
    pub fn get_params(&self) -> Vec<(String, Value)> {
        // 不包括id
        self.meta
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                let value = self.field_map
                    .get(&field.get_field_name())
                    .map_or(Value::NULL,
                            |field_value: &FieldValue| field_value.as_value());
                (field.get_column_name(), value)
            })
            .collect::<Vec<_>>()
    }
    pub fn set_values(&mut self, row: &mut Row, alias: &str) {
        // 包括id
        for field_meta in self.meta.get_non_refer_fields() {
            let field = field_meta.get_field_name();
            let key = format!("{}${}", alias, field);
            row.get::<Value, &str>(&key).map(|value| {
                let field_value = FieldValue::from(value);
                self.field_map.insert(field, field_value);
                // self.set_value(&field, Some(value));
            });
        }
    }

    pub fn do_insert<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_insert();
        let params = self.get_params();
        // TODO if !auto push(id)
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| {
            let field_value = FieldValue::from(Value::from(res.last_insert_id()));
            self.field_map.insert("id".to_string(), field_value);
        })
    }
    pub fn do_update<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_update();
        let mut params = self.get_params();
        let id = self.get_id_value();
        params.insert(0, ("id".to_string(), id));
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|_| ())
    }
    pub fn do_delete<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_delete();
        let id = self.get_id_value();
        let params = vec![("id".to_string(), id)];
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|_| ())
    }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let entity = &self.meta.entity_name;
        write!(f, "")
    }
}

impl PartialEq for EntityInner {
    fn eq(&self, other: &EntityInner) -> bool {
        self.get_addr() == other.get_addr()
    }
}

impl Hash for EntityInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get_addr().hash(state);
    }
}

// impl<T> fmt::Debug for T
//     where T: Entity
// {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{:?}", self.inner().borrow())
//     }
// }

pub trait Entity {
    fn orm_meta() -> &'static OrmMeta;
    fn meta() -> &'static EntityMeta;
    fn default() -> Self;
    fn new() -> Self;
    fn from_inner(inner: EntityInnerPointer) -> Self;
    fn inner(&self) -> EntityInnerPointer;
    fn debug(&self) {
        let inner = self.inner();
        // println!("{:?}", inner.borrow());
    }

    fn do_inner<F, R>(&self, cb: F) -> R
        where F: FnOnce(&EntityInner) -> R
    {
        let rc = self.inner();
        let inner = rc.borrow();
        cb(&inner)
    }
    fn do_inner_mut<F, R>(&self, cb: F) -> R
        where F: FnOnce(&mut EntityInner) -> R
    {
        let rc = self.inner();
        let mut inner = rc.borrow_mut();
        cb(&mut inner)
    }

    fn inner_set_value<V>(&self, field: &str, value: V)
        where Value: From<V>
    {
        self.do_inner_mut(|mut inner| inner.set_value(field, value))
    }
    fn inner_get_value<V>(&self, field: &str) -> V
        where V: FromValue
    {
        self.do_inner(|inner| inner.get_value::<V>(field))
    }
    fn inner_set_value_null(&self, field: &str) {
        self.do_inner_mut(|mut inner| inner.set_value_null(field))
    }
    fn inner_is_value_null(&self, field: &str) -> bool {
        self.do_inner(|inner| inner.is_value_null(field))
    }


    fn inner_set_entity<E>(&self, field: &str, entity: &E)
        where E: Entity
    {
        self.do_inner_mut(|mut inner| inner.set_entity(field, Some(entity.inner())))
    }
    fn inner_get_entity<E>(&self, field: &str) -> E
        where E: Entity
    {
        self.do_inner(|inner| E::from_inner(inner.get_entity(field).unwrap()))
    }
    fn inner_set_entity_null(&self, field: &str) {
        self.do_inner_mut(|mut inner| inner.set_entity(field, None))
    }
    fn inner_is_entity_null(&self, field: &str) -> bool {
        self.do_inner(|inner| inner.get_entity(field).is_none())
    }
}
