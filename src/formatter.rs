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
use ast::Entity;
use std;

static mut META: Option<&'static ast::meta::OrmMeta> = None;
static ONCE: std::sync::Once = std::sync::ONCE_INIT;

pub fn meta() -> &'static ast::meta::OrmMeta {
    let json = r#${JSON}#;
    ONCE.call_once(|| unsafe { META = Some(ast::init::init_meta(json)) });
    unsafe { META.unwrap() }
}

${ENTITIES}
"#;

static TPL_STRUCT: &'static str = r#"
#[derive(Debug, Clone)]
pub struct ${ENTITY_NAME} {
    inner: ast::EntityInnerPointer,
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
        self.inner_get("${FIELD}").unwrap()
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set("${FIELD}", Some(value));
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.inner_has("${FIELD}")
    }"#;

static TPL_IMPL_POINTER: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> Box<${TYPE}> {
        Box::new(self.inner_get_pointer("${FIELD}"))
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set_pointer("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.inner_has_pointer("${FIELD}")
    }   
    #[allow(dead_code)]
    pub fn clear_${FIELD}(&self) {
        self.inner_clear_pointer("${FIELD}");
    }"#;

static TPL_IMPL_ONE_ONE: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> Box<${TYPE}> {
        Box::new(self.inner_get_one_one("${FIELD}"))
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set_one_one("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.inner_has_one_one("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn clear_${FIELD}(&self) {
        self.inner_clear_one_one("${FIELD}")
    }"#;

static TPL_TRAIT: &'static str = r#"
impl ast::Entity for ${ENTITY_NAME} {
    fn meta() -> &'static ast::meta::EntityMeta {
        meta().entity_map.get("${ENTITY_NAME}").unwrap()
    }
    fn default() -> Self {
        ${ENTITY_NAME} {
            inner: std::rc::Rc::new(std::cell::RefCell::new(ast::EntityInner::new(Self::meta())))
        }
    }
    fn new(inner: ast::EntityInnerPointer) -> ${ENTITY_NAME} {
        ${ENTITY_NAME} {
            inner: inner,
        }
    }
    fn inner(&self) -> ast::EntityInnerPointer {
        self.inner.clone()
    }
}
"#;

fn do_id_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.get_id_fields()
        .into_iter()
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

fn do_spec_fields(fields: Vec<&FieldMeta>, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    fields.into_iter()
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

fn do_normal_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.get_normal_fields()
        .into_iter()
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

fn do_pointer_fields(meta: &EntityMeta, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    meta.get_pointer_fields()
        .into_iter()
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

pub fn format_meta(meta: &OrmMeta) -> String {
    let json = format!("\"{}\"", rustc_serialize::json::encode(&meta).unwrap());
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
    // let id_fields = do_id_fields(meta, "", &format_entity_define_field);
    // let normal_fields = do_normal_fields(meta, "", &format_entity_define_field);
    // let refer_fields = do_pointer_fields(meta, "", &format_entity_define_field);
    // let fields = format!("{}{}\n{}", id_fields, normal_fields, refer_fields);
    TPL_STRUCT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
}
fn format_entity_impl(meta: &EntityMeta) -> String {
    let normal_fields = do_normal_fields(meta, "", &format_entity_impl_field);
    let pointer_fields = do_pointer_fields(meta, "", &format_entity_impl_pointer);
    let one_one_fields = do_spec_fields(meta.get_one_one_fields(), "", &format_entity_impl_one_one);
    let fields = format!("{}\n{}\n{}", normal_fields, pointer_fields, one_one_fields);
    TPL_IMPL.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${IMPL_FIELDS}", &fields)
}
fn format_entity_trait_get_value(meta: &FieldMeta) -> String {
    format!("ast::Value::from(&self.{})", meta.field())
}
fn format_entity_trait(meta: &EntityMeta) -> String {
    TPL_TRAIT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
}
fn format_entity_define_field(meta: &FieldMeta) -> String {
    TPL_STRUCT_FIELD.to_string()
        .replace("${FIELD}", &meta.field())
        .replace("${TYPE}", &meta.type_name())
}
fn format_entity_impl_field(meta: &FieldMeta) -> String {
    TPL_IMPL_FIELD.to_string()
        .replace("${FIELD}", &meta.field())
        .replace("${TYPE}", &meta.type_name())
        .replace("${SET_TYPE}", &meta.type_name_set())
}
fn format_entity_impl_pointer(meta: &FieldMeta) -> String {
    TPL_IMPL_POINTER.to_string()
        .replace("${FIELD}", &meta.field())
        .replace("${TYPE}", &meta.type_name())
        .replace("${SET_TYPE}", &meta.type_name_set())
}
fn format_entity_impl_one_one(meta: &FieldMeta) -> String {
    TPL_IMPL_ONE_ONE.to_string()
        .replace("${FIELD}", &meta.field())
        .replace("${TYPE}", &meta.type_name())
        .replace("${SET_TYPE}", &meta.type_name_set())
}
