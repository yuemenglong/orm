use std::collections::HashMap;
use std::cell::Cell;
use attr::Attr;
use std::str::FromStr;
use regex::Regex;

use mysql;

const DEFAULT_LEN: u64 = 64;

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
enum TypeMeta {
    NULL,
    Id,
    Integer {
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
        refer_id: String,
        cascades: Vec<Cascade>,
        rt_cascade: Cell<Option<Cascade>>,
        fetch: Fetch,
    },
    OneToOne {
        entity: String,
        id: String,
        cascades: Vec<Cascade>,
        rt_cascade: Cell<Option<Cascade>>,
        fetch: Fetch,
    },
    OneToMany {
        entity: String,
        id: String,
        cascades: Vec<Cascade>,
        rt_cascade: Cell<Option<Cascade>>,
        fetch: Fetch,
    },
    ManyToMany {
        entity: String,
        middle: String,
        id: String,
        refer_id: String,
        cascades: Vec<Cascade>,
        rt_cascade: Cell<Option<Cascade>>,
        fetch: Fetch,
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
    pub field_vec: Vec<String>,
    pub field_map: HashMap<String, FieldMeta>, // pub column_map: HashMap<String, FieldMeta>,
}

#[derive(Debug, Default, Clone, RustcDecodable, RustcEncodable)]
pub struct OrmMeta {
    pub entity_vec: Vec<String>,
    pub entity_map: HashMap<String, EntityMeta>, // pub table_map: HashMap<String, EntityMeta>,
}

impl Default for TypeMeta {
    fn default() -> TypeMeta {
        TypeMeta::NULL
    }
}

impl FieldMeta {
    pub fn get_column_name(&self) -> String {
        match self.ty {
            TypeMeta::Id => "id".to_string(),
            TypeMeta::Integer { ref column, .. } => column.to_string(),
            TypeMeta::String { ref column, .. } => column.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_field_name(&self) -> String {
        self.field.to_string()
    }
    pub fn get_type_name(&self) -> String {
        match self.ty {
            TypeMeta::Id => "u64".to_string(),
            TypeMeta::Integer { ref number, .. } => number.to_string(),
            TypeMeta::String { .. } => "String".to_string(),
            TypeMeta::Pointer { ref entity, .. } => entity.to_string(),
            TypeMeta::OneToOne { ref entity, .. } => entity.to_string(),
            TypeMeta::OneToMany { ref entity, .. } => entity.to_string(),
            TypeMeta::ManyToMany { ref entity, .. } => entity.to_string(),
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
        let nullable_fn = |nullable| match nullable {
            true => "",
            false => " NOT NULL",
        };
        match self.ty {
            TypeMeta::Id => "`id` BIGINT PRIMARY KEY NOT NULL AUTO_INCREMENT".to_string(),
            TypeMeta::Integer { ref number, ref column, ref nullable } => {
                format!("`{}` {}{}",
                        column,
                        Self::get_db_type_number(number),
                        nullable_fn(nullable.clone()))
            }
            TypeMeta::String { ref len, ref column, ref nullable } => {
                format!("`{}` VARCHAR({}){}",
                        column,
                        len,
                        nullable_fn(nullable.clone()))
            }
            _ => unreachable!(),
        }
    }
    pub fn get_type_name_set(&self) -> String {
        match self.ty {
            TypeMeta::Id => self.get_type_name(),
            TypeMeta::Integer { .. } => self.get_type_name(),
            TypeMeta::String { .. } => "&str".to_string(),
            TypeMeta::Pointer { ref entity, .. } => format!("&{}", entity),
            TypeMeta::OneToOne { ref entity, .. } => format!("&{}", entity),
            TypeMeta::OneToMany { ref entity, .. } => format!("&{}", entity),
            TypeMeta::ManyToMany { ref entity, .. } => format!("&{}", entity),
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
            TypeMeta::Integer { .. } => true,
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
            TypeMeta::Pointer { ref entity, .. } => entity.to_string(),
            TypeMeta::OneToOne { ref entity, .. } => entity.to_string(),
            TypeMeta::OneToMany { ref entity, .. } => entity.to_string(),
            TypeMeta::ManyToMany { ref entity, .. } => entity.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_cascades(&self) -> &Vec<Cascade> {
        match self.ty {
            TypeMeta::Pointer { ref cascades, .. } => cascades,
            TypeMeta::OneToOne { ref cascades, .. } => cascades,
            TypeMeta::OneToMany { ref cascades, .. } => cascades,
            TypeMeta::ManyToMany { ref cascades, .. } => cascades,
            _ => unreachable!(),
        }
    }
    pub fn get_refer_fetch(&self) -> Fetch {
        match self.ty {
            TypeMeta::Pointer { ref fetch, .. } => fetch.clone(),
            TypeMeta::OneToOne { ref fetch, .. } => fetch.clone(),
            TypeMeta::OneToMany { ref fetch, .. } => fetch.clone(),
            TypeMeta::ManyToMany { ref fetch, .. } => fetch.clone(),
            _ => unreachable!(),
        }
    }
    pub fn get_refer_rt_cascade(&self) -> Option<Cascade> {
        match self.ty {
            TypeMeta::Pointer { ref rt_cascade, .. } => rt_cascade.get(),
            TypeMeta::OneToOne { ref rt_cascade, .. } => rt_cascade.get(),
            TypeMeta::OneToMany { ref rt_cascade, .. } => rt_cascade.get(),
            TypeMeta::ManyToMany { ref rt_cascade, .. } => rt_cascade.get(),
            _ => unreachable!(),
        }
    }
    pub fn set_refer_rt_cascade(&self, value: Option<Cascade>) {
        match self.ty {
            TypeMeta::Pointer { ref rt_cascade, .. } => rt_cascade.set(value),
            TypeMeta::OneToOne { ref rt_cascade, .. } => rt_cascade.set(value),
            TypeMeta::OneToMany { ref rt_cascade, .. } => rt_cascade.set(value),
            TypeMeta::ManyToMany { ref rt_cascade, .. } => rt_cascade.set(value),
            _ => unreachable!(),
        };
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
            TypeMeta::OneToMany { .. } => true,
            _ => false,
        }
    }
    pub fn is_refer_many_many(&self) -> bool {
        match self.ty {
            TypeMeta::ManyToMany { .. } => true,
            _ => false,
        }
    }
    pub fn get_pointer_id(&self) -> String {
        match self.ty {
            TypeMeta::Pointer { ref refer_id, .. } => refer_id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_one_one_id(&self) -> String {
        match self.ty {
            TypeMeta::OneToOne { ref id, .. } => id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_one_many_id(&self) -> String {
        match self.ty {
            TypeMeta::OneToMany { ref id, .. } => id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_many_many_id(&self) -> String {
        match self.ty {
            TypeMeta::ManyToMany { ref id, .. } => id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_many_many_refer_id(&self) -> String {
        match self.ty {
            TypeMeta::ManyToMany { ref refer_id, .. } => refer_id.to_string(),
            _ => unreachable!(),
        }
    }
    pub fn get_many_many_middle_entity(&self) -> String {
        match self.ty {
            TypeMeta::ManyToMany { ref middle, .. } => middle.to_string(),
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
        attr.get("len").map_or(DEFAULT_LEN, |str| u64::from_str(str).unwrap())
    }
    fn pick_cascades(attr: &Attr) -> Vec<Cascade> {
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
    fn pick_fetch(attr: &Attr) -> Fetch {
        attr.get("fetch").map_or(Fetch::Lazy, |str| {
            match str {
                "lazy" => Fetch::Lazy,
                "eager" => Fetch::Eager,
                _ => unreachable!(),
            }
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
            "i32" | "u32" | "i64" | "u64" => Self::new_integer(entity, field, ty, attr),
            "String" => Self::new_string(entity, field, ty, attr),
            _ => unreachable!(),
        }
    }
    fn new_refer(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        if attr.has("pointer") {
            return Self::new_pointer(entity, field, ty, attr);
        } else if attr.has("one_one") {
            return Self::new_one_one(entity, field, ty, attr);
        } else if attr.has("one_many") {
            return Self::new_one_many(entity, field, ty, attr);
        } else if attr.has("many_many") {
            return Self::new_many_many(entity, field, ty, attr);
        }
        unreachable!()
    }

    fn new_string(entity: &str, field: &str, _ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
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
    fn new_integer(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let meta = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::Integer {
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
        // 对象与id都挂在A上
        let mut ret = FieldMeta::new_integer(entity, refer_id_field.as_ref(), "u64", attr);
        let refer_object = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::Pointer {
                refer_id: refer_id_field.to_string(),
                entity: entity_name,
                cascades: Self::pick_cascades(attr),
                rt_cascade: Cell::new(None),
                fetch: Self::pick_fetch(attr),
            },
        };
        ret.push((entity.to_string(), refer_object));
        ret
    }
    fn new_one_one(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        // 对象挂在A上，id挂在B上
        let refer_id_field = format!("{}_id", entity.to_lowercase());
        let refer_entity = ty.to_string();
        let mut ret = FieldMeta::new_integer(&refer_entity, &refer_id_field, "u64", attr);
        let refer_object = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::OneToOne {
                id: refer_id_field.to_string(),
                entity: refer_entity,
                cascades: Self::pick_cascades(attr),
                rt_cascade: Cell::new(None),
                fetch: Self::pick_fetch(attr),
            },
        };
        ret.push((entity.to_string(), refer_object));
        ret
    }
    fn new_one_many(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let re = Regex::new(r"^Vec<(.+)>$").unwrap();
        if !re.is_match(ty) {
            unreachable!();
        }
        let caps = re.captures(ty).unwrap();
        // 对象挂在A上，id挂在B上
        let refer_id_field = format!("{}_id", entity.to_lowercase());
        let refer_entity = caps.get(1).unwrap().as_str().to_string();
        let mut ret = FieldMeta::new_integer(&refer_entity, &refer_id_field, "u64", attr);
        let refer_object = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::OneToMany {
                id: refer_id_field.to_string(),
                entity: refer_entity,
                cascades: Self::pick_cascades(attr),
                rt_cascade: Cell::new(None),
                fetch: Self::pick_fetch(attr),
            },
        };
        ret.push((entity.to_string(), refer_object));
        ret
    }
    fn new_many_many(entity: &str, field: &str, ty: &str, attr: &Attr) -> Vec<(String, FieldMeta)> {
        let re = Regex::new(r"^Vec<(.+)>$").unwrap();
        if !re.is_match(ty) {
            unreachable!();
        }
        let caps = re.captures(ty).unwrap();
        let a = entity.to_string();
        let b = caps.get(1).unwrap().as_str().to_string();
        let a_id = format!("{}_id", a.to_lowercase());
        let b_id = format!("{}_id", b.to_lowercase());
        // 生成中间表
        let middle = format!("{}{}", a, b);
        let a_id_vec = FieldMeta::new_integer(&middle, &a_id, "u64", attr);
        let b_id_vec = FieldMeta::new_integer(&middle, &b_id, "u64", attr);
        let a_b_meta = FieldMeta {
            field: field.to_string(),
            ty: TypeMeta::ManyToMany {
                id: a_id.to_string(),
                refer_id: b_id.to_string(),
                entity: b.to_string(),
                middle: middle.to_string(),
                cascades: Self::pick_cascades(attr),
                rt_cascade: Cell::new(None),
                fetch: Self::pick_fetch(attr),
            },
        };
        let mut ret = vec![(a, a_b_meta)];
        ret.extend(a_id_vec);
        ret.extend(b_id_vec);
        ret
    }
    pub fn format(&self, value: mysql::Value) -> String {
        if value == mysql::Value::NULL {
            return "null".to_string();
        }
        match self.ty {
            TypeMeta::Id => mysql::from_value::<u64>(value).to_string(),
            TypeMeta::Integer { .. } => mysql::from_value::<i64>(value).to_string(),
            TypeMeta::String { .. } => format!("\"{}\"", mysql::from_value::<String>(value)),
            _ => unreachable!(),
        }
    }
}

impl EntityMeta {
    pub fn get_fields(&self) -> Vec<&FieldMeta> {
        self.field_vec.iter().map(|field_name| self.field_map.get(field_name).unwrap()).collect()
    }
    pub fn get_id_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_type_id())
            .collect::<Vec<_>>()
    }
    pub fn get_normal_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_type_normal())
            .collect::<Vec<_>>()
    }
    pub fn get_refer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_type_refer())
            .collect::<Vec<_>>()
    }
    pub fn get_non_refer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| !field.is_type_refer())
            .collect::<Vec<_>>()
    }
    pub fn get_pointer_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_refer_pointer())
            .collect::<Vec<_>>()
    }
    pub fn get_one_one_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_refer_one_one())
            .collect::<Vec<_>>()
    }
    pub fn get_one_many_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_refer_one_many())
            .collect::<Vec<_>>()
    }
    pub fn get_many_many_fields(&self) -> Vec<&FieldMeta> {
        self.get_fields()
            .into_iter()
            .filter(|field| field.is_refer_many_many())
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
            .map(|entity_name| self.entity_map.get(entity_name).unwrap())
            .collect()
    }
}
