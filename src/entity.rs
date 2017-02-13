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
use meta::FieldMeta;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

#[derive(Clone)]
pub struct EntityInner {
    meta: &'static EntityMeta,
    fields: HashMap<String, Value>,
    refers: HashMap<String, EntityInnerPointer>,
}

impl EntityInner {
    pub fn new(meta: &'static EntityMeta) -> EntityInner {
        EntityInner {
            meta: meta,
            fields: HashMap::new(),
            refers: HashMap::new(),
        }
    }
    pub fn meta(&self) -> &'static EntityMeta {
        self.meta
    }
    pub fn fields(&self) -> &HashMap<String, Value> {
        &self.fields
    }
    pub fn refers(&self) -> &HashMap<String, EntityInnerPointer> {
        &self.refers
    }

    pub fn set<V>(&mut self, key: &str, value: Option<V>)
        where Value: From<Option<V>>
    {
        match value {
            None => self.fields.remove(key),
            Some(v) => self.fields.insert(key.to_string(), Value::from(Some(v))),
        };
    }
    pub fn get<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.fields.get(key).map(|value| value::from_value(value.clone()))
    }
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key) && self.fields.get(key).unwrap() != &Value::NULL
    }

    pub fn set_refer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        match value {
            None => self.refers.remove(key),
            Some(inner) => {
                // let field_meta = self.meta().field_map.get(key).unwrap();
                // let refer_id = field_meta.
                self.refers.insert(key.to_string(), inner.clone())
            }
        };
    }
    pub fn get_refer(&self, key: &str) -> Option<EntityInnerPointer> {
        self.refers.get(key).map(|rc| rc.clone())
    }
    pub fn has_refer(&self, key: &str) -> bool {
        self.refers.contains_key(key)
    }

    pub fn get_values(&self) -> Vec<Value> {
        // 不包括id
        self.meta()
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                self.fields
                    .get(&field.field_name)
                    .map(|value| value.clone())
                    .or(Some(Value::NULL))
                    .unwrap()
            })
            .collect::<Vec<_>>()
    }
    pub fn get_params(&self) -> Vec<(String, Value)> {
        // 不包括id
        self.meta()
            .get_normal_fields()
            .into_iter()
            .map(|field| {
                (field.column_name.clone(),
                 self.fields
                     .get(&field.field_name)
                     .map(|value| value.clone())
                     .or(Some(Value::NULL))
                     .unwrap())
            })
            .collect::<Vec<_>>()
    }
    pub fn set_values(&mut self, result: &QueryResult, row: &mut Row, prefix: &str) {
        // 包括id
        for field in self.meta.get_non_refer_fields() {
            let key = &field.field_name;
            result.column_index(key).map(|idx| {
                self.fields.insert(field.field_name.to_string(),
                                   row.as_ref(idx).unwrap().clone());
            });
        }
    }

    pub fn do_insert<C>(&mut self, conn: &mut C) -> Result<(), Error>
        where C: GenericConnection
    {
        let sql = self.meta().sql_insert();
        let params = self.get_params();
        println!("{}, {:?}", sql, params);
        conn.prep_exec(sql, params).map(|res| {
            self.fields.insert("id".to_string(), Value::from(res.last_insert_id()));
        })
    }
}

impl fmt::Debug for EntityInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{{ Fields: {:?}, Refers: {:?} }}",
               self.fields,
               self.refers)
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

    fn inner_get_refer<E>(&self, key: &str) -> Option<E>
        where E: Entity
    {
        self.do_inner(|inner| inner.get_refer(key)).map(|inner_rc| E::new(inner_rc))
    }
    fn inner_set_refer<E>(&self, key: &str, value: Option<&E>)
        where E: Entity
    {
        self.do_inner_mut(|inner| inner.set_refer(key, value.map(|v| v.inner())));
    }
    fn inner_has_refer(&self, key: &str) -> bool {
        self.do_inner(|inner| inner.has_refer(key))
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
