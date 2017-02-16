use std::collections::HashMap;
use rustc_serialize::json;

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
pub enum TypeNormalMeta {
    NULL,
    Number(String),
    String(u64),
}

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
pub enum TypeReferMeta {
    NULL,
    Pointer { id: String },
}

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
pub enum TypeMeta {
    NULL,
    Id,
    Normal {
        column: String,
        normal: TypeNormalMeta,
    },
    Refer {
        entity: String,
        refer: TypeReferMeta,
    },
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct FieldMeta {
    field_name: String,
    ty: TypeMeta,
    nullable: bool,
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct EntityMeta {
    pub entity_name: String,
    pub table_name: String,
    pub fields: Vec<FieldMeta>,
    pub field_map: HashMap<String, FieldMeta>, // pub column_map: HashMap<String, FieldMeta>,
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct OrmMeta {
    pub entities: Vec<EntityMeta>,
    pub entity_map: HashMap<String, EntityMeta>, // pub table_map: HashMap<String, EntityMeta>,
}

impl Default for TypeNormalMeta {
    fn default() -> TypeNormalMeta {
        TypeNormalMeta::NULL
    }
}

impl Default for TypeReferMeta {
    fn default() -> TypeReferMeta {
        TypeReferMeta::NULL
    }
}

impl Default for TypeMeta {
    fn default() -> TypeMeta {
        TypeMeta::NULL
    }
}

impl TypeNormalMeta {
    pub fn type_name(&self) -> String {
        match self {
            &TypeNormalMeta::Number(ref ty) => ty.to_string(),
            &TypeNormalMeta::String(..) => "String".to_string(),
            _ => unreachable!(),
        }
    }
    pub fn db_type_string(&self) -> String {
        match self {
            &TypeNormalMeta::Number(ref ty) => {
                match ty.as_ref() {
                    "i32" => format!("INTEGER"),
                    "i64" => format!("BIGINT"),
                    "u64" => format!("BIGINT"),
                    _ => unreachable!(),
                }
            }
            &TypeNormalMeta::String(ref len) => format!("VARCHAR({})", len),
            _ => unreachable!(),
        }
    }
    pub fn type_name_set(&self) -> String {
        match self {
            &TypeNormalMeta::String(..) => "&str".to_string(),
            _ => self.type_name(),
        }
    }
}

impl TypeMeta {
    pub fn is_id(&self) -> bool {
        match self {
            &TypeMeta::Id => true,
            _ => false,
        }
    }
    pub fn is_normal(&self) -> bool {
        match self {
            &TypeMeta::Normal { .. } => true,
            _ => false,
        }
    }
    pub fn is_refer(&self) -> bool {
        match self {
            &TypeMeta::Refer { .. } => true,
            _ => false,
        }
    }
}

impl FieldMeta {
    pub fn column(&self) -> String {
        match self.ty {
            TypeMeta::Id => "id".to_string(),
            TypeMeta::Normal { column: ref column, .. } => column.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn field(&self) -> String {
        self.field_name.to_string()
    }
    pub fn type_name(&self) -> String {
        match self.ty {
            TypeMeta::Id => "u64".to_string(),
            TypeMeta::Normal { normal: ref normal, .. } => normal.type_name(),
            TypeMeta::Refer { entity: ref entity, .. } => entity.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn db_type_string(&self) -> String {
        let nullableFn = |nullable| match nullable {
            true => "",
            false => " NOT NULL",
        };
        match self.ty {
            TypeMeta::Id => "`id` BIGINT PRIMARY KEY AUTO_INCREMENT".to_string(),
            TypeMeta::Normal { column: ref column, normal: ref normal } => {
                format!("`{}` {}{}",
                        column,
                        normal.db_type_string(),
                        nullableFn(self.nullable))
            }
            _ => unreachable!(),
        }
    }
    pub fn type_name_set(&self) -> String {
        match self.ty {
            TypeMeta::Id => self.type_name(),
            TypeMeta::Normal { normal: ref normal, .. } => normal.type_name_set(),
            TypeMeta::Refer { entity: ref entity, .. } => format!("&{}", entity),
            _ => unreachable!(),
        }
    }
    pub fn as_refer(&self) -> &TypeReferMeta {
        match self.ty {
            TypeMeta::Refer { refer: ref refer, .. } => refer,
            _ => unreachable!(),
        }
    }
    pub fn get_refer_pointer_id(&self) -> String {
        if let TypeMeta::Refer { refer: ref refer, .. } = self.ty {
            if let &TypeReferMeta::Pointer { id: ref id } = refer {
                return id.to_string();
            }
        }
        unreachable!();
    }

    pub fn create_pkey() -> FieldMeta {
        FieldMeta {
            field_name: "id".to_string(),
            ty: TypeMeta::Id,
            nullable: false,
        }
    }
    pub fn create_string(field: &str, len: u64, nullable: bool) -> FieldMeta {
        FieldMeta {
            field_name: field.to_string(),
            ty: TypeMeta::Normal {
                column: field.to_string(),
                normal: TypeNormalMeta::String(len),
            },
            nullable: nullable,
        }
    }
    pub fn create_number(field: &str, ty: &str, nullable: bool) -> FieldMeta {
        FieldMeta {
            field_name: field.to_string(),
            ty: TypeMeta::Normal {
                column: field.to_string(),
                normal: TypeNormalMeta::Number(ty.to_string()),
            },
            nullable: nullable,
        }
    }
    pub fn create_pointer(field: &str, entity: &str, nullable: bool) -> FieldMeta {
        let refer_id = format!("{}_id", field);
        FieldMeta {
            field_name: field.to_string(),
            ty: TypeMeta::Refer {
                entity: entity.to_string(),
                refer: TypeReferMeta::Pointer { id: refer_id.to_string() },
            },
            nullable: nullable,
        }
    }
    pub fn create_pointer_id(meta: &FieldMeta) -> FieldMeta {
        let refer_id = meta.get_refer_pointer_id();
        FieldMeta {
            field_name: refer_id.to_string(),
            ty: TypeMeta::Normal {
                column: refer_id.to_string(),
                normal: TypeNormalMeta::Number("u64".to_string()),
            },
            nullable: meta.nullable,
        }
    }
}

impl EntityMeta {
    pub fn get_id_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.ty.is_id())
            .collect::<Vec<_>>()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.ty.is_normal())
            .collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| !field.ty.is_refer())
            .collect::<Vec<_>>()
    }
    pub fn get_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.ty.is_refer())
            .collect::<Vec<_>>()
    }

    pub fn get_columns(&self) -> Vec<String> {
        self.get_normal_fields()
            .iter()
            .map(|field| field.column())
            .collect::<Vec<_>>()
    }
    pub fn sql_create_table(&self) -> String {
        let fields = self.get_non_refer_fields()
            .iter()
            .map(|field| field.db_type_string().to_string())
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
