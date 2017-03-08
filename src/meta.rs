use std::collections::HashMap;
use rustc_serialize::json;
use attr::Attr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, RustcDecodable, RustcEncodable)]
pub enum Cascade {
    NULL,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
enum TypeMeta {
    NULL,
    Id,
    Number {
        number: String,
        column: String,
        nullable: bool,
    },
    String {
        len: u64,
        column: String,
        nullable: bool,
    },
    Pointer {
        entity: String,
        table: String,
        refer_id: String,
        cascade: Vec<Cascade>,
    },
    OneToOne {
        entity: String,
        table: String,
        id: String,
        cascade: Vec<Cascade>,
    },
    OneToMany {
        entity: String,
        table: String,
        id: String,
        cascade: Vec<Cascade>,
    },
    ManyToMany {
        entity: String,
        table: String,
        mid: String,
        id: String,
        refer_id: String,
        cascade: Vec<Cascade>,
    },
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct FieldMeta {
    field: String,
    ty: TypeMeta,
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

impl Default for TypeMeta {
    fn default() -> TypeMeta {
        TypeMeta::NULL
    }
}

impl TypeMeta {}

impl FieldMeta {
    pub fn get_column_name(&self) -> String {
        match self.ty {
            TypeMeta::Id => "id".to_string(),
            TypeMeta::Number { column: ref column, .. } => column.to_string(),
            TypeMeta::String { column: ref column, .. } => column.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_field_name(&self) -> String {
        self.field.to_string()
    }
    pub fn get_type_name(&self) -> String {
        match self.ty {
            TypeMeta::Id => "u64".to_string(),
            TypeMeta::Number { number: ref number, .. } => number.to_string(),
            TypeMeta::String { .. } => "String".to_string(),
            TypeMeta::Pointer { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::OneToOne { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::OneToMany { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::ManyToMany { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::NULL => unreachable!(),
        }
    }
    fn get_db_type_number(number: &str) -> String {
        match number {
            "i32" => "INTEGER".to_string(),
            "u32" => "INTEGER".to_string(),
            "i64" => "BIGINT".to_string(),
            "u64" => "BIGINT".to_string(),
            _ => unreachable!(),
        }
    }
    fn get_db_type(&self) -> String {
        let nullableFn = |nullable| match nullable {
            true => "",
            false => " NOT NULL",
        };
        match self.ty {
            TypeMeta::Id => "`id` BIGINT PRIMARY KEY AUTO_INCREMENT".to_string(),
            TypeMeta::Number { number: ref number, column: ref column, nullable: ref nullable } => {
                format!("`{}` {}{}",
                        column,
                        Self::get_db_type_number(number),
                        nullableFn(nullable.clone()))
            }
            TypeMeta::String { len: ref len, column: ref column, nullable: ref nullable } => {
                format!("`{}` VARCHAR({}){}",
                        column,
                        len,
                        nullableFn(nullable.clone()))
            }
            _ => unreachable!(),
        }
    }
    pub fn get_type_name_set(&self) -> String {
        match self.ty {
            TypeMeta::Id => self.get_type_name(),
            TypeMeta::Number { .. } => self.get_type_name(),
            TypeMeta::String { .. } => "&str".to_string(),
            TypeMeta::Pointer { entity: ref entity, .. } => format!("&{}", entity),
            TypeMeta::OneToOne { entity: ref entity, .. } => format!("&{}", entity),
            TypeMeta::OneToMany { entity: ref entity, .. } => format!("&{}", entity),
            TypeMeta::ManyToMany { entity: ref entity, .. } => format!("&{}", entity),
            TypeMeta::NULL => unreachable!(),
        }
    }

    pub fn is_type_id(&self) -> bool {
        match self.ty {
            TypeMeta::Id => true,
            _ => false,
        }
    }
    pub fn is_type_normal(&self) -> bool {
        match self.ty {
            TypeMeta::Number { .. } => true,
            TypeMeta::String { .. } => true,
            _ => false,
        }
    }
    pub fn is_type_refer(&self) -> bool {
        match self.ty {
            TypeMeta::Pointer { .. } => true,
            TypeMeta::OneToOne { .. } => true,
            TypeMeta::OneToMany { .. } => true,
            TypeMeta::ManyToMany { .. } => true,
            _ => false,
        }
    }

    pub fn get_refer_entity(&self) -> String {
        match self.ty {
            TypeMeta::Pointer { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::OneToOne { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::OneToMany { entity: ref entity, .. } => entity.to_string(),
            TypeMeta::ManyToMany { entity: ref entity, .. } => entity.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_table(&self) -> String {
        match self.ty {
            TypeMeta::Pointer { table: ref table, .. } => table.to_string(),
            TypeMeta::OneToOne { table: ref table, .. } => table.to_string(),
            TypeMeta::OneToMany { table: ref table, .. } => table.to_string(),
            TypeMeta::ManyToMany { table: ref table, .. } => table.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_cascade(&self) -> &Vec<Cascade> {
        match self.ty {
            TypeMeta::Pointer { cascade: ref cascade, .. } => cascade,
            TypeMeta::OneToOne { cascade: ref cascade, .. } => cascade,
            TypeMeta::OneToMany { cascade: ref cascade, .. } => cascade,
            TypeMeta::ManyToMany { cascade: ref cascade, .. } => cascade,
            _ => unreachable!(),
        }
    }
    pub fn is_refer_pointer(&self) -> bool {
        match self.ty {
            TypeMeta::Pointer { .. } => true,
            _ => false,
        }
    }
    pub fn is_refer_one_one(&self) -> bool {
        match self.ty {
            TypeMeta::OneToOne { .. } => true,
            _ => false,
        }
    }
    pub fn is_refer_one_many(&self) -> bool {
        match self.ty {
            TypeMeta::OneToMany{ .. } => true,
            _ => false,
        }
    }
    pub fn get_pointer_id(&self) -> String {
        match self.ty {
            TypeMeta::Pointer { refer_id: ref refer_id, .. } => refer_id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_one_one_id(&self) -> String {
        match self.ty {
            TypeMeta::OneToOne { id: ref id, .. } => id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_one_many_id(&self) -> String {
        match self.ty {
            TypeMeta::OneToMany { id: ref id, .. } => id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_bulk_one_many_id(&self) -> String {
        unimplemented!()
    }

    pub fn has_cascade_insert(&self) -> bool {
        for cascade in self.get_refer_cascade() {
            match cascade {
                &Cascade::Insert => return true,
                _ => {
                    continue;
                }
            }
        }
        return false;
    }
    pub fn has_cascade_update(&self) -> bool {
        for cascade in self.get_refer_cascade() {
            match cascade {
                &Cascade::Update => return true,
                _ => {
                    continue;
                }
            }
        }
        return false;
    }
    pub fn has_cascade_delete(&self) -> bool {
        for cascade in self.get_refer_cascade() {
            match cascade {
                &Cascade::Delete => return true,
                _ => {
                    continue;
                }
            }
        }
        return false;
    }
}

impl FieldMeta {
    pub fn new_pkey(entity: &str) -> Vec<(String, FieldMeta)> {
        let meta = FieldMeta {
            field: "id".to_string(),
            ty: TypeMeta::Id,
        };
        vec![(entity.to_string(), meta)]
    }
    fn pick_nullable(attr: &Attr) -> bool {
        let default = true;
        attr.get("nullable").map_or(default, |str| bool::from_str(str).unwrap())
    }
    fn pick_len(attr: &Attr) -> u64 {
        let default = 64;
        attr.get("len").map_or(default, |str| u64::from_str(str).unwrap())
    }
    fn pick_cascade(attr: &Attr) -> Vec<Cascade> {
        attr.get_attr("cascade").map_or(Vec::new(), |attr| {
            attr.values.as_ref().map_or(Vec::new(), |values| {
                values.iter()
                    .map(|attr| {
                        match attr.name.as_ref() {
                            "insert" => Cascade::Insert,
                            "update" => Cascade::Update,
                            "delete" => Cascade::Delete,
                            _ => unreachable!(),
                        }
                    })
                    .collect::<Vec<_>>()
            })
        })
    }
    pub fn is_normal_type(ty: &str) -> bool {
        match ty {
            "i32" | "u32" | "i64" | "u64" => true,
            "String" => true,
            _ => false,
        }
    }
    pub fn new(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        if Self::is_normal_type(ty) {
            Self::new_normal(entity, field, ty, attr)
        } else {
            Self::new_refer(entity, field, ty, attr)
        }
    }
    fn new_normal(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        match ty {
            "i32" | "u32" | "i64" | "u64" => Self::new_number(entity, field, ty, attr),
            "String" => Self::new_string(entity, field, ty, attr),
            _ => unreachable!(),
        }
    }
    fn new_refer(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        if attr.has("pointer") {
            return Self::new_pointer(entity, field, ty, attr);
        } else if attr.has("one_one") {
            return Self::new_one_one(entity, field, ty, attr);
        }
        unreachable!()
    }

    fn new_string(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let meta = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::String {
                len: Self::pick_len(attr),
                column: field.to_string(),
                nullable: Self::pick_nullable(attr),
            },
        };
        vec![(entity.to_string(), meta)]
    }
    fn new_number(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let meta = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::Number {
                number: ty.to_string(),
                column: field.to_string(),
                nullable: Self::pick_nullable(attr),
            },
        };
        vec![(entity.to_string(), meta)]
    }
    fn new_pointer(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let refer_id_field = format!("{}_id", field);
        let entity_name = ty.to_string();
        let table_name = entity_name.to_string();
        // 对象与id都挂在A上
        let mut ret = FieldMeta::new_number(entity, refer_id_field.as_ref(), "u64", attr);
        let refer_object = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::Pointer {
                refer_id: refer_id_field.to_string(),
                entity: entity_name,
                table: table_name,
                cascade: Self::pick_cascade(attr),
            },
        };
        ret.push((entity.to_string(), refer_object));
        ret
    }
    fn new_one_one(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        // 对象挂在A上，id挂在B上
        let refer_id_field = format!("{}_id", entity.to_lowercase());
        let entity_name = ty.to_string();
        let table_name = entity_name.to_string();
        let mut ret = FieldMeta::new_number(ty, refer_id_field.as_ref(), "u64", attr);
        let refer_object = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::OneToOne {
                id: refer_id_field.to_string(),
                entity: entity_name,
                table: table_name,
                cascade: Self::pick_cascade(attr), /* refer: TypeReferMeta::OneToOne { id: refer_id_field.to_string() }, */
            },
        };
        ret.push((entity.to_string(), refer_object));
        ret
    }
}

impl EntityMeta {
    pub fn get_id_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.is_type_id())
            .collect::<Vec<_>>()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.is_type_normal())
            .collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| !field.is_type_refer())
            .collect::<Vec<_>>()
    }
    pub fn get_pointer_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.is_refer_pointer())
            .collect::<Vec<_>>()
    }
    pub fn get_one_one_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.is_refer_one_one())
            .collect::<Vec<_>>()
    }
    pub fn get_one_many_fields(&self) -> Vec<&FieldMeta> {
        self.fields
            .iter()
            .filter(|field| field.is_refer_one_many())
            .collect::<Vec<_>>()
    }

    pub fn get_columns(&self) -> Vec<String> {
        self.get_normal_fields()
            .iter()
            .map(|field| field.get_column_name())
            .collect::<Vec<_>>()
    }
    pub fn sql_create_table(&self) -> String {
        let fields = self.get_non_refer_fields()
            .iter()
            .map(|field| field.get_db_type())
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
