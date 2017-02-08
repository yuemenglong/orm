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
    pub refer: bool, // 是否为引用属性
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct EntityMeta {
    pub entity_name: String,
    pub table_name: String,
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

impl EntityMeta {
    pub fn get_id_fields(&self) -> Vec<&FieldMeta> {
        self.fields.iter().filter(|field| field.pkey).collect::<Vec<_>>()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.fields.iter().filter(|field| !field.refer && !field.pkey).collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields.iter().filter(|field| !field.refer).collect::<Vec<_>>()
    }
    pub fn get_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields.iter().filter(|field| field.refer).collect::<Vec<_>>()
    }
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
            refer: false,
        }
    }
    pub fn create_refer(field: &str, ty: &str) -> FieldMeta {
        FieldMeta {
            field_name: field.to_string(),
            column_name: "".to_string(),
            ty: ty.to_string(),
            db_ty: "".to_string(),
            nullable: true,
            len: 0,
            pkey: false,
            refer: true,
        }
    }
    pub fn create_refer_id(meta: &FieldMeta) -> FieldMeta {
        let field_name = format!("{}_id", meta.field_name);
        FieldMeta {
            field_name: field_name.clone(),
            column_name: field_name.clone(),
            ty: "u64".to_string(),
            db_ty: format!("`{}` BIGINT", &field_name),
            nullable: true,
            len: 0,
            pkey: false,
            refer: false,
        }
    }
}
