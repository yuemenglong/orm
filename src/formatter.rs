use std;
use std::cell::Cell;
use std::ops::Deref;

use syntax;
use syntax::ast::ItemKind;
use syntax::print::pprust::*;
use syntax::ast::VariantData;
use rustc_serialize;

use meta::*;

static TPL: &'static str = r#"
use ast;
use std::sync::Once;
use std::sync::ONCE_INIT;

static mut META: Option<&'static ast::meta::OrmMeta> = None;
static ONCE: Once = ONCE_INIT;

fn get_meta() -> &'static ast::meta::OrmMeta {
    let json = r#${JSON}#;
    ONCE.call_once(|| unsafe { META = Some(ast::init::init_meta(json)) });
    unsafe { META.unwrap() }
}

${ENTITIES}
"#;

static TPL_STRUCT: &'static str = r#"
#[derive(Debug, Clone, Default)]
pub struct ${ENTITY_NAME} {${STRUCT_FIELDS}
}
"#;

static TPL_STRUCT_FIELD: &'static str = r#"
    pub ${FIELD}: Option<${TYPE}>, "#;


static TPL_IMPL: &'static str = r#"
impl ${ENTITY_NAME} {${IMPL_FIELDS}
}
"#;

static TPL_IMPL_FIELD: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> ${TYPE} {
        self.${FIELD}.clone().unwrap()
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${TYPE}) {
        self.${FIELD} = Some(value);
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.${FIELD}.is_some()
    }"#;

static TPL_TRAIT_SET_VALUE: &'static str = r#"
        self.${FIELD} = row.get::<Option<${TYPE}>, &str>(format!("{}${COLUMN}", prefix).as_ref()).unwrap();"#;

static TPL_TRAIT: &'static str = r#"
impl ast::Entity for ${ENTITY_NAME} {
    fn get_meta() -> &'static ast::meta::EntityMeta {
        get_meta().entity_map.get("${ENTITY_NAME}").unwrap()
    }
    fn get_values(&self) -> Vec<ast::Value> {
        vec![${VALUES}]
    }
    fn set_values(&mut self, row: &mut ast::Row, prefix: &str) {${SET_VALUES}
    }
    fn get_id(&self) -> u64 {
        self.id.unwrap()
    }
    fn set_id(&mut self, value: u64) {
        self.id = Some(value);
    }
    fn has_id(&self) -> bool {
        self.id.is_some()
    }
}
"#;

fn do_id_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.fields
        .iter()
        .filter(|field| field.pkey)
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

fn do_normal_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.fields
        .iter()
        .filter(|field| !field.pkey && !field.refer)
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

fn do_refer_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.fields
        .iter()
        .filter(|field| field.refer)
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

pub fn format_meta(meta: &OrmMeta) -> String {
    let json = format!("\"{}\"", rustc_serialize::json::encode(&meta).unwrap());
    // println!("{}", json);
    let entities = meta.entities
        .iter()
        .map(format_entity)
        .collect::<Vec<_>>()
        .join("");
    let mut tpl = TPL.to_string();
    tpl.replace("${JSON}", &json).replace("${ENTITIES}", &entities)
}
fn format_entity(meta: &EntityMeta) -> String {
    let entity = format_entity_define(meta);
    let implt = format_entity_impl(meta);
    let treit = format_entity_trait(meta);
    format!("{}{}{}", entity, implt, treit)
}
fn format_entity_define(meta: &EntityMeta) -> String {
    let id_fields = do_id_fields(meta, "", &format_entity_field);
    let normal_fields = do_normal_fields(meta, "", &format_entity_field);
    let refer_fields = do_refer_fields(meta, "", &format_entity_field);
    let fields = format!("{}{}\n{}", id_fields, normal_fields, refer_fields);
    TPL_STRUCT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${STRUCT_FIELDS}", &fields)
}
fn format_entity_impl(meta: &EntityMeta) -> String {
    let normal_fields = do_normal_fields(meta, "", &format_entity_field_impl);
    let refer_fields = do_refer_fields(meta, "", &format_entity_field_impl);
    let fields = format!("{}\n{}", normal_fields, refer_fields);
    TPL_IMPL.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${IMPL_FIELDS}", &fields)
}
fn format_entity_trait_get_value(meta: &FieldMeta) -> String {
    format!("ast::Value::from(&self.{})", meta.field_name)
}
fn format_entity_trait_set_value(meta: &FieldMeta) -> String {
    TPL_TRAIT_SET_VALUE.to_string()
        .replace("${TYPE}", &meta.ty)
        .replace("${FIELD}", &meta.field_name)
        .replace("${COLUMN}", &meta.column_name)
}
fn format_entity_trait(meta: &EntityMeta) -> String {
    let values = do_normal_fields(meta, ", ", &format_entity_trait_get_value);
    let set_id_values = do_id_fields(meta, "", &format_entity_trait_set_value);
    let set_normal_values = do_normal_fields(meta, "", &format_entity_trait_set_value);
    let set_values = format!("{}{}", set_id_values, set_normal_values);
    TPL_TRAIT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${VALUES}", &values)
        .replace("${SET_VALUES}", &set_values)
}
fn format_entity_field(meta: &FieldMeta) -> String {
    TPL_STRUCT_FIELD.to_string().replace("${FIELD}", &meta.field_name).replace("${TYPE}", &meta.ty)
}
fn format_entity_field_impl(meta: &FieldMeta) -> String {
    TPL_IMPL_FIELD.to_string()
        .replace("${FIELD}", &meta.field_name)
        .replace("${TYPE}", &meta.ty)
}
