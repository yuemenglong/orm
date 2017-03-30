use mysql::Value;

use entity::Entity;

use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;

pub struct Cond {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    alias: String,
    items: Vec<Item>,
}

impl Cond {
    pub fn new<E>() -> Cond
        where E: Entity
    {
        Cond {
            meta: E::meta(),
            orm_meta: E::orm_meta(),
            alias: E::meta().entity_name.to_string(),
            items: Vec::new(),
        }
    }
    pub fn from_alias<E>(alias: &str) -> Cond
        where E: Entity
    {
        Cond {
            meta: E::meta(),
            orm_meta: E::orm_meta(),
            alias: alias.to_string(),
            items: Vec::new(),
        }
    }
    pub fn from_meta(meta:&'static EntityMeta, orm_meta:&'static OrmMeta) -> Cond
    {
        Cond {
            meta: meta,
            orm_meta: orm_meta,
            alias: meta.entity_name.to_string(),
            items: Vec::new(),
        }
    }
    pub fn meta(&self)->&'static EntityMeta{
        self.meta
    }
    pub fn orm_meta(&self)->&'static OrmMeta{
        self.orm_meta
    }
    pub fn id(&mut self, id: u64) -> &mut Self {
        self.items.push(Item::Id(Value::from(id)));
        self
    }
    pub fn eq<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Eq(field.to_string(), Value::from(value)));
        self
    }
    pub fn gt<V>(&mut self, field: &str, value: V) -> &mut Self
        where Value: From<V>
    {
        self.items.push(Item::Gt(field.to_string(), Value::from(value)));
        self
    }
    pub fn to_sql(&self) -> String {
        self.items
            .iter()
            .map(|item| item.to_sql(&self.alias))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
    pub fn to_params(&self) -> Vec<(String, Value)> {
        self.items
            .iter()
            .map(|item| item.to_params(&self.alias))
            .collect::<Vec<_>>()
    }
}

#[derive(Debug)]
enum Item {
    Id(Value),
    Eq(String, Value),
    Gt(String, Value),
}

fn concat(alias: &str, field: &str) -> String {
    format!("{}_{}", alias.to_lowercase(), field)
}

impl Item {
    fn to_sql(&self, alias: &str) -> String {
        match self {
            &Item::Id(..) => format!("{}.id = :{}", alias, concat(alias, "id")),
            &Item::Eq(ref field, ..) => format!("{}.{} = :{}", alias, field, concat(alias, field)),
            &Item::Gt(ref field, ..) => format!("{}.{} > :{}", alias, field, concat(alias, field)),
        }
    }
    fn to_params(&self, alias: &str) -> (String, Value) {
        match self {
            &Item::Id(ref value) => (concat(alias, "id"), value.clone()),
            &Item::Eq(ref field, ref value) => (concat(alias, field), value.clone()),
            &Item::Gt(ref field, ref value) => (concat(alias, field), value.clone()),
        }
    }
}
