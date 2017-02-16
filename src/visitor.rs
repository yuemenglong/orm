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
                .map(visit_struct_field)
                .collect();
            // 为引用类型加上id
            let refer_id_vec = entity_meta.get_refer_fields()
                .into_iter()
                .map(FieldMeta::create_pointer_id)
                .collect::<Vec<_>>();
            entity_meta.fields.extend(refer_id_vec);
            // 加上pkey
            entity_meta.fields.insert(0, FieldMeta::create_pkey());
            return entity_meta;
        }
    }
    unreachable!();
}
fn visit_struct_field(field: &syntax::ast::StructField) -> FieldMeta {
    let field_name = field.ident.as_ref().unwrap().name.as_str().to_string();

    // 检查 id
    if &field_name == "id" {
        panic!("Id Will Be Added To Entity Automatically");
    }

    // 处理注解
    let (nullable, len, pointer) = visit_struct_field_attrs(&field.attrs);
    let ty = ty_to_string(field.ty.deref());
    match (ty.as_ref(), pointer) {
        // 引用类型
        (_, true) => FieldMeta::create_pointer(&field_name, &ty, nullable),
        // String类型
        ("String", false) => FieldMeta::create_string(&field_name, len, nullable),
        // 数字类型
        (_, false) => FieldMeta::create_number(&field_name, &ty, nullable),
    }
}
//(nullable, len, pointer)
fn visit_struct_field_attrs(attrs: &Vec<syntax::ast::Attribute>) -> (bool, u64, bool) {
    let mut nullable = true;
    let mut len = 64;
    let mut pointer = false;
    for attr in attrs.iter() {
        match anno::visit_struct_field_attr(attr) {
            Annotation::Len(l) => {
                len = l;
            }
            Annotation::Nullable(b) => {
                nullable = b;
            }
            Annotation::Pointer => {
                pointer = true;
            }
            _ => {}
        }
    }
    (nullable, len, pointer)
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
