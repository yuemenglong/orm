use std::ops::Deref;
use std::cell::RefMut;
use std::str::FromStr;

use regex::Regex;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::ast::VariantData;
use syntax::ast::MetaItemKind;
use syntax::ast::NestedMetaItemKind;
use syntax::ast::LitKind;
use syntax::print::pprust::*;

use attr::visit_attrs;

use meta::*;

const DEFAULT_LEN: u64 = 64;

#[derive(Debug)]
enum FieldType {
    Normal,
    Refer,
}

pub fn visit_krate(krate: &syntax::ast::Crate) -> OrmMeta {
    let mut meta = OrmMeta::default();
    meta.entities = krate.module
        .items
        .iter()
        .map(|item| visit_item(item.deref()))
        .collect::<Vec<_>>();
    meta
}
fn visit_item(item: &syntax::ast::Item) -> EntityMeta {
    let entity_meta = match item.node {
        Struct(_, _) => visit_struct(item),
        _ => unreachable!(),
    };
    entity_meta
}
fn visit_struct(item: &syntax::ast::Item) -> EntityMeta {
    if let Struct(ref variant_data, ref _generics) = item.node {
        let mut entity_meta = EntityMeta::default();
        entity_meta.entity_name = item.ident.name.as_str().to_string();
        entity_meta.table_name = item.ident.name.as_str().to_string();
        if let &VariantData::Struct(ref vec, _id) = variant_data {
            entity_meta.fields = vec.iter()
                .flat_map(visit_struct_field)
                .collect();
            // 加上pkey
            entity_meta.fields.insert(0, FieldMeta::create_pkey());
            return entity_meta;
        }
    }
    unreachable!();
}
fn visit_struct_field(field: &syntax::ast::StructField) -> Vec<FieldMeta> {
    let field_name = field.ident.as_ref().unwrap().name.as_str().to_string();

    // 检查 id
    if &field_name == "id" {
        panic!("Id Will Be Added To Entity Automatically");
    }

    // 处理注解
    let ty = ty_to_string(field.ty.deref());
    let attr = visit_attrs(&field.attrs);
    if FieldMeta::is_normal_type(&ty) {
        vec![FieldMeta::create_normal(&field_name, &ty, &attr)]
    } else {
        FieldMeta::create_refer(&field_name, &ty, &attr)
    }
}


pub fn fix_meta(meta: &mut OrmMeta) {
    for entity_meta in meta.entities.iter_mut() {
        // build field map / column map
        entity_meta.field_map = entity_meta.fields
            .iter()
            .map(|field_meta_rc| (field_meta_rc.field(), field_meta_rc.clone()))
            .collect();
        // entity_meta.column_map = entity_meta.fields
        //     .iter()
        //     .map(|field_meta_rc| (field_meta_rc.column_name.clone(), field_meta_rc.clone()))
        //     .collect();
    }
    // build entity_map / table_map
    meta.entity_map = meta.entities
        .iter()
        .map(|entity_meta_rc| (entity_meta_rc.entity_name.clone(), entity_meta_rc.clone()))
        .collect();
    // meta.table_map = meta.entities
    //     .iter()
    //     .map(|entity_meta_rc| (entity_meta_rc.table_name.clone(), entity_meta_rc.clone()))
    //     .collect();
}
