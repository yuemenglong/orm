use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;
use std::fmt;

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

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

#[derive(Clone)]
pub struct EntityInner {
    pub orm_meta: &'static OrmMeta,
    pub meta: &'static EntityMeta,
    pub field_map: HashMap<String, Value>,
    pub pointer_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_one_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_many_map: HashMap<String, Vec<EntityInnerPointer>>,
    pub many_many_map: HashMap<String, Vec<(EntityInnerPointer, EntityInnerPointer)>>,

    pub cascade: Option<Cascade>,
    pub cache: Vec<(String, EntityInnerPointer)>,
}

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
            cache: Vec::new(),
        }
    }
    pub fn default(meta: &'static EntityMeta, orm_meta: &'static OrmMeta) -> EntityInner {
        // 用默认值
        // let field_map: HashMap<String, Value> = meta.get_non_refer_fields()
        //     .into_iter()
        //     .map(|meta| (meta.get_field_name(), Value::NULL))
        //     .collect();
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
        let many_many_map: HashMap<String, Vec<(EntityInnerPointer, EntityInnerPointer)>> =
            meta.get_many_many_fields()
                .into_iter()
                .map(|meta| (meta.get_field_name(), Vec::new()))
                .collect();
        EntityInner {
            orm_meta: orm_meta,
            meta: meta,
            field_map: HashMap::new(),
            pointer_map: pointer_map,
            one_one_map: one_one_map,
            one_many_map: one_many_map,
            many_many_map: many_many_map,
            cascade: None,
            cache: Vec::new(),
        }
    }

    pub fn get_id_value(&self) -> Value {
        self.field_map.get("id").map_or(Value::NULL, |id| id.clone())
    }

    pub fn set<V>(&mut self, key: &str, value: Option<V>)
        where Value: From<V>
    {
        match value {
            None => self.field_map.remove(key),
            Some(v) => self.field_map.insert(key.to_string(), Value::from(v)),
        };
    }
    pub fn get<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.field_map.get(key).map(|value| value::from_value(value.clone()))
    }

    pub fn set_pointer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        let a = self;
        // a.b_id = b.id;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let a_b_id_field = a_b_meta.get_pointer_id();
        let b_id = match value {
            None => Value::NULL,
            Some(ref rc) => rc.borrow().field_map.get("id").unwrap().clone(),
        };
        a.field_map.insert(a_b_id_field, b_id);
        // a.b = b;
        let b = value;
        let a_b_field = a_b_meta.get_field_name();
        a.pointer_map.insert(a_b_field, b);
    }
    pub fn get_pointer(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let a = &self;
        // return a.b
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let a_b = a.pointer_map.get(key);
        if a_b.is_none() {
            let a_b_id_field = a_b_meta.get_pointer_id();
            // lazy load
            unimplemented!();
        }
        a.pointer_map.get(key).unwrap().clone()
    }

    pub fn set_one_one(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        let mut a = self;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_a_id_field = a_b_meta.get_one_one_id();
        let old_b = a.get_one_one(key);
        // old_b.a_id = NULL;
        if old_b.is_some() {
            let old_b = old_b.unwrap();
            old_b.borrow_mut().field_map.insert(b_a_id_field.to_string(), Value::NULL);
            a.cache.push((key.to_string(), old_b));
        }
        // b.a_id = a.id;
        let a_id = a.field_map.get("id").unwrap();
        if value.is_some() {
            let b = value.as_ref().unwrap().clone();
            b.borrow_mut().field_map.insert(b_a_id_field, a_id.clone());
        }
        // a.b = b;
        a.one_one_map.insert(key.to_string(), value);
    }
    pub fn get_one_one(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let mut a = &self;
        let a_b = a.one_one_map.get(key);
        if a_b.is_none() {
            // lazy load
            // let a_b_meta = self.meta.field_map.get(key).unwrap();
            unimplemented!();
        }
        a.one_one_map.get(key).unwrap().clone()
    }

    pub fn set_one_many(&mut self, key: &str, value: Vec<EntityInnerPointer>) {
        let mut a = self;
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let b_a_id_field = a_b_meta.get_one_many_id();
        let old_b_vec = a.get_one_many(key);
        // old_b.a_id = NULL;
        for b in old_b_vec {
            b.borrow_mut().field_map.insert(b_a_id_field.to_string(), Value::NULL);
            a.cache.push((key.to_string(), b));
        }
        // b.a_id = a.id;
        let a_id = a.field_map.get("id").unwrap();
        for b in value.iter() {
            b.borrow_mut().field_map.insert(b_a_id_field.to_string(), a_id.clone());
        }
        // a.b = b;
        a.one_many_map.insert(key.to_string(), value);
    }
    pub fn get_one_many(&mut self, key: &str) -> Vec<EntityInnerPointer> {
        let mut a = &self;
        let a_b_field = key;
        let a_b = a.one_many_map.get(a_b_field);
        if a_b.is_none() {
            // lazy load
            unimplemented!();
        }
        a.one_many_map.get(a_b_field).unwrap().clone()
    }

    pub fn set_many_many(&mut self, key: &str, value: Vec<EntityInnerPointer>) {
        let a = self;
        // 确保中间表信息存在
        a.get_many_many(key);

        // mid.a_id = a.id
        // mid.b_id = b.id
        let a_b_meta = a.meta.field_map.get(key).unwrap();
        let a_id = a.field_map.get("id").map_or(Value::NULL, |id| id.clone());
        let middle_name = a_b_meta.get_many_many_middle_name();
        let middle_meta = a.orm_meta.entity_map.get(&middle_name).unwrap();
        let a_id_field = a_b_meta.get_many_many_id();
        let b_id_field = a_b_meta.get_many_many_refer_id();
        let orm_meta = a.orm_meta;
        let middle_inner_fn = |b_id: Value| {
            let mut middle_inner = EntityInner::default(middle_meta, orm_meta);
            middle_inner.field_map.insert(a_id_field.to_string(), a_id.clone());
            middle_inner.field_map.insert(b_id_field.to_string(), b_id.clone());
            Rc::new(RefCell::new(middle_inner))
        };

        // 新id集合
        let new_b_id_set = value.iter()
            .filter_map(|b_rc| {
                let b = b_rc.borrow();
                b.field_map.get("id").map(|b_id| value::from_value::<u64>(b_id.clone()))
            })
            .collect::<HashSet<u64>>();

        // 临时替换，处理老数据
        let old_b_vec = a.many_many_map.insert(key.to_string(), Vec::new()).unwrap();
        let old_b_id_map = old_b_vec.iter()
            .filter_map(|&(ref m_rc, ref b_rc)| {
                let b = b_rc.borrow();
                b.field_map.get("id").map(|b_id| {
                    let b_id = value::from_value::<u64>(b_id.clone());
                    (b_id, m_rc.clone())
                })
            })
            .collect::<HashMap<u64, EntityInnerPointer>>();
        for (middle, b_rc) in old_b_vec.into_iter() {
            // 不在新value里的删除，在新value里的留下
            let b = b_rc.borrow();
            let b_id = b.field_map.get("id").map_or(Value::NULL, |id| id.clone());
            if b_id == Value::NULL {
                // b没有id，不用管中间表(肯定没有存在数据库中)
                continue;
            }
            let b_id = value::from_value::<u64>(b_id);
            if new_b_id_set.contains(&b_id) {
                // 和新集合中，保留
                continue;
            }
            // 不在新集合中，删除
            middle.borrow_mut().cascade_delete();
            a.cache.push((key.to_string(), middle));
            a.cache.push((key.to_string(), b_rc.clone()));
        }
        // 处理新数据
        let new_b_vec = value.into_iter()
            .map(|b_rc| {
                let b = b_rc.borrow();
                // 1. 没有id或id为NULL，生成的中间表也没id
                // 2. 有id，但是不在老中间表中，按照新b_id
                // 3. 有id，在老中间表里，直接用
                let m_rc = match b.field_map.get("id").map_or(Value::NULL, |id| id.clone()) {
                    Value::NULL => middle_inner_fn(Value::NULL),
                    b_id_value @ _ => {
                        let b_id = value::from_value::<u64>(b_id_value.clone());
                        old_b_id_map.get(&b_id)
                            .map_or(middle_inner_fn(b_id_value), |m_rc| m_rc.clone())
                    }
                };
                (m_rc.clone(), b_rc.clone())
            })
            .collect::<Vec<_>>();
        a.many_many_map.insert(key.to_string(), new_b_vec);
    }
    pub fn get_many_many(&mut self, key: &str) -> Vec<EntityInnerPointer> {
        let mut a = &self;
        let a_b_vec = a.many_many_map.get(key);
        if a_b_vec.is_none() {
            // lazy load
            // let a_b_meta = self.meta.field_map.get(key).unwrap();
            unimplemented!();
        }
        a.many_many_map
            .get(key)
            .unwrap()
            .iter()
            .map(|&(_, ref b_rc)| b_rc.clone())
            .collect::<Vec<_>>()
    }

    pub fn cascade_field_insert(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_cascade(Some(Cascade::Insert));
    }
    pub fn cascade_field_update(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_cascade(Some(Cascade::Update));
    }
    pub fn cascade_field_delete(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_cascade(Some(Cascade::Delete));
    }
    pub fn cascade_field_null(&mut self, field: &str) {
        let a_b_meta = self.meta.field_map.get(field).unwrap();
        a_b_meta.set_refer_cascade(Some(Cascade::NULL));
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
        // cache cascade
        for &(_, ref b_rc) in &self.cache {
            b_rc.borrow_mut().cascade_reset();
        }
        // 字段cascade
        for a_b_meta in &self.meta.get_refer_fields() {
            a_b_meta.set_refer_cascade(None);
        }
    }

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
    pub fn set_values(&mut self, result: &QueryResult, row: &mut Row, prefix: &str) {
        // 包括id
        for field in self.meta.get_non_refer_fields() {
            let key = &field.get_field_name();
            result.column_index(key).map(|idx| {
                self.field_map.insert(field.get_field_name(), row.as_ref(idx).unwrap().clone());
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
        let id = self.field_map.get("id").unwrap().clone();
        params.insert(0, ("id".to_string(), id));
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| ())
    }
    pub fn do_get<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_get();
        let id = self.field_map.get("id").unwrap().clone();
        let params = vec![("id".to_string(), id.clone())];
        println!("{}, {:?}", sql, params);
        let res = conn.prep_exec(sql, params);
        if let Err(err) = res {
            return Err(err);
        }
        let mut res = res.unwrap();
        let row = res.next();
        if row.is_none() {
            // 没有读取到，返回id无效
            return Err(Error::MySqlError(MySqlError {
                state: "ID_NOT_EXIST".to_string(),
                message: id.into_str(),
                code: 60001,
            }));
        }
        let row = row.unwrap();
        if let Err(err) = row {
            return Err(err);
        }
        let mut row = row.unwrap();
        self.set_values(&res, &mut row, "");
        Ok(())
    }
    pub fn do_delete<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_delete();
        let id = self.field_map.get("id").unwrap().clone();
        let params = vec![("id".to_string(), id)];
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| ())
    }
}

