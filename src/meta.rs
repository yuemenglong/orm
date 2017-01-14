use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

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
    pub extend: bool, // 是否为系统自动扩展出的属性
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

impl FieldMeta {
    pub fn create_pkey() -> FieldMetaPtr {
        Rc::new(RefCell::new(FieldMeta {
            field_name: "id".to_string(),
            column_name: "id".to_string(),
            ty: "u64".to_string(),
            db_ty: "`id` BIGINT PRIMARY KEY AUTOINCREMENT".to_string(),
            raw_ty: "Option<u64>".to_string(),
            nullable: false,
            len: 0,
            pkey: true,
            extend: true,
        }))
    }
}

impl EntityMeta {
    pub fn new_field(&mut self) -> FieldMetaPtr {
        self.fields.push(FieldMetaPtr::default());
        self.cur_field()
    }
    pub fn cur_field(&mut self) -> FieldMetaPtr {
        self.fields.last().unwrap().clone()
    }
}

impl OrmMeta {
    pub fn new_entity(&mut self) -> EntityMetaPtr {
        self.entities.push(EntityMetaPtr::default());
        self.cur_entity()
    }
    pub fn cur_entity(&mut self) -> EntityMetaPtr {
        self.entities.last().unwrap().clone()
    }
}
