use rustc_serialize;

use meta::*;

static TPL: &'static str = r#"
use orm;
use orm::Entity;
use std;

static mut ORM_META: Option<&'static orm::meta::OrmMeta> = None;
static ONCE: std::sync::Once = std::sync::ONCE_INIT;

pub fn orm_meta() -> &'static orm::meta::OrmMeta {
    let json = r#${JSON}#;
    ONCE.call_once(|| unsafe { ORM_META = Some(orm::init::init_meta(json)) });
    unsafe { ORM_META.unwrap() }
}

${ENTITIES}
"#;

static TPL_STRUCT: &'static str = r#"
#[derive(Debug, Clone)]
pub struct ${ENTITY_NAME} {
    inner: orm::EntityInnerPointer,
}
"#;

static TPL_IMPL: &'static str = r#"
impl ${ENTITY_NAME} {${IMPL_FIELDS}
}
"#;

static TPL_IMPL_VALUE: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> ${TYPE} {
        self.inner_get_value::<${TYPE}>("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set_value("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}_null(&self) {
        self.inner_set_value_null("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn is_${FIELD}_null(&self) -> bool {
        self.inner_is_value_null("${FIELD}")
    }"#;

static TPL_IMPL_ENTITY: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> Box<${TYPE}> {
        Box::new(self.inner_get_entity("${FIELD}"))
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${SET_TYPE}) {
        self.inner_set_entity("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}_null(&self) {
        self.inner_set_entity_null("${FIELD}")
    }
    #[allow(dead_code)]
    pub fn is_${FIELD}_null(&self) -> bool {
        self.inner_is_entity_null("${FIELD}")
    }"#;

static TPL_IMPL_VEC: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> Box<Vec<${TYPE}>> {
        Box::new(self.inner_get_vec("${FIELD}"))
    }
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: Vec<${TYPE}>) {
        self.inner_set_vec("${FIELD}", value);
    }
    #[allow(dead_code)]
    pub fn is_${FIELD}_null(&self) -> bool {
        self.inner_is_vec_null("${FIELD}")
    }"#;

static TPL_TRAIT: &'static str = r#"
impl orm::Entity for ${ENTITY_NAME} {
    fn orm_meta() -> &'static orm::meta::OrmMeta {
        orm_meta()
    }
    fn meta() -> &'static orm::meta::EntityMeta {
        orm_meta().entity_map.get("${ENTITY_NAME}").unwrap()
    }
    fn default() -> Self {
        ${ENTITY_NAME} {
            inner: std::rc::Rc::new(std::cell::RefCell::new(orm::EntityInner::default(Self::meta(), Self::orm_meta())))
        }
    }
    fn new() -> Self {
        ${ENTITY_NAME} {
            inner: std::rc::Rc::new(std::cell::RefCell::new(orm::EntityInner::new(Self::meta(), Self::orm_meta())))
        }
    }
    fn from_inner(inner: orm::EntityInnerPointer) -> ${ENTITY_NAME} {
        ${ENTITY_NAME} {
            inner: inner,
        }
    }
    fn inner(&self) -> orm::EntityInnerPointer {
        self.inner.clone()
    }
}
"#;

pub fn format_meta(meta: &OrmMeta) -> String {
    let json = format!("\"{}\"", rustc_serialize::json::encode(&meta).unwrap());
    let entities = meta.get_entities()
        .into_iter()
        .map(format_entity)
        .collect::<Vec<_>>()
        .join("");
    let tpl = TPL.to_string();
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
        .replace("${ENTITY_NAME}", &meta.entity)
}
fn format_entity_impl(meta: &EntityMeta) -> String {
    let fields = meta.field_vec
        .iter()
        .map(|field| {
            let field_meta = meta.field_map.get(field).expect(&expect!());
            format_entity_field(field_meta)
        })
        .collect::<Vec<_>>()
        .join("\n");
    TPL_IMPL.to_string()
        .replace("${ENTITY_NAME}", &meta.entity)
        .replace("${IMPL_FIELDS}", &fields)
}
fn format_entity_field(meta: &FieldMeta) -> String {
    let tpl = match meta {
        &FieldMeta::Id { .. } |
        &FieldMeta::String { .. } |
        &FieldMeta::Integer { .. } => TPL_IMPL_VALUE,
        &FieldMeta::Refer { .. } |
        &FieldMeta::Pointer { .. } |
        &FieldMeta::OneToOne { .. } => TPL_IMPL_ENTITY,
        &FieldMeta::OneToMany { .. } => TPL_IMPL_VEC,
    };
    tpl.to_string()
        .replace("${FIELD}", &meta.get_field_name())
        .replace("${TYPE}", &meta.get_type_name())
        .replace("${SET_TYPE}", &meta.get_type_name_set())
}
fn format_entity_trait(meta: &EntityMeta) -> String {
    TPL_TRAIT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity)
}
