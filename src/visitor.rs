use std::ops::Deref;
use std::collections::HashMap;
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
    let (entities, fields): (Vec<_>, Vec<_>) = krate.module
        .items
        .iter()
        .map(|item| visit_item(item.deref()))
        .unzip();
    // 根据entity聚合fields
    let mut map: HashMap<String, Vec<FieldMeta>> = HashMap::new();
    fields.into_iter().flat_map(|vec| vec).fold(&mut map, |mut acc, (entity, field)| {
        if !acc.contains_key(&entity) {
            acc.insert(entity.to_string(), Vec::new());
        }
        acc.get_mut(&entity).unwrap().push(field);
        acc
    });
    meta.entities = entities.into_iter()
        .map(|mut entity| {
            entity.fields = map.remove(&entity.entity_name).unwrap();
            entity
        })
        .collect();
    meta
}
fn visit_item(item: &syntax::ast::Item) -> (EntityMeta, Vec<(String, FieldMeta)>) {
    match item.node {
        Struct(_, _) => visit_struct(item),
        _ => unreachable!(),
    }
}
fn visit_struct(item: &syntax::ast::Item) -> (EntityMeta, Vec<(String, FieldMeta)>) {
    if let Struct(ref variant_data, ref _generics) = item.node {
        let mut entity_meta = EntityMeta::default();
        let entity_name = item.ident.name.as_str().to_string();
        entity_meta.entity_name = entity_name.to_string();
        entity_meta.table_name = entity_name.to_string();
        if let &VariantData::Struct(ref vec, _id) = variant_data {
            // 加上pkey
            let mut ret = FieldMeta::new_pkey(&entity_name);
            let fields: Vec<_> = vec.iter()
                .flat_map(|field| visit_struct_field(&entity_name, field))
                .collect();
            ret.extend(fields);
            return (entity_meta, ret);
        }
    }
    unreachable!();
}
fn visit_struct_field(entity: &str, field: &syntax::ast::StructField) -> Vec<(String, FieldMeta)> {
    let field_name = field.ident.as_ref().unwrap().name.as_str().to_string();

    // 检查 id
    if &field_name == "id" {
        panic!("Id Will Be Added To Entity Automatically");
    }

    // 处理注解
    let ty = ty_to_string(field.ty.deref());
    let attr = visit_attrs(&field.attrs);
    FieldMeta::new(&entity, &field_name, &ty, &attr)
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
