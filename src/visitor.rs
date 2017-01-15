use std::ops::Deref;
use std::cell::RefMut;

use regex::Regex;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::ast::VariantData;
use syntax::ast::MetaItemKind;
use syntax::ast::NestedMetaItemKind;
use syntax::ast::LitKind;
use syntax::print::pprust::*;

use anno;
use anno::Annotation;

use meta::*;

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
        self.fix_krate();
    }
    fn visit_item(&mut self, item: &syntax::ast::Item) {
        let entity_meta = match item.node {
            Struct(_, _) => Self::visit_struct(item),
            _ => unreachable!(),
        };
        self.meta.entities.push(entity_meta);
    }
    fn visit_struct(item: &syntax::ast::Item) -> EntityMeta {
        if let Struct(ref variant_data, ref _generics) = item.node {
            let mut entity_meta = EntityMeta::default();
            entity_meta.entity_name = item.ident.name.as_str().to_string();
            entity_meta.table_name = item.ident.name.as_str().to_string();
            if let &VariantData::Struct(ref vec, _id) = variant_data {
                entity_meta.fields = vec.iter()
                    .map(Self::visit_struct_field)
                    .collect();
                return entity_meta;
            }
        }
        unreachable!();
    }
    fn visit_struct_field(field: &syntax::ast::StructField) -> FieldMeta {
        let mut field_meta = FieldMeta::default();
        field_meta.field_name = field.ident.as_ref().unwrap().name.as_str().to_string();
        field_meta.column_name = field.ident.as_ref().unwrap().name.as_str().to_string();

        // 检查 id
        if field_meta.field_name == "id" {
            panic!("Id Will Be Added To Entity Automatically");
        }

        // 处理注解
        Self::visit_struct_field_attrs(&mut field_meta, &field.attrs);

        // 处理类型信息
        // 1.raw_ty
        let raw_ty = ty_to_string(field.ty.deref());
        field_meta.raw_ty = raw_ty.clone();
        // 2.ty
        Self::attach_type(&mut field_meta);
        // 3.db_ty
        Self::attach_db_type(&mut field_meta);

        field_meta
    }
    fn visit_struct_field_attrs(field_meta: &mut FieldMeta, attrs: &Vec<syntax::ast::Attribute>) {
        for attr in attrs.iter() {
            match anno::visit_struct_field_attr(attr) {
                Annotation::Len(len) => {
                    field_meta.len = len;
                }
                _ => {}
            }
        }
    }
    fn attach_type(field_meta: &mut FieldMeta) {
        let ty_pattern = Regex::new(r"(^Option<([^<>]+)>$)|(^[^<>]+$)").unwrap();
        let attach = match ty_pattern.captures(&field_meta.raw_ty) {
            Some(captures) => {
                match captures.get(3) {
                    Some(_) => (field_meta.raw_ty.clone(), false),
                    None => (captures.get(2).unwrap().as_str().to_string(), true),
                }
            }
            None => {
                panic!("Unsupport Type: {}", field_meta.raw_ty);
            }
        };
        field_meta.ty = attach.0;
        field_meta.nullable = attach.1;
    }
    fn attach_db_type(field_meta: &mut FieldMeta) {
        let postfix = match field_meta.nullable {
            true => "",
            false => " NOT NULL",
        };
        field_meta.db_ty = match field_meta.ty.as_ref() {
            "i32" => format!("`{}` INTEGER{}", field_meta.column_name, postfix),
            "i64" => format!("`{}` BIGINT{}", field_meta.column_name, postfix),
            "u64" => format!("`{}` BIGINT{}", field_meta.column_name, postfix),
            "String" => {
                format!("`{}` VARCHAR({}){}",
                        field_meta.column_name,
                        field_meta.len,
                        postfix)
            }
            _ => {
                panic!("Unsupported Type: {}", field_meta.ty);
            }
        }
    }
    fn fix_krate(&mut self) {
        for entity_meta in self.meta.entities.iter_mut() {
            // fix pkey
            let pkey_rc = FieldMeta::create_pkey();
            entity_meta.pkey = pkey_rc.clone();
            entity_meta.fields.insert(0, pkey_rc.clone());

            // fix field map / column map
            entity_meta.field_map = entity_meta.fields
                .iter()
                .map(|field_meta_rc| (field_meta_rc.field_name.clone(), field_meta_rc.clone()))
                .collect();
            entity_meta.column_map = entity_meta.fields
                .iter()
                .map(|field_meta_rc| (field_meta_rc.column_name.clone(), field_meta_rc.clone()))
                .collect();
        }
        // fix entity_map / table_map
        self.meta.entity_map = self.meta
            .entities
            .iter()
            .map(|entity_meta_rc| (entity_meta_rc.entity_name.clone(), entity_meta_rc.clone()))
            .collect();
        self.meta.table_map = self.meta
            .entities
            .iter()
            .map(|entity_meta_rc| (entity_meta_rc.table_name.clone(), entity_meta_rc.clone()))
            .collect();
    }
}
