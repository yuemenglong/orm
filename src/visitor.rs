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

use anno;
use anno::Annotation;


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
        let entity_meta = self.meta.cur_entity();
        let mut entity_meta = entity_meta.borrow_mut();
        let field_meta = entity_meta.new_field();
        let mut field_meta = field_meta.borrow_mut();
        field_meta.field_name = field.ident.as_ref().unwrap().name.as_str().to_string();
        // 0.annos & pkey
        for attr in field.attrs.iter() {
            // self.visit_struct_field_attr(attr);
            let annotation = anno::visit_struct_field_attr(attr);
            match &annotation {
                &Annotation::Id(auto) => {
                    field_meta.pkey = true;
                }
                _ => {}
            }
            field_meta.annos.push(annotation);
        }
        // 处理类型信息
        // 1.raw_ty
        let raw_ty = ty_to_string(field.ty.deref());
        field_meta.raw_ty = raw_ty.clone();
        // 2.ty
        Self::attach_type(&mut field_meta);
        // 3.db_ty
        Self::attach_db_type(&mut field_meta);
    }

    fn fix_krate(&mut self) {
        for entity_meta_rc in self.meta.entities.iter() {
            // fix table_name
            let mut entity_meta = entity_meta_rc.borrow_mut();
            if entity_meta.table_name.len() == 0 {
                entity_meta.table_name = entity_meta.entity_name.clone();
            }
            let mut pkey = None;
            for field_meta_rc in entity_meta.fields.iter() {
                // fix column_name
                let mut field_meta = field_meta_rc.borrow_mut();
                if field_meta.column_name.len() == 0 {
                    field_meta.column_name = field_meta.field_name.clone();
                }
                // fix pkey
                if field_meta.pkey {
                    pkey = Some(field_meta_rc.clone());
                }
            }
            match pkey {
                Some(field_meta_rc) => entity_meta.pkey = field_meta_rc.clone(),
                None => panic!("Entity `{}` Has No Pkey", entity_meta.entity_name),
            }
            // fix field map / column map
            entity_meta.field_map = entity_meta.fields
                .iter()
                .map(|field_meta_rc| {
                    (field_meta_rc.borrow().field_name.clone(), field_meta_rc.clone())
                })
                .collect();
            entity_meta.column_map = entity_meta.fields
                .iter()
                .map(|field_meta_rc| {
                    (field_meta_rc.borrow().column_name.clone(), field_meta_rc.clone())
                })
                .collect();
        }
        // fix entity_map / table_map
        self.meta.entity_map = self.meta
            .entities
            .iter()
            .map(|entity_meta_rc| {
                (entity_meta_rc.borrow().entity_name.clone(), entity_meta_rc.clone())
            })
            .collect();
        self.meta.table_map = self.meta
            .entities
            .iter()
            .map(|entity_meta_rc| {
                (entity_meta_rc.borrow().table_name.clone(), entity_meta_rc.clone())
            })
            .collect();
    }
}
