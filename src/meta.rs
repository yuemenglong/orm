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

    pub fn get_columns(&self) -> Vec<String> {
        self.get_normal_fields()
            .iter()
            .map(|field| field.column_name.clone())
            .collect::<Vec<_>>()
    }
    pub fn sql_create_table(&self) -> String {
        let fields = self.get_non_refer_fields()
            .iter()
            .map(|field| field.db_ty.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("CREATE TABLE IF NOT EXISTS `{}`({})",
                self.table_name,
                fields)
    }
    pub fn sql_drop_table(&self) -> String {
        format!("DROP TABLE IF EXISTS `{}`", self.table_name)
    }
    pub fn sql_insert(&self) -> String {
        let columns = self.get_columns().join(", ");
        let values = self.get_columns()
            .iter()
            .map(|column| format!(":{}", column))
            .collect::<Vec<_>>()
            .join(", ");
        format!("INSERT INTO `{}`({}) VALUES ({})",
                &self.table_name,
                &columns,
                &values)
    }
    pub fn sql_update(&self) -> String {
        let pairs = self.get_columns()
            .iter()
            .map(|column| format!("{} = :{}", column, column))
            .collect::<Vec<_>>()
            .join(", ");
        format!("UPDATE `{}` SET {} where id = :id",
                &self.table_name,
                &pairs)
    }
    pub fn sql_get(&self) -> String {
        format!("SELECT * FROM `{}` WHERE id = :id", &self.table_name)
    }
    pub fn sql_delete(&self) -> String {
        format!("DELETE FROM `{}` WHERE id = :id", &self.table_name)
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
        let column_name = format!("{}_id", field);
        FieldMeta {
            field_name: field.to_string(),
            column_name: column_name,
            ty: ty.to_string(),
            db_ty: format!("`{}` BIGINT", field),
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
            column_name: meta.column_name.clone(),
            ty: "u64".to_string(),
            db_ty: meta.db_ty.clone(),
            nullable: true,
            len: 0,
            pkey: false,
            refer: false,
        }
    }
}
