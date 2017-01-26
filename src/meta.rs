use std::collections::HashMap;
use rustc_serialize::json;

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct FieldMeta {
    pub field_name: String,
    pub column_name: String,
    pub ty: String,
    pub db_ty: String,
    pub nullable: bool,
    pub len: u64,
    pub pkey: bool,
    pub extend: bool, // 是否为系统自动扩展出的属性
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct EntityMeta {
    pub entity_name: String,
    pub table_name: String,
    pub pkey: FieldMeta,
    pub fields: Vec<FieldMeta>,
    pub field_map: HashMap<String, FieldMeta>,
    pub column_map: HashMap<String, FieldMeta>,
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct OrmMeta {
    pub entities: Vec<EntityMeta>,
    pub entity_map: HashMap<String, EntityMeta>,
    pub table_map: HashMap<String, EntityMeta>,
}

impl FieldMeta {
    pub fn create_pkey() -> FieldMeta {
        FieldMeta {
            field_name: "id".to_string(),
            column_name: "id".to_string(),
            ty: "u64".to_string(),
            db_ty: "`id` BIGINT PRIMARY KEY AUTO_INCREMENT".to_string(),
            nullable: false,
            len: 0,
            pkey: true,
            extend: true,
        }
    }
}
