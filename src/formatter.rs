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

static mut ORM_META: Option<&'static ast::meta::OrmMeta> = None;
static ONCE: std::sync::Once = std::sync::ONCE_INIT;

pub fn orm_meta() -> &'static ast::meta::OrmMeta {
    let json = r#${JSON}#;
    ONCE.call_once(|| unsafe { ORM_META = Some(ast::init::init_meta(json)) });
    unsafe { ORM_META.unwrap() }
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
        self.inner_get("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.inner_has::<${TYPE}>("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn clear_${FIELD}(&self) {
        self.inner_clear::<${TYPE}>("${FIELD}");
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

static TPL_IMPL_ONE_MANY: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> Box<Vec<${TYPE}>> {
        Box::new(self.inner_get_one_many("${FIELD}"))
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: Vec<${TYPE}>) {
        self.inner_set_one_many("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn has_${FIELD}(&self) -> bool {
        self.inner_has_one_many("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn clear_${FIELD}(&self) {
        self.inner_clear_one_many("${FIELD}")
    }"#;

static TPL_IMPL_CASCADE: &'static str = r#"
    #[allow(dead_code)]
    pub fn cascade_${FIELD}_insert(&self) {
        self.inner_cascade_field_insert("${FIELD}");
    }
    #[allow(dead_code)]
    pub fn cascade_${FIELD}_update(&self) {
        self.inner_cascade_field_update("${FIELD}");
    }
    #[allow(dead_code)]
    pub fn cascade_${FIELD}_delete(&self) {
        self.inner_cascade_field_delete("${FIELD}");
    }
    #[allow(dead_code)]
    pub fn cascade_${FIELD}_null(&self) {
        self.inner_cascade_field_null("${FIELD}");
    }"#;

static TPL_TRAIT: &'static str = r#"
impl ast::Entity for ${ENTITY_NAME} {
    fn meta() -> &'static ast::meta::EntityMeta {
        orm_meta().entity_map.get("${ENTITY_NAME}").unwrap()
    }
    fn default() -> Self {
        ${ENTITY_NAME} {
            inner: std::rc::Rc::new(std::cell::RefCell::new(ast::EntityInner::default(Self::meta())))
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

fn do_spec_fields(fields: Vec<&FieldMeta>, join: &str, cb: &Fn(&FieldMeta) -> String) -> String {
    fields.into_iter()
        .map(cb)
        .collect::<Vec<_>>()
        .join(join)
}

pub fn format_meta(meta: &OrmMeta) -> String {
    let json = format!("\"{}\"", rustc_serialize::json::encode(&meta).unwrap());
    let entities = meta.get_entities()
        .into_iter()
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
    TPL_STRUCT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
}
fn format_entity_impl(meta: &EntityMeta) -> String {
    let normal_fields = do_spec_fields(meta.get_normal_fields(), "", &format_entity_impl_field);
    let pointer_fields = do_spec_fields(meta.get_pointer_fields(), "", &format_entity_impl_pointer);
    let one_one_fields = do_spec_fields(meta.get_one_one_fields(), "", &format_entity_impl_one_one);
    let one_many_fields =
        do_spec_fields(meta.get_one_many_fields(), "", &format_entity_impl_one_many);
    let cascade_detail = do_spec_fields(meta.get_refer_fields(), "", &format_entity_impl_cascade);
    let fields = format!("{}\n{}\n{}\n{}\n{}",
                         normal_fields,
                         pointer_fields,
                         one_one_fields,
                         one_many_fields,
                         cascade_detail);
    TPL_IMPL.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${IMPL_FIELDS}", &fields)
}
fn format_entity_trait_get_value(meta: &FieldMeta) -> String {
    format!("ast::Value::from(&self.{})", meta.get_field_name())
}
fn format_entity_trait(meta: &EntityMeta) -> String {
    TPL_TRAIT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
}
fn format_entity_define_field(meta: &FieldMeta) -> String {
    TPL_STRUCT_FIELD.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
}
fn format_entity_impl_field(meta: &FieldMeta) -> String {
    TPL_IMPL_FIELD.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
        .replace("${SET_TYPE}", &meta.get_type_name_set())
}
fn format_entity_impl_pointer(meta: &FieldMeta) -> String {
    TPL_IMPL_POINTER.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
        .replace("${SET_TYPE}", &meta.get_type_name_set())
}
fn format_entity_impl_one_one(meta: &FieldMeta) -> String {
    TPL_IMPL_ONE_ONE.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
        .replace("${SET_TYPE}", &meta.get_type_name_set())
}
fn format_entity_impl_one_many(meta: &FieldMeta) -> String {
    TPL_IMPL_ONE_MANY.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
        .replace("${SET_TYPE}", &meta.get_type_name_set())
}
fn format_entity_impl_cascade(meta: &FieldMeta) -> String {
    TPL_IMPL_CASCADE.to_string().replace("${FIELD}", &meta.get_field_name())
}
