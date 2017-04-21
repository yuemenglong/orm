#[macro_use]
use macros;

use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;

use mysql::Value;
use mysql::Error;
use mysql::error::MySqlError;
use mysql::value;
use mysql::prelude::FromValue;
use mysql::QueryResult;
use mysql::Row;
use mysql::prelude::GenericConnection;

use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;
use meta::Cascade;
use session::Session;
use session::SessionStatus;
use select::Select;

use cond::Cond;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

pub struct EntityInner {
    pub orm_meta: &'static OrmMeta,
    pub meta: &'static EntityMeta,
    pub field_map: HashMap<String, Value>,
    pub pointer_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_one_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_many_map: HashMap<String, Vec<EntityInnerPointer>>,
    pub many_many_map: HashMap<String, Vec<(Option<EntityInnerPointer>, EntityInnerPointer)>>,

    pub cascade: Option<Cascade>,
    pub session: Option<Session>, // pub cache: Vec<(String, EntityInnerPointer)>,
}

// 和字段编辑相关
impl EntityInner {
    pub fn new(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> EntityInner {
        EntityInner {
            orm_meta: orm_meta,
            meta: meta,
            field_map: HashMap::new(),
            pointer_map: HashMap::new(),
            one_one_map: HashMap::new(),
            one_many_map: HashMap::new(),
            many_many_map: HashMap::new(),
            cascade: None,
            session: None, // cache: Vec::new(),
        }
    }
    pub fn default(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> EntityInner {
        // 用默认值?
        // 一旦有则为正常值，不能为NULL，因为外层无法设为NULL
        let field_map: HashMap<String, Value> = HashMap::new();
        // 避免lazy load, 用默认None
        let pointer_map: HashMap<String, Option<EntityInnerPointer>> = meta.get_pointer_fields()
            .into_iter()
            .map(|meta| (meta.get_field_name(), None))
            .collect();
        let one_one_map: HashMap<String, Option<EntityInnerPointer>> = meta.get_one_one_fields()
            .into_iter()
            .map(|meta| (meta.get_field_name(), None))
            .collect();
        let one_many_map: HashMap<String, Vec<EntityInnerPointer>> = meta.get_one_many_fields()
            .into_iter()
            .map(|meta| (meta.get_field_name(), Vec::new()))
            .collect();
        let many_many_map: HashMap<String, Vec<(Option<EntityInnerPointer>, EntityInnerPointer)>> =
            meta.get_many_many_fields()
                .into_iter()
                .map(|meta| (meta.get_field_name(), Vec::new()))
                .collect();
        EntityInner {
            orm_meta: orm_meta,
            meta: meta,
            field_map: field_map,
            pointer_map: pointer_map,
            one_one_map: one_one_map,
            one_many_map: one_many_map,
            many_many_map: many_many_map,
            cascade: None,
            session: None, // cache: Vec::new(),
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
        self.field_map.get("id").map_or(Value::NULL, |id| id.clone())
    }
    pub fn get_id_u64(&self) -> Option<u64> {
        self.get_u64("id")
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.field_map.get(key).map_or(None, |id| match id {
            &Value::NULL => None,
            id @ _ => Some(value::from_value::<u64>(id.clone())),
        })
    }

    pub fn set_value<V>(&mut self, key: &str, value: Option<V>)
        where Value: From<V>
    {
        match value {
            None => self.field_map.remove(key),
            Some(v) => self.field_map.insert(key.to_string(), Value::from(v)),
        };
    }
    pub fn get_value<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.field_map.get(key).map(|value| value::from_value(value.clone()))
    }

    pub fn set_pointer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        self.ensure_session_not_closed();
        let a = self;
        // a.b_id = b.id;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let a_b_id_field = a_b_meta.get_pointer_id();
        let b_id = match value {
            None => Value::NULL,
            Some(ref rc) => rc.borrow().get_id_value(),
        };
        a.field_map.insert(a_b_id_field, b_id);
        // a.b = b;
        let b = value;
        let a_b_field = a_b_meta.get_field_name();
        a.pointer_map.insert(a_b_field, b);
    }
    pub fn get_pointer(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let mut a = self;
        {
            // 查到了就直接返回了
            let a_b = a.pointer_map.get(key);
            if a_b.is_some() {
                return a_b.unwrap().clone();
            }
        }
        if !a.need_lazy_load() {
            return None;
        }
        // 懒加载
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_entity = a_b_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
        let b_id_field = a_b_meta.get_pointer_id();
        let b_id = a.field_map.get(&b_id_field).unwrap();
        // let mut cond = Cond::from_meta(b_meta, a.orm_meta);
        // cond.eq("id", b_id.clone());
        let session = a.session.as_ref().unwrap();
        let res = session.get_inner(b_meta, a.orm_meta, &Cond::by_id(b_id.clone()));
        if res.is_err() {
            panic!("Get Pointer Fail");
        }
        let b_rc_opt = res.unwrap();
        a.pointer_map.insert(key.to_string(), b_rc_opt.clone());
        b_rc_opt
    }

    pub fn set_one_one(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        self.ensure_session_not_closed();
        let mut a = self;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_a_id_field = a_b_meta.get_one_one_id();
        let old_b = a.get_one_one(key);
        // old_b.a_id = NULL;
        if old_b.is_some() && old_b != value {
            // 不同对象才需要cache
            let old_b = old_b.unwrap();
            old_b.borrow_mut().field_map.insert(b_a_id_field.to_string(), Value::NULL);
            // a.cache.push((key.to_string(), old_b));
            a.push_cache(old_b.clone());
        }
        // b.a_id = a.id;
        let a_id = a.get_id_value();
        if value.is_some() {
            let b = value.as_ref().unwrap().clone();
            b.borrow_mut().field_map.insert(b_a_id_field, a_id.clone());
        }
        // a.b = b;
        a.one_one_map.insert(key.to_string(), value);
    }
    pub fn get_one_one(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let mut a = self;
        {
            // 查到了就直接返回了
            let a_b = a.one_one_map.get(key);
            if a_b.is_some() {
                return a_b.unwrap().clone();
            }
        }
        if !a.need_lazy_load() {
            return None;
        }
        // 懒加载
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_entity = a_b_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
        let a_id = a.get_id_value();
        let b_a_id_field = a_b_meta.get_one_one_id();
        let session = a.session.as_ref().unwrap();
        // let res = session.get_inner(&cond);
        let res = session.get_inner(b_meta, a.orm_meta, &Cond::by_eq(&b_a_id_field, a_id));
        if res.is_err() {
            panic!("Get One One Fail");
        }
        let b_rc_opt = res.unwrap();
        a.one_one_map.insert(key.to_string(), b_rc_opt.clone());
        b_rc_opt
        // unimplemented!();
    }

    pub fn set_one_many(&mut self, key: &str, value: Vec<EntityInnerPointer>) {
        self.ensure_session_not_closed();
        let mut a = self;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_a_id_field = a_b_meta.get_one_many_id();
        let old_b_vec = a.get_one_many(key);
        let new_b_set = value.iter().map(|rc| rc.borrow().get_addr()).collect::<HashSet<_>>();
        // old_b.a_id = NULL;
        for b in old_b_vec {
            if new_b_set.contains(&b.borrow().get_addr()) {
                // 还在原集合中的不需要断开关系
                continue;
            }
            b.borrow_mut().field_map.insert(b_a_id_field.to_string(), Value::NULL);
            // a.cache.push((key.to_string(), b));
            a.push_cache(b.clone());
        }
        // b.a_id = a.id;
        let a_id = a.get_id_value();
        for b in value.iter() {
            b.borrow_mut().field_map.insert(b_a_id_field.to_string(), a_id.clone());
        }
        // a.b = b;
        a.one_many_map.insert(key.to_string(), value);
    }
    pub fn get_one_many(&mut self, key: &str) -> Vec<EntityInnerPointer> {
        let a = self;
        {
            let a_b = a.one_many_map.get(key);
            if a_b.is_some() {
                return a_b.unwrap().clone();
            }
        }
        if !a.need_lazy_load() {
            return Vec::new();
        }
        // 懒加载
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_entity = a_b_meta.get_refer_entity();
        let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
        let a_id = a.get_id_value();
        let b_a_id_field = a_b_meta.get_one_many_id();
        // let mut cond = Cond::from_meta(b_meta, a.orm_meta);
        // cond.eq(&b_a_id_field, a_id);
        let session = a.session.as_ref().unwrap();
        let res = session.select_inner(b_meta, a.orm_meta, &Cond::by_eq(&b_a_id_field, a_id));
        if res.is_err() {
            panic!("Get One Many Fail");
        }
        let b_rc_vec = res.unwrap();
        a.one_many_map.insert(key.to_string(), b_rc_vec.clone());
        b_rc_vec
    }
    pub fn push_one_many(&mut self, key: &str, value: EntityInnerPointer) {
        let mut a = self;
        let b_rc = value;
        // a.bs.push(b);
        a.one_many_map.entry(key.to_string()).or_insert(Vec::new());
        a.one_many_map.get_mut(key).unwrap().push(b_rc);
    }

    pub fn set_many_many(&mut self, key: &str, value: Vec<EntityInnerPointer>) {
        self.ensure_session_not_closed();
        let a = self;
        // 确保中间表信息存在
        a.get_many_many(key);

        // mid.a_id = a.id
        // mid.b_id = b.id
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let a_id = a.get_id_value();
        let middle_entity = a_b_meta.get_many_many_middle_entity();
        let middle_meta = a.orm_meta.entity_map.get(&middle_entity).unwrap();
        let a_id_field = a_b_meta.get_many_many_id();
        let b_id_field = a_b_meta.get_many_many_refer_id();
        let orm_meta = a.orm_meta;

        let create_middle_inner = |b_id: Value| {
            let mut middle_inner = EntityInner::default(middle_meta, orm_meta);
            middle_inner.field_map.insert(a_id_field.to_string(), a_id.clone());
            middle_inner.field_map.insert(b_id_field.to_string(), b_id.clone());
            middle_inner.cascade_insert();
            Rc::new(RefCell::new(middle_inner))
        };

        // 只要存在必然有效
        let mut old_b_mid_map = a.many_many_map.get(key).map_or(HashMap::new(), |vec| {
            vec.iter()
                .filter_map(|&(ref m_opt, _)| {
                    m_opt.clone().map(|m_rc| {
                        let id = m_rc.borrow().get_u64(&b_id_field).unwrap();
                        (id, m_rc.clone())
                    })
                })
                .collect::<HashMap<u64, EntityInnerPointer>>()
        });
        let new_b_pair_vec = value.iter()
            .map(|b_rc| {
                if a_id == Value::NULL {
                    // 一方没有id即为None
                    return (None, b_rc.clone());
                }
                let b_id_value = b_rc.borrow().get_id_value();
                let b_id_opt = b_rc.borrow().get_id_u64();
                match b_id_opt {
                    // 一方没有id即为None
                    None => (None, b_rc.clone()),
                    Some(b_id) => {
                        match old_b_mid_map.remove(&b_id) {
                            // 新的关系
                            None => (Some(create_middle_inner(b_id_value)), b_rc.clone()),
                            // 老的关系
                            Some(m_rc) => (Some(m_rc.clone()), b_rc.clone()),
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        // 剩下的老关系都要删掉
        for (m_id, m_rc) in old_b_mid_map.iter() {
            m_rc.borrow_mut().cascade_delete();
            a.push_cache(m_rc.clone());
        }
        a.many_many_map.insert(key.to_string(), new_b_pair_vec);
    }
    pub fn get_many_many(&mut self, key: &str) -> Vec<EntityInnerPointer> {
        let a = self;
        {
            let a_b = a.many_many_map.get(key);
            if a_b.is_some() {
                return a_b.unwrap().iter().map(|&(_, ref b_rc)| b_rc.clone()).collect::<_>();
            }
        }
        if !a.need_lazy_load() {
            debug!("not need_lazy_load");
            return Vec::new();
        }
        // 下面为懒加载
        let mut select = Select::from_meta(a.meta, a.orm_meta);
        select.join(key);
        select.wher(&Cond::by_id(a.get_id_u64().unwrap()));
        let vec = (|| {
            let session = a.session.as_ref().unwrap();
            session.query_inner(&select).unwrap()
        })();
        if vec.len() == 0 {
            a.set_many_many(key, Vec::new());
            return Vec::new();
        }
        let ref a2 = vec[0];
        // 这里比较特殊，没有通过set接口，因为要传递中间表，懒加载的情况下可以这么处理
        let pair_vec = a2.borrow_mut().many_many_map.remove(key).unwrap();
        a.many_many_map.insert(key.to_string(), pair_vec);
        a.get_many_many(key)
        // 懒加载
        // let a_b_meta = a.meta.field_map.get(key).unwrap();
        // let b_entity = a_b_meta.get_refer_entity();
        // let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
        // let a_id = a.get_id_value();
        // let b_a_id_field = a_b_meta.get_many_many_id();
        // let mut cond = Cond::from_meta(b_meta, a.orm_meta);
        // cond.eq(&b_a_id_field, a_id);
        // let session = a.session.as_ref().unwrap();
        // let res = session.select_inner(&cond);
        // if res.is_err() {
        //     panic!("Get One Many Fail");
        // }
        // let b_rc_vec = res.unwrap();
        // a.many_many_map.insert(key.to_string(), b_rc_vec.clone());
        // b_rc_vec


        // let mut a = &self;
        // let a_b_vec = a.many_many_map.get(key);
        // if a_b_vec.is_none() {
        //     // lazy load
        //     // let a_b_meta = self.meta.field_map.get(key).unwrap();
        //     unimplemented!();
        // }
        // a.many_many_map
        //     .get(key)
        //     .unwrap()
        //     .iter()
        //     .map(|&(_, ref b_rc)| b_rc.clone())
        //     .collect::<Vec<_>>()
    }
    pub fn push_many_many(&mut self, key: &str, value: (EntityInnerPointer, EntityInnerPointer)) {
        let mut a = self;
        let (m_rc, b_rc) = value;
        // a.bs.push(b)
        a.many_many_map.entry(key.to_string()).or_insert(Vec::new());
        a.many_many_map.get_mut(key).unwrap().push((Some(m_rc), b_rc));
    }
}

// 和session相关
impl EntityInner {
    fn need_lazy_load(&self) -> bool {
        // 以下都是没有查到的情况
        if self.session.is_none() {
            // 没有session，属于临时对象，不进行懒加载
            return false;
        }
        // 以下为有session，即非临时对象的情况
        let session = self.session.as_ref().unwrap();
        if session.status() == SessionStatus::Closed {
            // 游离态,抛异常
            panic!("Can't Call Set/Get In Detached Status");
        }
        if session.status() == SessionStatus::Select {
            // 在执行查询的过程中，说明正在组装对象，不进行懒加载
            return false;
        }
        if session.status() == SessionStatus::Normal {
            // 最常见的情况，正常的lazy load的情况
            return true;
        }
        // 未考虑到的情况
        unreachable!();
    }
    fn push_cache(&self, rc: EntityInnerPointer) {
        // a和b有一个是临时态都不需要做这项操作
        if self.session.is_none() || rc.borrow().session.is_none() {
            return;
        }
        let session = self.session.as_ref().unwrap();
        match session.status() {
            SessionStatus::Closed => unreachable!(), // 异常情况
            SessionStatus::Select => unreachable!(), // 目前的情况不应该出现
            SessionStatus::Normal => session.push_cache(rc), // 在session内进行操作
            SessionStatus::Insert => session.push_cache(rc), // 操作完成后的级联更新
            SessionStatus::Update => session.push_cache(rc), // 操作完成后的级联更新
            SessionStatus::Delete => session.push_cache(rc), // 操作完成后的级联更新
        }
    }
    fn ensure_session_not_closed(&self) {
        // 游离态
        if self.session.is_some() &&
           self.session.as_ref().unwrap().status() == SessionStatus::Closed {
            panic!("Session Is Closed");
        }
    }
}

// 和级联相关
impl EntityInner {
    pub fn cascade_field_insert(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_rt_cascade(Some(Cascade::Insert));
    }
    pub fn cascade_field_update(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_rt_cascade(Some(Cascade::Update));
    }
    pub fn cascade_field_delete(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_rt_cascade(Some(Cascade::Delete));
    }
    pub fn cascade_field_null(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_rt_cascade(Some(Cascade::NULL));
    }
    pub fn cascade_insert(&mut self) {
        self.cascade = Some(Cascade::Insert);
    }
    pub fn cascade_update(&mut self) {
        self.cascade = Some(Cascade::Update);
    }
    pub fn cascade_delete(&mut self) {
        self.cascade = Some(Cascade::Delete);
    }
    pub fn cascade_null(&mut self) {
        self.cascade = Some(Cascade::NULL);
    }
    pub fn cascade_reset(&mut self) {
        self.cascade = None;
        for (_, b_rc) in &self.pointer_map {
            let b_rc = b_rc.clone();
            if b_rc.is_some() {
                let b_rc = b_rc.unwrap();
                b_rc.borrow_mut().cascade_reset();
            }
        }
        for (_, b_rc) in &self.one_one_map {
            let b_rc = b_rc.clone();
            if b_rc.is_some() {
                let b_rc = b_rc.unwrap();
                b_rc.borrow_mut().cascade_reset();
            }
        }
        for (_, vec) in &self.one_many_map {
            for b_rc in vec {
                b_rc.borrow_mut().cascade_reset();
            }
        }
        for (_, vec) in &self.many_many_map {
            for &(ref m_opt, ref b_rc) in vec {
                m_opt.iter().map(|m_rc| m_rc.borrow_mut().cascade_reset());
                b_rc.borrow_mut().cascade_reset();
            }
        }
        // cache cascade
        // for &(_, ref b_rc) in &self.cache {
        //     b_rc.borrow_mut().cascade_reset();
        // }
        // 字段cascade
        for a_b_meta in &self.meta.get_refer_fields() {
            a_b_meta.set_refer_rt_cascade(None);
        }
    }
    pub fn set_session(&mut self, session: Session) {
        self.session = Some(session);
    }
    pub fn clear_session(&mut self) {
        self.session = None;
    }
}

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
                    .map(|value| value.clone())
                    .or(Some(Value::NULL))
                    .unwrap()
            })
            .collect::<Vec<_>>()
    }
    pub fn get_params(&self) -> Vec<(String, Value)> {
        // 不包括id
        self.meta
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                (field.get_column_name(),
                 self.field_map
                     .get(&field.get_field_name())
                     .map(|value| value.clone())
                     .or(Some(Value::NULL))
                     .unwrap())
            })
            .collect::<Vec<_>>()
    }
    pub fn set_values(&mut self, row: &mut Row, alias: &str) {
        // 包括id
        for field_meta in self.meta.get_non_refer_fields() {
            let field = field_meta.get_field_name();
            let key = format!("{}${}", alias, field);
            row.get::<Value, &str>(&key).map(|value| {
                self.set_value(&field, Some(value));
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
            self.field_map.insert("id".to_string(), Value::from(res.last_insert_id()));
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
        conn.prep_exec(sql, params).map(|res| ())
    }
    pub fn do_delete<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_delete();
        let id = self.get_id_value();
        let params = vec![("id".to_string(), id)];
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| ())
    }
}

// 和debug相关
impl EntityInner {
    fn format(&self) -> String {
        let inner = vec![self.fmt_value(),
                         self.fmt_pointer(),
                         self.fmt_one_one(),
                         self.fmt_one_many(),
                         self.fmt_many_many()]
            .into_iter()
            .filter(|s| s.len() > 0)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{{}}}", inner)
    }
    fn fmt_rc(rc: &EntityInnerPointer) -> String {
        let rc = rc.clone();
        let inner = rc.borrow();
        inner.format()
    }
    fn fmt_value(&self) -> String {
        self.meta
            .get_non_refer_fields()
            .into_iter()
            .flat_map(|meta| {
                let field = meta.get_field_name();
                let value_opt = self.field_map.get(&field);
                value_opt.map(|value| format!("{}: {}", field, meta.format(value.clone())))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_pointer(&self) -> String {
        self.meta
            .get_pointer_fields()
            .into_iter()
            .flat_map(|meta| {
                let field = meta.get_field_name();
                let value_opt = self.pointer_map.get(&field);
                value_opt.map(|value| match value {
                    &None => format!("{}: null", field),
                    &Some(ref value) => format!("{}: {}", field, Self::fmt_rc(value)),
                })
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_one_one(&self) -> String {
        self.meta
            .get_one_one_fields()
            .into_iter()
            .flat_map(|meta| {
                let field = meta.get_field_name();
                let value_opt = self.one_one_map.get(&field);
                value_opt.map(|value| match value {
                    &None => format!("{}: null", field),
                    &Some(ref value) => format!("{}: {}", field, Self::fmt_rc(value)),
                })
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_one_many(&self) -> String {
        self.meta
            .get_one_many_fields()
            .into_iter()
            .flat_map(|meta| {
                let field = meta.get_field_name();
                let vec_opt = self.one_many_map.get(&field);
                vec_opt.map(|vec| {
                    let vec_string = vec.iter().map(Self::fmt_rc).collect::<Vec<_>>().join(", ");
                    format!("{}: [{}]", field, vec_string)
                })
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_many_many(&self) -> String {
        self.meta
            .get_many_many_fields()
            .into_iter()
            .flat_map(|meta| {
                let field = meta.get_field_name();
                let vec_opt = self.many_many_map.get(&field);
                vec_opt.map(|pair_vec| {
                    let vec_string = pair_vec.iter()
                        .map(|&(_, ref rc)| rc)
                        .map(Self::fmt_rc)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}: [{}]", field, vec_string)
                })
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let entity = &self.meta.entity_name;
        write!(f, "{}: {}", entity, self.format())
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

pub trait Entity {
    fn orm_meta() -> &'static OrmMeta;
    fn meta() -> &'static EntityMeta;
    fn default() -> Self;
    fn new() -> Self;
    fn from_inner(inner: EntityInnerPointer) -> Self;
    fn inner(&self) -> EntityInnerPointer;
    fn debug(&self) {
        let inner = self.inner();
        println!("{:?}", inner.borrow());
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

    fn inner_get_value<V>(&self, key: &str) -> V
        where V: FromValue
    {
        let opt = self.do_inner(|inner| inner.get_value::<V>(key));
        opt.expect(&format!("[{}] Get [{}] Of None", Self::meta().entity_name, key))
    }
    fn inner_set_value<V>(&self, key: &str, value: V)
        where Value: From<V>
    {
        self.do_inner_mut(|inner| inner.set_value::<V>(key, Some(value)));
    }
    fn inner_has_value<V>(&self, key: &str) -> bool
        where V: FromValue
    {
        self.do_inner(|inner| inner.get_value::<V>(key)).is_some()
    }
    fn inner_clear_value<V>(&self, key: &str)
        where Value: From<V>
    {
        self.do_inner_mut(|inner| inner.set_value::<V>(key, None));
    }

    fn inner_get_pointer<E>(&self, key: &str) -> E
        where E: Entity
    {
        let opt = self.do_inner_mut(|inner| inner.get_pointer(key)).map(|rc| E::from_inner(rc));
        opt.expect(&format!("[{}] Get [{}] Of None", Self::meta().entity_name, key))
    }
    fn inner_set_pointer<E>(&self, key: &str, value: &E)
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.set_pointer(key, Some(value.inner())));
    }
    fn inner_has_pointer(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.get_pointer(key)).is_some()
    }
    fn inner_clear_pointer(&self, key: &str) {
        self.do_inner_mut(|inner| inner.set_pointer(key, None))
    }

    fn inner_get_one_one<E>(&self, key: &str) -> E
        where E: Entity
    {
        let opt = self.do_inner_mut(|inner| inner.get_one_one(key)).map(|rc| E::from_inner(rc));
        opt.expect(&format!("[{}] Get [{}] Of None", Self::meta().entity_name, key))
    }
    fn inner_set_one_one<E>(&self, key: &str, value: &E)
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.set_one_one(key, Some(value.inner())));
    }
    fn inner_has_one_one(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.get_one_one(key)).is_some()
    }
    fn inner_clear_one_one(&self, key: &str) {
        self.do_inner_mut(|inner| inner.set_one_one(key, None));
    }

    fn inner_get_one_many<E>(&self, key: &str) -> Vec<E>
        where E: Entity
    {
        let vec = self.do_inner_mut(|inner| inner.get_one_many(key));
        vec.into_iter().map(E::from_inner).collect::<Vec<_>>()
    }
    fn inner_set_one_many<E>(&self, key: &str, value: Vec<E>)
        where E: Entity
    {
        let vec = value.iter().map(E::inner).collect::<Vec<_>>();
        self.do_inner_mut(|inner| inner.set_one_many(key, vec));
    }
    fn inner_has_one_many(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.get_one_many(key)).len() > 0
    }
    fn inner_clear_one_many(&self, key: &str) {
        self.do_inner_mut(|inner| inner.set_one_many(key, Vec::new()));
    }

    fn inner_get_many_many<E>(&self, key: &str) -> Vec<E>
        where E: Entity
    {
        let vec = self.do_inner_mut(|inner| inner.get_many_many(key));
        vec.into_iter().map(E::from_inner).collect::<Vec<_>>()
    }
    fn inner_set_many_many<E>(&self, key: &str, value: Vec<E>)
        where E: Entity
    {
        let vec = value.iter().map(E::inner).collect::<Vec<_>>();
        self.do_inner_mut(|inner| inner.set_many_many(key, vec));
    }
    fn inner_has_many_many(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.get_many_many(key)).len() > 0
    }
    fn inner_clear_many_many(&self, key: &str) {
        self.do_inner_mut(|inner| inner.set_many_many(key, Vec::new()));
    }

    fn set_id(&self, id: u64) {
        self.inner_set_value("id", id);
    }
    fn get_id(&self) -> u64 {
        self.inner_get_value("id")
    }
    fn has_id(&self) -> bool {
        self.inner_has_value::<u64>("id")
    }
    fn clear_id(&self) {
        self.inner_clear_value::<u64>("id")
    }

    fn cascade_insert(&self) {
        self.do_inner_mut(|inner| inner.cascade_insert());
    }
    fn cascade_update(&self) {
        self.do_inner_mut(|inner| inner.cascade_update());
    }
    fn cascade_delete(&self) {
        self.do_inner_mut(|inner| inner.cascade_delete());
    }
    fn cascade_null(&self) {
        self.do_inner_mut(|inner| inner.cascade_null());
    }
    fn cascade_reset(&self) {
        self.do_inner_mut(|inner| inner.cascade_reset());
    }
    fn inner_cascade_field_insert(&self, field: &str) {
        self.do_inner_mut(|inner| inner.cascade_field_insert(field));
    }
    fn inner_cascade_field_update(&self, field: &str) {
        self.do_inner_mut(|inner| inner.cascade_field_update(field));
    }
    fn inner_cascade_field_delete(&self, field: &str) {
        self.do_inner_mut(|inner| inner.cascade_field_delete(field));
    }
    fn inner_cascade_field_null(&self, field: &str) {
        self.do_inner_mut(|inner| inner.cascade_field_null(field));
    }

    // fn get_refer<E:Entity>(&self, field: &str) -> Option<&E>;
    // fn set_refer(&mut self, field: &str, e: Option<Entity>);


    // fn get_name() -> String;
    // // fn get_field_meta() -> Vec<FieldMeta>;
    // fn get_params(&self) -> Vec<(String, Value)>;
    // fn from_row(row: Row) -> Self;
    // fn from_row_ex(row: Row, nameMap: &HashMap<String, String>) -> Self;

    // fn sql_create_table() -> String;
    // fn sql_drop_table() -> String;

    // fn get_field_list() -> String;
    // fn get_prepare() -> String;
    // fn get_params_id(&self) -> Vec<(String, Value)>;
    //  {
    //     vec![("id".to_string(), Value::from(self.get_id()))]
    // }
}
