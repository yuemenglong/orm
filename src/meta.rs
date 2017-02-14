use std::collections::HashMap;
use rustc_serialize::json;

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
pub enum TypeMeta {
    NULL,
    Id,
    Number(String),
    String(u64),
    Pointer(String, String), // refer_entity, refer_id
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct FieldMeta {
    pub field_name: String,
    pub column_name: String,
    pub ty: TypeMeta,
    pub nullable: bool,
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

impl Default for TypeMeta {
    fn default() -> TypeMeta {
        TypeMeta::NULL
    }
}

impl FieldMeta {
    pub fn ty(&self) -> String {
        match self.ty {
            TypeMeta::Id => "u64".to_string(),
            TypeMeta::Number(ref ty) => ty.to_string(),
            TypeMeta::String(ref len) => "String".to_string(),
            TypeMeta::Pointer(ref entity, _) => entity.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn db_ty(&self) -> String {
        let nullable = match self.nullable {
            true => "",
            false => " NOT NULL",
        };
        match self.ty {
            TypeMeta::Id => "`id` BIGINT PRIMARY KEY AUTO_INCREMENT".to_string(),
            TypeMeta::Number(ref ty) => {
                match ty.as_ref() {
                    "i32" => format!("`{}` INTEGER{}", self.column_name, nullable),
                    "i64" => format!("`{}` BIGINT{}", self.column_name, nullable),
                    "u64" => format!("`{}` BIGINT{}", self.column_name, nullable),
                    _ => unreachable!(),
                }
            }
            TypeMeta::String(ref len) => {
                format!("`{}` VARCHAR({}){}", self.column_name, len, nullable)
            }
            _ => unreachable!(),
        }
    }
    pub fn set_ty(&self) -> String {
        match self.ty {
            TypeMeta::String(_) => "&str".to_string(),
            _ => self.ty(),
        }
    }
    pub fn create_pkey() -> FieldMeta {
        FieldMeta {
            field_name: "id".to_string(),
            column_name: "id".to_string(),
            ty: TypeMeta::Id,
            nullable: false,
        }
    }
    pub fn create_normal(field: &str, ty: &str, nullable: bool) -> FieldMeta {
        FieldMeta {
            field_name: field.to_string(),
            column_name: field.to_string(),
            ty: TypeMeta::NULL,
            nullable: nullable,
        }
    }
    pub fn create_refer(field: &str, ty: &str, nullable: bool) -> FieldMeta {
        let refer_id = format!("{}_id", field);
        FieldMeta {
            field_name: field.to_string(),
            column_name: refer_id.to_string(),
            ty: TypeMeta::Pointer(ty.to_string(), refer_id.to_string()),
            nullable: nullable,
        }
    }
    pub fn create_refer_id(meta: &FieldMeta) -> FieldMeta {
        match meta.ty {
            TypeMeta::Pointer(ref entity_name, ref id_field) => {
                FieldMeta {
                    field_name: id_field.to_string(),
                    column_name: meta.column_name.clone(),
                    ty: TypeMeta::Number("u64".to_string()),
                    nullable: true,
                }
            }
            _ => unreachable!(),
        }
    }
}

impl EntityMeta {
    pub fn get_id_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| {
                match &field.ty {
                    &TypeMeta::Id => true,
                    _ => false,
                }
            })
            .collect::<Vec<_>>()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| {
                match &field.ty {
                    &TypeMeta::Number(_) => true,
                    &TypeMeta::String(_) => true,
                    _ => false,
                }
            })
            .collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| {
                match &field.ty {
                    &TypeMeta::Pointer(_, _) => false,
                    _ => true,
                }
            })
            .collect::<Vec<_>>()
    }
    pub fn get_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| {
                match field.ty {
                    TypeMeta::Pointer(_, _) => true,
                    _ => false,
                }
            })
            .collect::<Vec<_>>()
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
            .map(|field| field.db_ty().to_string())
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
