use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;
use std::cell::RefMut;

use regex::Regex;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::ast::VariantData;
use syntax::ast::MetaItemKind;
use syntax::ast::NestedMetaItemKind;
use syntax::ast::LitKind;
use syntax::print::pprust::*;

#[derive(Debug, Default)]
pub struct FieldMeta {
    pub field_name: String,
    pub column_name: String,
    pub ty: String,
    pub len: usize,
    pub db_ty: String,
    pub raw_ty: String,
    pub nullable: bool,
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

#[derive(Debug)]
pub struct Visitor {
    pub meta: OrmMeta,
}


impl Visitor {
    pub fn new() -> Visitor {
        Visitor { meta: OrmMeta::default() }
    }

    pub fn visit_krate(&mut self, krate: &syntax::ast::Crate) {
        for item in krate.module.items.iter() {
            self.visit_item(item.deref());
        }
    }

    fn visit_item(&mut self, item: &syntax::ast::Item) {
        match item.node {
            Struct(_, _) => self.visit_struct(item),
            _ => unreachable!(),
        }
    }

    fn visit_struct(&mut self, item: &syntax::ast::Item) {
        if let Struct(ref variant_data, ref _generics) = item.node {
            let entity_meta = self.meta.new_entity();
            entity_meta.borrow_mut().entity_name = item.ident.name.as_str().to_string();
            if let &VariantData::Struct(ref vec, _id) = variant_data {
                for field in vec {
                    self.visit_struct_field(field);
                }
            } else {
                unreachable!();
            }
        } else {
            unreachable!();
        }
    }

    fn visit_struct_field(&mut self, field: &syntax::ast::StructField) {
        {
            let entity_meta = self.meta.cur_entity();
            let mut entity_meta = entity_meta.borrow_mut();
            let field_meta = entity_meta.new_field();
            let mut field_meta = field_meta.borrow_mut();
            field_meta.field_name = field.ident.as_ref().unwrap().name.as_str().to_string();
            // 处理类型信息
            // 1.raw_ty
            let raw_ty = ty_to_string(field.ty.deref());
            field_meta.raw_ty = raw_ty.clone();
            // 2.ty
            Self::attach_type(&mut field_meta);
            // 3.db_ty
            Self::attach_db_type(&mut field_meta);
        }
        for attr in field.attrs.iter() {
            self.visit_struct_field_attr(attr);
        }
    }

    fn visit_struct_field_attr(&mut self, attr: &syntax::ast::Attribute) {
        self.visit_meta_item(&attr.value);
    }

    fn visit_meta_item(&mut self, item: &syntax::ast::MetaItem) {
        println!("MetaItem Name: {:?}", item.name);
        match item.node {
            MetaItemKind::Word => {
                println!("MetaItemKind::Word");
            }
            MetaItemKind::List(ref vec) => {
                println!("MetaItemKind::List");
                for item in vec {
                    // println!("Item {:?}", item);
                    self.visit_nest_meta_item(&item);
                }
            }
            MetaItemKind::NameValue(ref lit) => {
                println!("MetaItemKind::NameValue");
                self.visit_lit_meta_item(lit);
            }
        }
    }

    fn visit_nest_meta_item(&mut self, item: &syntax::ast::NestedMetaItem) {
        match item.node {
            NestedMetaItemKind::MetaItem(ref item) => {
                self.visit_meta_item(&item);
            }
            _ => {}

        }
    }

    fn visit_lit_meta_item(&mut self, lit: &syntax::ast::Lit) {
        match lit.node {
            LitKind::Str(ref symbol, ref _str_style) => {
                println!("Lit Value: {:?}", symbol.as_str());
            }
            _ => {}
        }
    }
    fn attach_type(field_meta: &mut RefMut<FieldMeta>) {
        let ty_pattern = Regex::new(r"(^Option<([^<>]+)>$)|(^[^<>]+$)").unwrap();
        match ty_pattern.captures(&field_meta.raw_ty) {
            Some(captures) => {
                match captures.get(3) {
                    Some(_) => {
                        field_meta.ty = field_meta.raw_ty.clone();
                        field_meta.nullable = false;
                    }
                    None => {
                        field_meta.ty = captures.get(2).unwrap().as_str().to_string();
                        field_meta.nullable = true;
                    }
                }
            }
            None => {
                panic!("Unsupport Type: {}", field_meta.raw_ty);
            }
        }
    }
    fn attach_db_type(field_meta: &mut RefMut<FieldMeta>) {
        let postfix = match field_meta.nullable {
            true => "",
            false => " NOT NULL",
        };
        field_meta.db_ty = match field_meta.ty.as_ref() {
            "i32" => format!("INTEGER{}", postfix),
            "i64" => format!("BIGINT{}", postfix),
            "String" => format!("VARCHAR({}){}", field_meta.len, postfix),
            _ => {
                panic!("Unsupported Type: {}", field_meta.ty);
            }
        }
    }
}
