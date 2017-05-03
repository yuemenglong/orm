use std::ops::Deref;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::ast::VariantData;
use syntax::print::pprust::*;

use attr::visit_attrs;

use meta::*;

pub fn visit_krate(krate: &syntax::ast::Crate) -> OrmMeta {
    let mut orm_meta = OrmMeta::default();
    let (entities, fields): (Vec<_>, Vec<_>) = krate.module
        .items
        .iter()
        .map(|item| visit_item(item.deref()))
        .unzip();
    let fields = fields.into_iter().flat_map(|item| item).collect::<Vec<_>>();
    // 根据entity聚合field_vec
    orm_meta.entity_vec = entities.iter().map(|entity| entity.entity_name.to_string()).collect();
    for entity in entities {
        orm_meta.entity_map.insert(entity.entity_name.to_string(), entity);
    }
    // 自动生成ManyToMany的中间表
    for &(ref entity, _) in fields.iter() {
        if orm_meta.entity_map.contains_key(entity) {
            continue;
        }
        let mut entity_meta = EntityMeta::default();
        let id_pairs = FieldMeta::new_pkey(&entity);
        entity_meta.field_vec.push("id".to_string());
        for (_, id_field_meta) in id_pairs {
            entity_meta.field_map.insert("id".to_string(), id_field_meta);
        }

        entity_meta.entity_name = entity.to_string();
        entity_meta.table_name = entity.to_string();
        orm_meta.entity_vec.push(entity.to_string());
        orm_meta.entity_map.insert(entity.to_string(), entity_meta);
    }
    for (entity_name, field_meta) in fields.into_iter() {
        let mut entity_meta = orm_meta.entity_map.get_mut(&entity_name).unwrap();
        entity_meta.field_vec.push(field_meta.get_field_name());
        entity_meta.field_map.insert(field_meta.get_field_name(), field_meta);
    }
    orm_meta
}
fn visit_item(item: &syntax::ast::Item) -> (EntityMeta, Vec<(String, FieldMeta)>) {
    match item.node {
        Struct(_, _) => visit_struct(item),
        _ => unreachable!(),
    }
}
fn visit_struct(item: &syntax::ast::Item) -> (EntityMeta, Vec<(String, FieldMeta)>) {
    if let Struct(ref variant_data, ref _generics) = item.node {
        let attr = visit_attrs(&item.attrs);
        let mut entity_meta = EntityMeta::default();
        let entity_name = item.ident.name.as_str().to_string();
        entity_meta.entity_name = entity_name.to_string();
        entity_meta.table_name = attr.get("table")
            .map_or(entity_name.to_string(), |v| v.to_string());
        // entity_meta.table_name = entity_name.to_string();
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
