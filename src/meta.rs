use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Annotation {
    Id(bool),
    Len(u64),
    PointTo(String),
    HasOne(String),
    HasMany(String),
    ManyToMany(String),
}

#[derive(Debug, Default)]
pub struct FieldMeta {
    pub field_name: String,
    pub column_name: String,
    pub ty: String,
    pub db_ty: String,
    pub raw_ty: String,
    pub nullable: bool,
    pub len: usize,
    pub pkey: bool,
    pub annos: Vec<Annotation>,
}

type FieldMetaPtr = Rc<RefCell<FieldMeta>>;

#[derive(Debug, Default)]
pub struct EntityMeta {
    pub entity_name: String,
    pub table_name: String,
    pub pkey: FieldMetaPtr,
    pub fields: Vec<FieldMetaPtr>,
    pub field_map: HashMap<String, FieldMetaPtr>,
    pub column_map: HashMap<String, FieldMetaPtr>,
}

type EntityMetaPtr = Rc<RefCell<EntityMeta>>;

#[derive(Debug, Default)]
pub struct OrmMeta {
    pub entities: Vec<EntityMetaPtr>,
    pub entity_map: HashMap<String, EntityMetaPtr>,
    pub table_map: HashMap<String, EntityMetaPtr>,
}

impl EntityMeta {
    fn new_field(&mut self) -> FieldMetaPtr {
        self.fields.push(FieldMetaPtr::default());
        self.cur_field()
    }
    fn cur_field(&mut self) -> FieldMetaPtr {
        self.fields.last().unwrap().clone()
    }
}

impl OrmMeta {
    fn new_entity(&mut self) -> EntityMetaPtr {
        self.entities.push(EntityMetaPtr::default());
        self.cur_entity()
    }
    fn cur_entity(&mut self) -> EntityMetaPtr {
        self.entities.last().unwrap().clone()
    }
}