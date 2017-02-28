use std::collections::HashMap;
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

use meta::EntityMeta;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

#[derive(Clone)]
pub struct EntityInner {
    pub meta: &'static EntityMeta,
    pub field_map: HashMap<String, Value>,
    pub pointer_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_one_map: HashMap<String, Option<EntityInnerPointer>>,
    pub refers: HashMap<String, EntityInnerPointer>,
    pub bulks: HashMap<String, Option<Vec<EntityInnerPointer>>>,
}

impl EntityInner {
    pub fn new(meta: &'static EntityMeta) -> EntityInner {
        EntityInner {
            meta: meta,
            field_map: HashMap::new(),
            pointer_map: HashMap::new(),
            one_one_map: HashMap::new(),
            refers: HashMap::new(),
            bulks: HashMap::new(),
        }
    }

    pub fn set<V>(&mut self, key: &str, value: Option<V>)
        where Value: From<Option<V>>
    {
        match value {
            None => self.field_map.remove(key),
            Some(v) => self.field_map.insert(key.to_string(), Value::from(Some(v))),
        };
    }
    pub fn get<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.field_map.get(key).map(|value| value::from_value(value.clone()))
    }
    pub fn has(&self, key: &str) -> bool {
        self.field_map.contains_key(key) && self.field_map.get(key).unwrap() != &Value::NULL
    }

    pub fn set_pointer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        let refer_meta = self.meta.field_map.get(key).unwrap();
        let refer_id_field = refer_meta.get_pointer_id();
        match value {
            None => {
                // a.b_id => NULL, a.b = None
                self.field_map.insert(refer_id_field, Value::NULL);
                self.pointer_map.insert(key.to_string(), None);
            }
            Some(inner_rc) => {
                // a.b_id = b.id, a.b = Some(b);
                let inner = inner_rc.borrow();
                let refer_id = inner.field_map.get("id");
                if refer_id.is_some() {
                    self.field_map.insert(refer_id_field, refer_id.unwrap().clone());
                }
                self.pointer_map.insert(key.to_string(), Some(inner_rc.clone()));
            }
        }
    }
    pub fn get_pointer(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let refer_meta = self.meta.field_map.get(key).unwrap();
        let refer_id_field = refer_meta.get_pointer_id();
        match self.pointer_map.get(key) {
            None => {
                // lazy load
                // TODO
                unimplemented!()
            }
            Some(opt) => {
                // 里面是啥就是啥
                opt.as_ref().map(|inner| inner.clone())
            }
        }
    }
    pub fn has_pointer(&mut self, key: &str) -> bool {
        self.get_pointer(key).is_some()
    }

    pub fn set_one_one(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        let refer_meta = self.meta.field_map.get(key).unwrap();
        let refer_id_field = refer_meta.get_one_one_id();
        match value {
            None => {
                // a.b_id => NULL, a.b = None
                self.field_map.insert(refer_id_field, Value::NULL);
                self.one_one_map.insert(key.to_string(), None);
            }
            Some(inner_rc) => {
                // a.b_id = b.id, a.b = Some(b);
                let inner = inner_rc.borrow();
                let refer_id = inner.field_map.get("id");
                if refer_id.is_some() {
                    self.field_map.insert(refer_id_field, refer_id.unwrap().clone());
                }
                self.one_one_map.insert(key.to_string(), Some(inner_rc.clone()));
            }
        }
    }
    pub fn get_one_one(&mut self, key: &str) -> Option<EntityInnerPointer> {
        let refer_meta = self.meta.field_map.get(key).unwrap();
        let refer_id_field = refer_meta.get_one_one_id();
        match self.one_one_map.get("key") {
            None => {
                // lazy load
                // TODO
                unimplemented!()
            }
            Some(opt) => {
                // 里面是啥就是啥
                opt.as_ref().map(|inner| inner.clone())
            }
        }
    }
    pub fn has_one_one(&mut self, key: &str) -> bool {
        self.get_pointer(key).is_some()
    }

    pub fn set_refer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        let refer_meta = self.meta.field_map.get(key).unwrap();
        if refer_meta.is_refer_pointer() {
            return self.set_pointer(key, value);
        } else if refer_meta.is_refer_one_one() {
            return self.set_one_one(key, value);
        }
        unreachable!();
    }

    pub fn get_refer(&self, key: &str) -> Option<EntityInnerPointer> {
        self.refers.get(key).map(|rc| rc.clone())
    }
    pub fn has_refer(&self, key: &str) -> bool {
        self.refers.contains_key(key)
    }

    pub fn get_values(&self) -> Vec<Value> {
        // 不包括id
        self.meta
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                self.field_map
                    .get(&field.field())
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
                (field.column(),
                 self.field_map
                     .get(&field.field())
                     .map(|value| value.clone())
                     .or(Some(Value::NULL))
                     .unwrap())
            })
            .collect::<Vec<_>>()
    }
    pub fn set_values(&mut self, result: &QueryResult, row: &mut Row, prefix: &str) {
        // 包括id
        for field in self.meta.get_non_refer_fields() {
            let key = &field.field();
            result.column_index(key).map(|idx| {
                self.field_map.insert(field.field(), row.as_ref(idx).unwrap().clone());
            });
        }
    }

    pub fn do_insert<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta.sql_insert();
        let params = self.get_params();
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| {
            self.field_map.insert("id".to_string(), Value::from(res.last_insert_id()));
        })
    }
}