impl EntityInner {
    fn fmt_rc(rc: &EntityInnerPointer) -> String {
        let rc = rc.clone();
        let inner = rc.borrow();
        format!("{:?}", inner)
    }
    fn fmt_map_value(map: &HashMap<String, Value>) -> String {
        map.iter()
            .map(|(key, value)| format!("{}: \"{:?}\"", key, value))
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_map_opt(map: &HashMap<String, Option<EntityInnerPointer>>) -> String {
        map.iter()
            .map(|(key, value)| {
                let value_string = value.as_ref().map_or("NULL".to_string(), Self::fmt_rc);
                format!("{}: {}", key, value_string)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    fn fmt_map_vec(map: &HashMap<String, Vec<EntityInnerPointer>>) -> String {
        map.iter()
            .map(|(key, vec)| {
                // let value_string = value.as_ref().map_or("NULL".to_string(), Self::fmt_rc);
                let value_string = vec.iter().map(Self::fmt_rc).collect::<Vec<_>>().join(", ");
                format!("{}: [{}]", key, value_string)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let many_many_map = self.many_many_map
            .iter()
            .map(|(ref key, ref vec_pair)| {
                println!("{:?}", vec_pair);
                let vec = vec_pair.iter().map(|&(_, ref b_rc)| b_rc.clone()).collect::<Vec<_>>();
                (key.to_string(), vec)
                // (key.to_string(), vec.iter().map(&|(_, ref b_rc)| b_rc.clone()).collect::<Vec<_>>())
            })
            .collect::<HashMap<_, _>>();
        let inner = vec![Self::fmt_map_value(&self.field_map),
                         Self::fmt_map_opt(&self.pointer_map),
                         Self::fmt_map_opt(&self.one_one_map),
                         Self::fmt_map_vec(&self.one_many_map),
                         Self::fmt_map_vec(&many_many_map)]
            .into_iter()
            .filter(|s| s.len() > 0)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{{{}}}", inner)
    }
}

pub trait Entity {
    fn orm_meta() -> &'static OrmMeta;
    fn meta() -> &'static EntityMeta;
    fn default() -> Self;
    fn new(inner: EntityInnerPointer) -> Self;
    fn inner(&self) -> EntityInnerPointer;
    fn debug(&self) {
        let inner = self.inner();
        let inner = inner.borrow();
        let inner = inner.deref();
        let entity = &Self::meta().entity_name;
        println!("{}: {:?}", entity, inner);
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

    fn inner_get<V>(&self, key: &str) -> V
        where V: FromValue
    {
        let opt = self.do_inner(|inner| inner.get::<V>(key));
        opt.expect(&format!("[{}] Get [{}] Of None", Self::meta().entity_name, key))
    }
    fn inner_set<V>(&self, key: &str, value: V)
        where Value: From<V>
    {
        self.do_inner_mut(|inner| inner.set::<V>(key, Some(value)));
    }
    fn inner_has<V>(&self, key: &str) -> bool
        where V: FromValue
    {
        self.do_inner(|inner| inner.get::<V>(key)).is_some()
    }
    fn inner_clear<V>(&self, key: &str)
        where Value: From<V>
    {
        self.do_inner_mut(|inner| inner.set::<V>(key, None));
    }

    fn inner_get_pointer<E>(&self, key: &str) -> E
        where E: Entity
    {
        let opt = self.do_inner_mut(|inner| inner.get_pointer(key)).map(|rc| E::new(rc));
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
        let opt = self.do_inner_mut(|inner| inner.get_one_one(key)).map(|rc| E::new(rc));
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
        vec.into_iter().map(E::new).collect::<Vec<_>>()
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

    fn set_id(&self, id: u64) {
        self.inner_set("id", id);
    }
    fn get_id(&self) -> u64 {
        self.inner_get("id")
    }
    fn has_id(&self) -> bool {
        self.inner_has::<u64>("id")
    }
    fn clear_id(&self) {
        self.inner_clear::<u64>("id")
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
