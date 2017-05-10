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
// use session::Session;
// use session::SessionStatus;
// use select::Select;

use entity::EntityInnerPointer;

// use cond::Cond;

#[derive(Debug, Clone)]
pub enum FieldValue {
    Value(Value),
    Entity(Option<EntityInnerPointer>),
    Vec(Vec<EntityInnerPointer>),
}

impl FieldValue {
    pub fn default(meta: &FieldMeta) -> Self {
        match meta {
            &FieldMeta::Id { .. } |
            &FieldMeta::Integer { .. } |
            &FieldMeta::String { .. } => FieldValue::Value(Value::NULL),
            &FieldMeta::Refer { .. } |
            &FieldMeta::Pointer { .. } |
            &FieldMeta::OneToOne { .. } => FieldValue::Entity(None),
            &FieldMeta::OneToMany { .. } => FieldValue::Vec(Vec::new()),
        }
    }
}

impl FieldValue {
    pub fn is_value(&self) -> bool {
        match self {
            &FieldValue::Value(_) => true,
            _ => false,
        }
    }
    pub fn is_entity(&self) -> bool {
        match self {
            &FieldValue::Entity(_) => true,
            _ => false,
        }
    }
    pub fn is_vec(&self) -> bool {
        match self {
            &FieldValue::Vec(_) => true,
            _ => false,
        }
    }
    pub fn as_value(&self) -> Value {
        match self {
            &FieldValue::Value(ref value) => value.clone(),
            _ => unreachable!(),
        }
    }
    pub fn as_entity(&self) -> Option<EntityInnerPointer> {
        match self {
            &FieldValue::Entity(ref opt) => opt.clone(),
            _ => unreachable!(),
        }
    }
    pub fn as_vec(&self) -> Vec<EntityInnerPointer> {
        match self {
            &FieldValue::Vec(ref vec) => vec.clone(),
            _ => unreachable!(),
        }
    }
    pub fn as_vec_mut(&mut self) -> &mut Vec<EntityInnerPointer> {
        match self {
            &mut FieldValue::Vec(ref mut vec) => vec,
            _ => unreachable!(),
        }
    }

    pub fn null() -> Self {
        FieldValue::from(Value::NULL)
    }
}

impl FieldValue {
    pub fn to_json(&self, meta: &FieldMeta) -> String {
        if !meta.is_type_refer() && self.as_value() == Value::NULL {
            return "NULL".to_string();
        }
        match meta {
            &FieldMeta::Id { .. } |
            &FieldMeta::Integer { .. } => format!("{}", value::from_value::<u64>(self.as_value())),
            &FieldMeta::String { .. } => format!("\"{}\"", value::from_value::<String>(self.as_value())),
            &FieldMeta::Refer { .. } |
            &FieldMeta::Pointer { .. } |
            &FieldMeta::OneToOne { .. } => {
                self.as_entity().map_or("NULL".to_string(), |v| v.borrow().to_json())
            }
            &FieldMeta::OneToMany { .. } => {
                let content =
                    self.as_vec().into_iter().map(|v| v.borrow().to_json()).collect::<Vec<_>>().join(", ");
                format!("[{}]", content)
            }
        }
    }
}

impl From<Value> for FieldValue {
    fn from(value: Value) -> Self {
        FieldValue::Value(value)
    }
}

impl From<Option<EntityInnerPointer>> for FieldValue {
    fn from(value: Option<EntityInnerPointer>) -> Self {
        FieldValue::Entity(value)
    }
}

impl From<Vec<EntityInnerPointer>> for FieldValue {
    fn from(value: Vec<EntityInnerPointer>) -> Self {
        FieldValue::Vec(value)
    }
}