// 私有函数在这里实现
impl EntityInner {
    // fn set_refer_pointer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
    //     let refer_meta = self.meta.field_map.get(key).unwrap();
    //     let refer_id_field = refer_meta.get_pointer_id();
    //     match value {
    //         // 设为NULL等价于删除对象+对象引用id
    //         None => {
    //             self.field_map.remove(&refer_id_field);
    //             self.refers.remove(key);
    //         }
    //         // 写入对象+对象引用id
    //         Some(inner_rc) => {
    //             let inner = inner_rc.borrow();
    //             // 对引用id的操作
    //             if inner.has("id") {
    //                 // 对象有id的情况下更新引用id
    //                 self.field_map.insert(refer_id_field, inner.get("id").unwrap());
    //             } else {
    //                 // 对象没有id的情况下删除引用id
    //                 self.field_map.remove(&refer_id_field);
    //             }
    //             // 写入对象
    //             self.refers.insert(key.to_string(), inner_rc.clone());
    //         }
    //     };
    // }
    // fn set_refer_one_one(&mut self, key: &str, value: Option<EntityInnerPointer>) {
    //     let refer_meta = self.meta.field_map.get(key).unwrap();
    //     let refer_id_field = refer_meta.get_one_one_id();
    //     match value {
    //         // A中去掉对象，B中去掉id(如果A中有B的话)
    //         None => {
    //             let other = self.refers.remove(key);
    //             match other {
    //                 // 本来就没有，什么都不干
    //                 None => {}
    //                 // 有的话将B的引用id也去掉
    //                 Some(other) => {
    //                     other.borrow_mut().field_map.remove(&refer_id_field);
    //                 }
    //             };
    //         }
    //         Some(inner_rc) => {
    //             // A保存B对象，B保存A的id
    //             let mut inner = inner_rc.borrow_mut();
    //             if self.has("id") {
    //                 // 如果A有id，更新B上的
    //                 inner.field_map.insert(refer_id_field, self.get("id").unwrap());
    //             } else {
    //                 // 如果A没有id，删除B上的
    //                 inner.field_map.remove(&refer_id_field);
    //             }
    //             self.refers.insert(key.to_string(), inner_rc.clone());
    //         }
    //     };
    // }

