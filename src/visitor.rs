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
    // 补全refer相关的字段
    let clone = orm_meta.clone();
    for (entity_name, entity_meta) in clone.entity_map.into_iter() {
        for (field_name, field_meta) in entity_meta.field_map.into_iter() {
            if !field_meta.is_type_refer() {
                continue;
            }
            let refer_entity = field_meta.get_refer_entity();
            let (left, right) = field_meta.get_refer_lr();
            {
                let mut left_entity_meta = orm_meta.entity_map.get_mut(&entity_name).unwrap();
                if left_entity_meta.field_map.get(&left).is_none() {
                    let left_meta = FieldMeta::new_refer_id(&left, &left);
                    left_entity_meta.field_vec.push(left.clone());
                    left_entity_meta.field_map.insert(left, left_meta);
                }
            }
            {
                let mut right_entity_meta = orm_meta.entity_map.get_mut(&refer_entity).unwrap();
                if right_entity_meta.field_map.get(&right).is_none() {
                    let right_meta = FieldMeta::new_refer_id(&right, &right);
                    right_entity_meta.field_vec.push(right.clone());
                    right_entity_meta.field_map.insert(right, right_meta);
                }
            }
        }
    }
    orm_meta
}

fn visit_struct(item: &syntax::ast::Item, mut orm_meta: &mut OrmMeta) {
    if let Struct(ref variant_data, ref _generics) = item.node {
        // 1. 先注册这个entity
        let entity = item.ident.name.as_str().to_string();
        orm_meta.entity_vec.push(entity.to_string());

        // 2. 产生entity_meta
        let attr = visit_attrs(&item.attrs);
        let mut entity_meta = EntityMeta::default();
        entity_meta.entity = entity.to_string();
        entity_meta.table = attr.get("table").map_or(entity.to_string(), |v| v.to_string());
        entity_meta.alias = attr.get("alias").map_or(entity.to_lowercase(), |v| v.to_string());

        if let &VariantData::Struct(ref vec, _id) = variant_data {
            for field in vec.iter() {
                visit_struct_field(field, &mut entity_meta, &mut orm_meta);
            }
            if entity_meta.field_map.get("id").is_none() {
                // 没有配置的话，默认自动生成auto id
                entity_meta.field_vec.insert(0, "id".to_string());
                entity_meta.field_map.insert("id".to_string(), FieldMeta::new_pkey(true));
            }
        }
        orm_meta.entity_map.insert(entity.to_string(), entity_meta);
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
        let auto = pick_auto(&attr);
        let field_meta = FieldMeta::new_pkey(auto);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
        return;
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
    if attr.has("refer") {
        let values = attr.get_values("refer");
        if values.len() != 2 {
            panic!("Refer Must Has Left And Right Field");
        }
        let left = values[0];
        let right = values[1];
        let field_meta =
            FieldMeta::new_refer(&field_name, ty.as_ref(), left, right, cascades, fetch);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
    } else if attr.has("pointer") {
        let values = attr.get_values("pointer");
        let (left, right) = match values.len() {
            0 => (format!("{}_id", &field_name), "id".to_string()),
            1 => (values[0].to_string(), "id".to_string()),
            2 => (values[0].to_string(), values[1].to_string()),
            _ => panic!("Pointer Must Has Less Than 2 Anno"),
        };
        let field_meta =
            FieldMeta::new_pointer(&field_name, ty.as_ref(), &left, &right, cascades, fetch);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
    } else if attr.has("one_one") {
        let values = attr.get_values("one_one");
        let (left, right) = match values.len() {
            0 => ("id".to_string(), format!("{}_id", &entity_meta.alias)),
            1 => ("id".to_string(), values[0].to_string()),
            2 => (values[0].to_string(), values[1].to_string()),
            _ => panic!("OneToOne Must Has Less Than 2 Anno"),
        };
        let field_meta =
            FieldMeta::new_one_one(&field_name, ty.as_ref(), &left, &right, cascades, fetch);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
    } else if attr.has("one_many") {
        let values = attr.get_values("one_many");
        let (left, right) = match values.len() {
            0 => ("id".to_string(), format!("{}_id", &entity_meta.alias)),
            1 => ("id".to_string(), values[0].to_string()),
            2 => (values[0].to_string(), values[1].to_string()),
            _ => panic!("OneToMany Must Has Less Than 2 Anno"),
        };
        let field_meta =
            FieldMeta::new_one_many(&field_name, ty.as_ref(), &left, &right, cascades, fetch);
        entity_meta.field_map.insert(field_name.to_string(), field_meta);
    }
}

fn pick_nullable(attr: &Attr) -> bool {
    let default = true;
    attr.get("nullable").map_or(default, |str| bool::from_str(str).unwrap())
}
fn pick_auto(attr: &Attr) -> bool {
    let default = false;
    attr.get("auto").map_or(default, |str| bool::from_str(str).unwrap())
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
