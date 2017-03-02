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
use meta::Cascade;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

#[derive(Clone)]
pub struct EntityInner {
    pub meta: &'static EntityMeta,
    pub field_map: HashMap<String, Value>,
    pub pointer_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_one_map: HashMap<String, Option<EntityInnerPointer>>,
    pub one_many_map: HashMap<String, Option<Vec<EntityInnerPointer>>>,

    pub cascade: Option<Cascade>,

    pub refers: HashMap<String, EntityInnerPointer>,
    pub bulks: HashMap<String, Option<Vec<EntityInnerPointer>>>,
}

impl EntityInner {
    pub fn new(meta: &'static EntityMeta) -> EntityInner {
        let field_map: HashMap<String, Value> = meta.get_non_refer_fields()
            .into_iter()
            .map(|meta| (meta.field(), Value::NULL))
            .collect();
        let pointer_map: HashMap<String, Option<EntityInnerPointer>> = meta.get_pointer_fields()
            .into_iter()
            .map(|meta| (meta.field(), None))
            .collect();
        let one_one_map: HashMap<String, Option<EntityInnerPointer>> = meta.get_one_one_fields()
            .into_iter()
            .map(|meta| (meta.field(), None))
            .collect();
        EntityInner {
            meta: meta,
            field_map: field_map,
            pointer_map: pointer_map,
            one_one_map: one_one_map,
            one_many_map: HashMap::new(),
            cascade: None,
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
                // a.b_id = NULL, a.b = None
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
        let refer_object = self.get_one_one(key);
        // 先将之前的b.a_id置为空
        if refer_object.is_some(){

        }

        let refer_id_field = refer_meta.get_one_one_id();
        let refer_object = refer_object.unwrap();
        match value {
            None => {
                //将之前的a_id置为空
                let refer_object = self.get_one_one(key);
                refer_object = refer_object.unwrap();
                // 到这里一定有b，只是可以为NULL
                // a.b = None, b.a_id = NULL
                if refer_object.is_some() {
                    // 且未设置为空
                    let refer_object = refer_object.unwrap();
                    refer_object.borrow_mut().field_map.insert(refer_id_field, Value::NULL);
                }
            }
            Some(inner_rc) => {
                // b.a_id = a.id, a.b = Some(b);
                let inner = inner_rc.borrow();
                let refer_id = inner.field_map.get("id").unwrap_or(Value::NULL);
                if refer_object.is_some() {
                    refer_object = refer_object.unwrap();
                    println!("{:?}=========================", refer_id_field);
                    println!("{:?}=========================", refer_id);
                    refer_object.field_map.insert(refer_id_field, refer_id.unwrap().clone());
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
}

fn set_one_one(a:&mut EntityInner, b_rc:EntityInnerPointer, a_b_meta:&FieldMeta){
    // a.b = b; b.a_id = a.id;
    let b = b_rc.borrow_mut();
    let b_field = a_b_meta.field();
    a.one_one_map.insert(b_field, b_rc.clone());
    let a_id_field = a_b_meta.get_one_one_id();
    let a_id = a.field_map.get("id").unwrap_or(Value::NULL).clone();
    b.field_map.insert(a_id_field, a_id);
}

fn clear_one_one(a:&mut EntityInner, a_b_meta:&FieldMeta){
    // a.b = None; old_b.a_id = NULL; 
    let b_field = a_b_meta.field();
    let old_b = a.one_one_map.insert(b_field, None);
    if old_b.is_some(){
        old_b = old_b.unwrap();
        let b_a_id_field= a_b_meta.get_one_one_id();
        old_b.field_map.insert(b_a_id_field, Value::NULL);
    }
}

fn clear_one_one_id(b_rc:EntityInnerPointer, a_b_meta:&FieldMeta){
    // b.a_id = NULL
    let a_id_field = a_b_meta.get_one_one_id();
    let b = b.rc.borrow_mut();
    b.field_map.insert(a_id_field, Value::NULL);
}

impl EntityInner {
    fn fmt_rc(rc: &EntityInnerPointer) -> String {
        let rc = rc.clone();
        let inner = rc.borrow();
        format!("{:?}", inner)
    }
    fn fmt_map_value(map: &HashMap<String, Value>) -> String {
        let content = map.iter()
            .map(|(key, value)| format!("{}: \"{:?}\"", key, value))
            .collect::<Vec<_>>()
            .join(", ");
        // format!("{{{}}}", content)
        content
    }
    fn fmt_map_single(map: &HashMap<String, Option<EntityInnerPointer>>) -> String {
        let content = map.iter()
            .map(|(key, value)| {
                let value_string = value.as_ref().map_or("NULL".to_string(), Self::fmt_rc);
                format!("{}: {}", key, value_string)
            })
            .collect::<Vec<_>>()
            .join(", ");
        // format!("{{{}}}", content)
        content
    }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let inner = vec![Self::fmt_map_value(&self.field_map),
                         Self::fmt_map_single(&self.pointer_map),
                         Self::fmt_map_single(&self.one_one_map)]
            .into_iter()
            .filter(|s| s.len() > 0)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{{{}}}", inner)
    }
}

pub trait Entity {
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
