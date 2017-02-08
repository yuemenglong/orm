use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use mysql::Value;
use mysql::Error;
use mysql::error::MySqlError;
use mysql::value::from_value;
use mysql::prelude::FromValue;
use mysql::QueryResult;
use mysql::Row;
use mysql::prelude::GenericConnection;

use meta::EntityMeta;
use meta::FieldMeta;
use sql;

pub type EntityInnerPointer = Rc<RefCell<EntityInner>>;

#[derive(Clone, Default, Debug)]
pub struct EntityInner {
    fields: HashMap<String, Value>,
    refers: HashMap<String, EntityInnerPointer>,
}

impl EntityInner {
    pub fn set<V>(&mut self, key: &str, value: Option<V>)
        where Value: From<V>
    {
        match value {
            None => self.fields.remove(key),
            Some(v) => self.fields.insert(key.to_string(), Value::from(v)),
        };
    }
    pub fn get<V>(&self, key: &str) -> Option<V>
        where V: FromValue
    {
        self.fields.get(key).map(|value| from_value(value.clone()))
    }
    pub fn has(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    pub fn set_refer(&mut self, key: &str, value: Option<EntityInnerPointer>) {
        match value {
            None => self.refers.remove(key),
            Some(inner) => self.refers.insert(key.to_string(), inner.clone()),
        };
    }
    pub fn get_refer(&self, key: &str) -> Option<&EntityInnerPointer> {
        self.refers.get(key)
    }
    pub fn has_refer(&self, key: &str) -> bool {
        self.refers.contains_key(key)
    }
}

pub trait Entity {
    fn meta() -> &'static EntityMeta;
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

    fn do_inner_get<R>(&self, key: &str) -> R
        where R: FromValue
    {
        self.do_inner(|inner| inner.get(key).unwrap())
    }
    fn do_inner_set<V>(&self, key: &str, value: V)
        where Value: From<V>
    {
        self.do_inner_mut(|inner| inner.set(key, Some(value)))
    }
    fn do_inner_has(&self, key: &str) -> bool {
        self.do_inner(|inner| inner.has(key))
    }

    fn set_id(&mut self, id: u64) {
        self.do_inner_mut(|inner| inner.set("id", Some(id)));
    }
    fn get_id(&self) -> u64 {
        self.do_inner(|inner| inner.get("id").unwrap())
    }
    fn has_id(&self) -> bool {
        self.do_inner(|inner| inner.has("id"))
    }

    fn get_columns() -> Vec<String> {
        sql::entity_get_columns(Self::meta())
    }
    fn get_values(&self) -> Vec<Value> {
        // 不包括id
        let meta = Self::meta();
        meta.fields
            .iter()
            .map(|field| {
                self.do_inner(|inner| Value::from(inner.fields.get(&field.field_name).clone()))
            })
            .collect::<Vec<_>>()
    }
    fn get_params(&self) -> Vec<(String, Value)> {
        Self::get_columns().into_iter().zip(self.get_values().into_iter()).collect::<Vec<_>>()
    }
    fn set_values(&mut self,
                  result: &QueryResult,
                  row: &mut Row,
                  prefix: &str)
                  -> Result<(), Error> {
        let meta = Self::meta();
        let fields = meta.get_non_refer_fields();

        fields.iter().fold(Ok(()), |acc, field| {
            if acc.is_err() {
                return acc;
            }
            let key = &field.field_name;
            match result.column_index(key) {
                Some(idx) => {
                    let value = row.as_ref(idx).clone();
                    self.do_inner_mut(|inner| inner.set(key, value));
                    Ok(())
                }
                None => {
                    let state = "ORM_INVALID_COLUMN_NAME".to_string();
                    let message = key.to_string();
                    let code = 60001;
                    Err(Error::MySqlError(MySqlError {
                        state: state,
                        message: message,
                        code: code,
                    }))
                }
            }
        })
    }

    // fn get_refer<E:Entity>(&self, field: &str) -> Option<&E>;
    // fn set_refer(&mut self, field: &str, e: Option<Entity>);

    fn do_insert<C>(&self, conn: &mut C) -> Result<Self, Error>
        where C: GenericConnection,
              Self: Clone
    {
        let sql = sql::sql_insert(Self::meta());
        println!("{}", sql);
        let res = conn.prep_exec(sql, self.get_params());
        match res {
            Ok(res) => {
                let mut ret = (*self).clone();
                ret.set_id(res.last_insert_id());
                Ok(ret)
            }
            Err(err) => Err(err),
        }
    }

    // fn get_name() -> String;
    // // fn get_field_meta() -> Vec<FieldMeta>;
    // fn get_params(&self) -> Vec<(String, Value)>;
    // fn from_row(row: Row) -> Self;
    // fn from_row_ex(row: Row, nameMap: &HashMap<String, String>) -> Self;

    // fn get_create_table() -> String;
    // fn get_drop_table() -> String;

    // fn get_field_list() -> String;
    // fn get_prepare() -> String;
    // fn get_params_id(&self) -> Vec<(String, Value)>;
    //  {
    //     vec![("id".to_string(), Value::from(self.get_id()))]
    // }
}
