use std::ops::Deref;
use std::str::FromStr;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::ast::VariantData;
use syntax::print::pprust::*;

use regex::Regex;

use attr::visit_attrs;
use attr::Attr;

use meta::*;

const DEFAULT_LEN: u64 = 128;

pub fn visit_krate(krate: &syntax::ast::Crate) -> OrmMeta {
    let mut orm_meta = OrmMeta::default();
    for item in krate.module.items.iter() {
        visit_struct(item, &mut orm_meta);
    }
    orm_meta
    // let (entities, fields): (Vec<_>, Vec<_>) = krate.module
    //     .items
    //     .iter()
    //     .map(|item| visit_module(item.deref()))
    //     .unzip();
    // let fields = fields.into_iter().flat_map(|item| item).collect::<Vec<_>>();
    // // 根据entity聚合field_vec
    // orm_meta.entity_vec = entities.iter().map(|entity| entity.entity_name.to_string()).collect();
    // for entity in entities {
    //     orm_meta.entity_map.insert(entity.entity_name.to_string(), entity);
    // }
    // // 自动生成ManyToMany的中间表
    // for &(ref entity, _) in fields.iter() {
    //     if orm_meta.entity_map.contains_key(entity) {
    //         continue;
    //     }
    //     let mut entity_meta = EntityMeta::default();
    //     let id_pairs = FieldMeta::new_pkey(&entity);
    //     entity_meta.field_vec.push("id".to_string());
    //     for (_, id_field_meta) in id_pairs {
    //         entity_meta.field_map.insert("id".to_string(), id_field_meta);
    //     }

    //     entity_meta.entity_name = entity.to_string();
    //     entity_meta.table_name = entity.to_string();
    //     orm_meta.entity_vec.push(entity.to_string());
    //     orm_meta.entity_map.insert(entity.to_string(), entity_meta);
    // }
    // for (entity_name, field_meta) in fields.into_iter() {
    //     let mut entity_meta = orm_meta.entity_map.get_mut(&entity_name).unwrap();
    //     entity_meta.field_vec.push(field_meta.get_field_name());
    //     entity_meta.field_map.insert(field_meta.get_field_name(), field_meta);
    // }
    // orm_meta
}

fn visit_struct(item: &syntax::ast::Item, mut orm_meta: &mut OrmMeta) {
    if let Struct(ref variant_data, ref _generics) = item.node {
        // 1. 先注册这个entity
        let entity_name = item.ident.name.as_str().to_string();
        orm_meta.entity_vec.push(entity_name.to_string());

        // 2. 产生entity_meta
        let attr = visit_attrs(&item.attrs);
        let mut entity_meta = EntityMeta::default();
        entity_meta.entity_name = entity_name.to_string();
        entity_meta.table_name = attr.get("table")
            .map_or(entity_name.to_string(), |v| v.to_string());
        // entity_meta.table_name = entity_name.to_string();

        if let &VariantData::Struct(ref vec, _id) = variant_data {
            // 首先生成pkey
            entity_meta.field_vec.push("id".to_string());
            entity_meta.field_map.insert("id".to_string(), FieldMeta::Id);

            for field in vec.iter() {
                visit_struct_field(field, &mut entity_meta, &mut orm_meta);
            }
        }
        orm_meta.entity_map.insert(entity_name.to_string(), entity_meta);
        return;
    }
    unreachable!();
}
fn visit_struct_field(field: &syntax::ast::StructField,
                      mut entity_meta: &mut EntityMeta,
                      mut orm_meta: &mut OrmMeta) {
    let field_name = field.ident.as_ref().unwrap().name.as_str().to_string();
    let ty = ty_to_string(field.ty.deref());
    let attr = visit_attrs(&field.attrs);
    entity_meta.field_vec.push(field_name.to_string());

    // 检查 id
    if &field_name == "id" {
        panic!("Id Will Be Added To Entity Automatically");
    }
    // FieldMeta::new(&entity, &field_name, &ty, &attr)
    match ty.as_ref() {
        "i32" | "i64" | "u32" | "u64" => {
            let column_name = &field_name;
            let nullable = pick_nullable(&attr);
            let field_meta = FieldMeta::new_integer(&field_name, column_name, &ty, nullable);
            entity_meta.field_map.insert(field_name.to_string(), field_meta);
            return;
        }
        "String" => {
            let column_name = &field_name;
            let nullable = pick_nullable(&attr);
            let len = pick_len(&attr);
            let field_meta = FieldMeta::new_string(&field_name, column_name, len, nullable);
            entity_meta.field_map.insert(field_name.to_string(), field_meta);
            return;
        }
        _ => {}
    };
    let cascades = pick_cascades(&attr);
    let fetch = pick_fetch(&attr);
    if attr.has("refer"){
        let values = attr.get_values("refer");
        if values.len() != 2{
            panic!("Refer Must Has Left And Right Field");
        }
        let left = values[0];
        let right = values[1];
        let field_meta = FieldMeta::new_refer(&field_name, ty.as_ref(), left, right, cascades, fetch);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
    }
    //     let fetch = pick_fetch(&attr);
    // let cascades = pick_cascades(&attr);
    // let (left, right) = pick_refer(&attr);
    // let re = Regex::new(r"^Vec<(.+)>$").unwrap();
    // if !re.is_match(ty) {
    //     let entity = ty.to_string();
    //     let field_meta = FieldMeta::new_one_one(&field_name,
    //                                             &entity,
    //                                             &left,
    //                                             &right,
    //                                             cascades,
    //                                             fetch);
    // } else {
    //     let field_meta = FieldMeta::new_one_many(&field_name,
    //                                              &column_name,
    //                                              &left,
    //                                              &right,
    //                                              cascades,
    //                                              fetch);
    // }
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
fn pick_refer(attr: &Attr) -> (String, String) {
    let values = attr.get_values("refer");
    if values.len() != 2 {
        panic!("Refer Must Define Left And Right Field");
    }
    (values[0].to_string(), values[1].to_string())
}