    // pub fn get_bulk(&self, key: &str, idx: usize) -> Option<EntityInnerPointer> {
    //     match self.bulks.get(key) {
    //         None => None,//说明是延迟加载
    //         Some(opt) => {
    //             match opt {
    //                 &None => None,//说明真的没有
    //                 &Some(ref vec) => {
    //                     // 有并且已经加载
    //                     vec.get(idx).map(|rc| rc.clone())
    //                 }
    //             }
    //         }
    //     }
    // }
    // pub fn get_bulks(&self, key: &str, idx: u64) -> Option<&Vec<EntityInnerPointer>> {
    //     match self.bulks.get(key) {
    //         None => None,//说明是延迟加载
    //         Some(opt) => opt.as_ref(), // 有或者没有都由这个结构决定
    //     }
    // }
    // pub fn set_bulks(&self, key: &str, value: Vec<EntityInnerPointer>) {
    //     // 先把当下的都解开引用
    //     let bulk_meta = self.meta.field_map.get(key).unwrap();
    //     let bulk_id_field = bulk_meta.get_bulk_one_many_id();

    //     // 把目前的bulk都解开引用
    //     let bulk = self.bulks.get(key);
    //     if bulk.is_none() {
    //         // load
    //         unimplemented!();
    //     }
    //     let opt = bulk.unwrap();
    //     if opt.is_some() {
    //         let vec = opt.as_ref().unwrap();
    //         for rc in vec {
    //             let mut inner = rc.borrow_mut();
    //             inner.field_map.insert(bulk_id_field.to_string(), Value::NULL);
    //         }
    //     }

    //     // 把参数里的都加上引用
    //     let self_id = self.field_map.get("id").unwrap();
    //     for rc in value {
    //         let mut inner = rc.borrow_mut();
    //         inner.field_map.insert(bulk_id_field.to_string(), self_id.clone());
    //     }
    // }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = |content, len| {
            match len {
                0 => "".to_string(),
                _ => content,
            }
        };
        write!(f,
               "{{ {}, {}, {} }}",
               output(format!("FieldMap: {:?}", self.field_map),
                      self.field_map.len()),
               output(format!("PointerMap: {:?}", self.pointer_map),
                      self.pointer_map.len()),
               output(format!("OneOneMap: {:?}", self.one_one_map),
                      self.one_one_map.len()))
    }
}

pub trait Entity {
    fn meta() -> &'static EntityMeta;
    fn default() -> Self;
    fn new(inner: EntityInnerPointer) -> Self;
    fn inner(&self) -> EntityInnerPointer;

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

    fn inner_get<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.do_inner(|inner| inner.get::<V>(key))
    }
    fn inner_set<V>(&self, key: &str, value: Option<V>)
        where Value: From<Option<V>>
    {
        self.do_inner_mut(|inner| inner.set(key, value));
    }
    fn inner_has(&self, key: &str) -> bool {
        self.do_inner(|inner| inner.has(key))
    }

    fn inner_get_pointer<E>(&self, key: &str) -> Option<E>
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.get_pointer(key)).map(|rc| E::new(rc))
    }
    fn inner_set_pointer<E>(&self, key: &str, value: Option<&E>)
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.set_pointer(key, value.map(|v| v.inner())));
    }
    fn inner_has_pointer(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.has_pointer(key))
    }

    fn inner_get_one_one<E>(&self, key: &str) -> Option<E>
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.get_one_one(key)).map(|rc| E::new(rc))
    }
    fn inner_set_one_one<E>(&self, key: &str, value: Option<&E>)
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.set_one_one(key, value.map(|v| v.inner())));
    }
    fn inner_has_one_one(&self, key: &str) -> bool {
        self.do_inner_mut(|inner| inner.has_one_one(key))
    }

    fn set_id(&mut self, id: u64) {
        self.inner_set("id", Some(id));
    }
    fn get_id(&self) -> u64 {
        self.inner_get("id").unwrap()
    }
    fn has_id(&self) -> bool {
        self.inner_has("id")
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
