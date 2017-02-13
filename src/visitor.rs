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
            let refer_id_vec = entity_meta.fields
                .iter()
                .filter(|field| field.refer)
                .map(FieldMeta::create_refer_id)
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
    let mut field_meta = FieldMeta::default();
    field_meta.nullable = true;
    field_meta.field_name = field.ident.as_ref().unwrap().name.as_str().to_string();
    field_meta.column_name = field.ident.as_ref().unwrap().name.as_str().to_string();

    // 检查 id
    if field_meta.field_name == "id" {
        panic!("Id Will Be Added To Entity Automatically");
    }

    // 处理注解
    match visit_struct_field_attrs(&mut field_meta, &field.attrs) {
        FieldType::Normal => {
            // 处理类型信息
            // 1.ty
            let ty = ty_to_string(field.ty.deref());
            field_meta.ty = ty.clone();
            // 2.len
            attach_len(&mut field_meta);
            // 3.db_ty
            attach_db_type(&mut field_meta);
            println!("{:?}", field_meta);

            field_meta
        }
        FieldType::Refer => {
            // 处理引用类型信息
            let ty = ty_to_string(field.ty.deref());
            let field = &field_meta.field_name;
            FieldMeta::create_refer(field, &ty)
        }
    }
}
fn visit_struct_field_attrs(field_meta: &mut FieldMeta,
                            attrs: &Vec<syntax::ast::Attribute>)
                            -> FieldType {
    let mut ret = FieldType::Normal;
    for attr in attrs.iter() {
        match anno::visit_struct_field_attr(attr) {
            Annotation::Len(len) => {
                field_meta.len = len;
            }
            Annotation::Nullable(b) => {
                field_meta.nullable = b;
            }
            Annotation::Pointer => {
                ret = FieldType::Refer;
            }
            _ => {}
        }
    }
    ret
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

fn attach_len(field_meta: &mut FieldMeta) {
    match (field_meta.ty.as_ref(), field_meta.len) {
        ("String", 0) => field_meta.len = DEFAULT_LEN,
        (_, _) => {}
    }
}

pub fn fix_meta(meta: &mut OrmMeta) {
    for entity_meta in meta.entities.iter_mut() {
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
    meta.entity_map = meta.entities
        .iter()
        .map(|entity_meta_rc| (entity_meta_rc.entity_name.clone(), entity_meta_rc.clone()))
        .collect();
    meta.table_map = meta.entities
        .iter()
        .map(|entity_meta_rc| (entity_meta_rc.table_name.clone(), entity_meta_rc.clone()))
        .collect();
}
