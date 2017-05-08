use std::collections::HashMap;
use std::cell::Cell;
use attr::Attr;
use std::str::FromStr;
use regex::Regex;

#[macro_use]
use macros;

use mysql;

const DEFAULT_LEN: u64 = 128;

#[derive(Debug, Clone, Copy, PartialEq, RustcDecodable, RustcEncodable)]
pub enum Cascade {
    NULL,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, RustcDecodable, RustcEncodable)]
pub enum Fetch {
    Lazy,
    Eager,
}

#[derive(Debug, Clone, RustcDecodable, RustcEncodable)]
pub enum FieldMeta {
    Id { auto: bool },
    Integer {
        field: String,
        column: String,
        number: String,
        nullable: bool,
    },
    String {
        field: String,
        column: String,
        len: u64,
        nullable: bool,
    },
    Refer {
        field: String,
        entity: String,
        left: String,
        right: String,
        cascades: Vec<Cascade>,
        fetch: Fetch,
    },
    Pointer {
        field: String,
        entity: String,
        left: String,
        right: String,
        cascades: Vec<Cascade>,
        fetch: Fetch,
    },
    OneToOne {
        field: String,
        entity: String,
        left: String,
        right: String,
        cascades: Vec<Cascade>,
        fetch: Fetch,
    },
    OneToMany {
        field: String,
        entity: String,
        left: String,
        right: String,
        cascades: Vec<Cascade>,
        fetch: Fetch,
    },
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct EntityMeta {
    pub entity_name: String,
    pub table_name: String,
    pub field_vec: Vec<String>,
    pub field_map: HashMap<String, FieldMeta>, // pub column_map: HashMap<String, FieldMeta>,
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct OrmMeta {
    pub entity_vec: Vec<String>,
    pub entity_map: HashMap<String, EntityMeta>, // pub table_map: HashMap<String, EntityMeta>,
}

impl FieldMeta {
    pub fn new_pkey(auto: bool) -> Self {
        FieldMeta::Id { auto: auto }
    }
    pub fn new_integer(field: &str, column: &str, number: &str, nullable: bool) -> Self {
        FieldMeta::Integer {
            field: field.to_string(),
            column: column.to_string(),
            number: number.to_string(),
            nullable: nullable,
        }
    }
    pub fn new_string(field: &str, column: &str, len: u64, nullable: bool) -> Self {
        FieldMeta::String {
            field: field.to_string(),
            column: column.to_string(),
            len: len,
            nullable: nullable,
        }
    }
    pub fn new_refer(field: &str,
                     entity: &str,
                     left: &str,
                     right: &str,
                     cascades: Vec<Cascade>,
                     fetch: Fetch)
                     -> Self {
        FieldMeta::Refer {
            field: field.to_string(),
            entity: entity.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            cascades: cascades,
            fetch: fetch,
        }
    }
    pub fn new_pointer(field: &str,
                       entity: &str,
                       left: &str,
                       right: &str,
                       cascades: Vec<Cascade>,
                       fetch: Fetch)
                       -> Self {
        FieldMeta::Pointer {
            field: field.to_string(),
            entity: entity.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            cascades: cascades,
            fetch: fetch,
        }
    }
    pub fn new_one_one(field: &str,
                       entity: &str,
                       left: &str,
                       right: &str,
                       cascades: Vec<Cascade>,
                       fetch: Fetch)
                       -> Self {
        FieldMeta::OneToOne {
            field: field.to_string(),
            entity: entity.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            cascades: cascades,
            fetch: fetch,
        }
    }
    pub fn new_one_many(field: &str,
                        entity: &str,
                        left: &str,
                        right: &str,
                        cascades: Vec<Cascade>,
                        fetch: Fetch)
                        -> Self {
        FieldMeta::OneToMany {
            field: field.to_string(),
            entity: entity.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            cascades: cascades,
            fetch: fetch,
        }
    }
}

impl FieldMeta {
    pub fn is_auto(&self) -> bool {
        match self {
            &FieldMeta::Id { ref auto } => auto.clone(),
            _ => unreachable!(),
        }
    }
    pub fn get_column_name(&self) -> String {
        match self {
            &FieldMeta::Id { .. } => "id".to_string(),
            &FieldMeta::Integer { ref column, .. } => column.to_string(),
            &FieldMeta::String { ref column, .. } => column.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_field_name(&self) -> String {
        match self {
            &FieldMeta::Id { .. } => "id".to_string(),
            &FieldMeta::Integer { ref field, .. } => field.to_string(),
            &FieldMeta::String { ref field, .. } => field.to_string(),
            &FieldMeta::Refer { ref field, .. } => field.to_string(),
            &FieldMeta::Pointer { ref field, .. } => field.to_string(),
            &FieldMeta::OneToOne { ref field, .. } => field.to_string(),
            &FieldMeta::OneToMany { ref field, .. } => field.to_string(),
        }
    }
    pub fn get_type_name(&self) -> String {
        match self {
            &FieldMeta::Id { .. } => "u64".to_string(),
            &FieldMeta::Integer { ref number, .. } => number.to_string(),
            &FieldMeta::String { .. } => "String".to_string(),
            &FieldMeta::Refer { ref entity, .. } => entity.to_string(),
            &FieldMeta::Pointer { ref entity, .. } => entity.to_string(),
            &FieldMeta::OneToOne { ref entity, .. } => entity.to_string(),
            &FieldMeta::OneToMany { ref entity, .. } => entity.to_string(),
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
        let nullable_fn = |nullable| match nullable {
            true => "",
            false => " NOT NULL",
        };
        let auto_fn = |auto| match auto {
            true => " AUTO_INCREMENT",
            false => "",
        };
        match self {
            &FieldMeta::Id { ref auto } => {
                format!("`id` BIGINT PRIMARY KEY NOT NULL{}", auto_fn(auto.clone()))
            }
            &FieldMeta::Integer { ref number, ref column, ref nullable, .. } => {
                format!("`{}` {}{}",
                        column,
                        Self::get_db_type_number(number),
                        nullable_fn(nullable.clone()))
            }
            &FieldMeta::String { ref len, ref column, ref nullable, .. } => {
                format!("`{}` VARCHAR({}){}",
                        column,
                        len,
                        nullable_fn(nullable.clone()))
            }
            _ => unreachable!(),
        }
    }
    pub fn get_type_name_set(&self) -> String {
        match self {
            &FieldMeta::Id { .. } => self.get_type_name(),
            &FieldMeta::Integer { .. } => self.get_type_name(),
            &FieldMeta::String { .. } => "&str".to_string(),
            &FieldMeta::Refer { ref entity, .. } => format!("&{}", entity),
            &FieldMeta::Pointer { ref entity, .. } => format!("&{}", entity),
            &FieldMeta::OneToOne { ref entity, .. } => format!("&{}", entity),
            &FieldMeta::OneToMany { ref entity, .. } => format!("&{}", entity),
        }
    }

    pub fn is_type_id(&self) -> bool {
        match self {
            &FieldMeta::Id { .. } => true,
            _ => false,
        }
    }
    pub fn is_type_normal(&self) -> bool {
        match self {
            &FieldMeta::Integer { .. } => true,
            &FieldMeta::String { .. } => true,
            _ => false,
        }
    }
    pub fn is_type_refer(&self) -> bool {
        match self {
            &FieldMeta::Refer { .. } => true,
            &FieldMeta::Pointer { .. } => true,
            &FieldMeta::OneToOne { .. } => true,
            &FieldMeta::OneToMany { .. } => true,
            _ => false,
        }
    }

    pub fn get_refer_entity(&self) -> String {
        match self {
            &FieldMeta::Refer { ref entity, .. } => entity.to_string(),
            &FieldMeta::Pointer { ref entity, .. } => entity.to_string(),
            &FieldMeta::OneToOne { ref entity, .. } => entity.to_string(),
            &FieldMeta::OneToMany { ref entity, .. } => entity.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_cascades(&self) -> &Vec<Cascade> {
        match self {
            &FieldMeta::Refer { ref cascades, .. } => cascades,
            &FieldMeta::Pointer { ref cascades, .. } => cascades,
            &FieldMeta::OneToOne { ref cascades, .. } => cascades,
            &FieldMeta::OneToMany { ref cascades, .. } => cascades,
            _ => unreachable!(),
        }
    }
    pub fn get_refer_fetch(&self) -> Fetch {
        match self {
            &FieldMeta::Refer { ref fetch, .. } => fetch.clone(),
            &FieldMeta::Pointer { ref fetch, .. } => fetch.clone(),
            &FieldMeta::OneToOne { ref fetch, .. } => fetch.clone(),
            &FieldMeta::OneToMany { ref fetch, .. } => fetch.clone(),
            _ => unreachable!(),
        }
    }

    pub fn get_refer_left(&self) -> String {
        match self {
            &FieldMeta::Refer { ref left, .. } => left.to_string(),
            &FieldMeta::Pointer { ref left, .. } => left.to_string(),
            &FieldMeta::OneToOne { ref left, .. } => left.to_string(),
            &FieldMeta::OneToMany { ref left, .. } => left.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_right(&self) -> String {
        match self {
            &FieldMeta::Refer { ref right, .. } => right.to_string(),
            &FieldMeta::Pointer { ref right, .. } => right.to_string(),
            &FieldMeta::OneToOne { ref right, .. } => right.to_string(),
            &FieldMeta::OneToMany { ref right, .. } => right.to_string(),
            _ => unreachable!(),
        }
    }

    pub fn has_cascade_insert(&self) -> bool {
        for cascade in self.get_refer_cascades() {
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
        for cascade in self.get_refer_cascades() {
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
        for cascade in self.get_refer_cascades() {
            match cascade {
                &Cascade::Delete => return true,
                _ => {
                    continue;
                }
            }
        }
        return false;
    }
    pub fn is_fetch_eager(&self) -> bool {
        self.get_refer_fetch() == Fetch::Eager
    }
}

impl FieldMeta {
    // pub fn format(&self, value: mysql::Value) -> String {
    //     if value == mysql::Value::NULL {
    //         return "null".to_string();
    //     }
    //     match self {
    //         &FieldMeta::Id => mysql::from_value::<u64>(value).to_string(),
    //         &FieldMeta::Integer { .. } => mysql::from_value::<i64>(value).to_string(),
    //         &FieldMeta::String { .. } => format!("\"{}\"", mysql::from_value::<String>(value)),
    //         _ => unreachable!(),
    //     }
    // }
}

impl EntityMeta {
    pub fn is_id_auto(&self) -> bool {
        self.field_map.get("id").unwrap().is_auto()
    }
    pub fn get_fields(&self) -> Vec<&FieldMeta> {
        self.field_vec.iter().map(|field_name| self.field_map.get(field_name).unwrap()).collect()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_type_normal())
            .collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| !field.is_type_refer())
            .collect::<Vec<_>>()
    }

    pub fn get_refer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| match field {
                &&FieldMeta::Refer { .. } => true,
                _ => false,
            })
            .collect::<Vec<_>>()
    }
    pub fn get_pointer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| match field {
                &&FieldMeta::Pointer { .. } => true,
                _ => false,
            })
            .collect::<Vec<_>>()
    }
    pub fn get_one_one_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| match field {
                &&FieldMeta::OneToOne { .. } => true,
                _ => false,
            })
            .collect::<Vec<_>>()
    }
    pub fn get_one_many_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| match field {
                &&FieldMeta::OneToMany { .. } => true,
                _ => false,
            })
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

impl OrmMeta {
    pub fn get_entities(&self) -> Vec<&EntityMeta> {
        self.entity_vec
            .iter()
            .map(|entity_name| self.entity_map.get(entity_name).expect(expect!().as_ref()))
            .collect()
    }
}
