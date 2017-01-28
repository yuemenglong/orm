use std;
use std::cell::Cell;
use std::ops::Deref;

use syntax;
use syntax::ast::ItemKind::*;
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

static TPL_ENTITY: &'static str = r#"
#[derive(Debug, Clone, Default)]
pub struct ${ENTITY_NAME} {${ENTITY_FIELDS}
}
"#;

static TPL_FIELD: &'static str = r#"
    pub ${FIELD}: Option<${TYPE}>, "#;


static TPL_IMPL: &'static str = r#"
impl ${ENTITY_NAME} {${GETTER_SETTER}
}
"#;

static TPL_GETTER: &'static str = r#"
    #[allow(dead_code)]
    pub fn get_${FIELD}(&self) -> ${TYPE} {
        self.${FIELD}.clone().unwrap()
    }"#;

static TPL_SETTER: &'static str = r#"
    #[allow(dead_code)]
    pub fn set_${FIELD}(&mut self, value: ${TYPE}) {
        self.${FIELD} = Some(value);
    }"#;

static TPL_TRAIT: &'static str = r#"
impl ast::Entity for ${ENTITY_NAME} {
    fn get_meta() -> &'static ast::meta::EntityMeta {
        get_meta().entity_map.get("${ENTITY_NAME}").unwrap()
    }
    fn get_values(&self) -> Vec<ast::Value> {
        vec![${VALUES}]
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

#[derive(Debug)]
pub struct Formatter {
    indent: Cell<usize>,
}

struct Indent<'a> {
    indent: usize,
    formatter: &'a Formatter,
}

impl<'a> Indent<'a> {
    pub fn new(formatter: &'a Formatter) -> Indent<'a> {
        let indent = formatter.indent.get();
        formatter.indent.set(indent + 4);
        Indent {
            indent: indent,
            formatter: formatter,
        }
    }
}

impl<'a> Drop for Indent<'a> {
    fn drop(&mut self) {
        self.formatter.indent.set(self.indent);
    }
}

impl Formatter {
    pub fn new() -> Formatter {
        Formatter { indent: Cell::new(0) }
    }
    pub fn indent_str(&self) -> String {
        std::iter::repeat(" ").take(self.indent.get()).collect::<String>()
    }
    pub fn format_krate(&self, krate: &syntax::ast::Crate) -> String {
        krate.module
            .items
            .iter()
            .map(|item| self.format_item(item.deref()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn format_item(&self, item: &syntax::ast::Item) -> String {
        match item.node {
            Struct(_, _) => self.format_struct(item),
            _ => unreachable!(),
        }
    }
    fn format_struct(&self, item: &syntax::ast::Item) -> String {
        if let Struct(ref variant_data, ref _generics) = item.node {
            if let &VariantData::Struct(ref vec, _id) = variant_data {
                let indent_str = self.indent_str();
                let _indent = Indent::new(self);
                let content = vec.iter()
                    .map(|field| self.format_struct_field(field))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}struct {} {{\n{}\n{}}}",
                        indent_str,
                        item.ident.name.as_str(),
                        content,
                        indent_str)
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    }
    fn format_struct_field(&self, field: &syntax::ast::StructField) -> String {
        let ident = ident_to_string(field.ident.unwrap());
        let ty = ty_to_string(field.ty.deref());
        let attrs = self.format_attrs(&field.attrs);
        let indent_str = self.indent_str();
        match attrs.len() {
            0 => format!("{}{}: {},", indent_str, ident, ty),
            _ => format!("{}{}\n{}{}: {},", indent_str, attrs, indent_str, ident, ty),
        }
    }
    fn format_attrs(&self, attrs: &Vec<syntax::ast::Attribute>) -> String {
        attrs.iter().map(attr_to_string).collect::<Vec<_>>().join("\n")
    }
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
    let fields = meta.fields
        .iter()
        .map(format_entity_field)
        .collect::<Vec<_>>()
        .join("");
    TPL_ENTITY.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${ENTITY_FIELDS}", &fields)
}
fn format_entity_impl(meta: &EntityMeta) -> String {
    let fields = meta.fields
        .iter()
        .filter(|field| !field.pkey)
        .map(format_entity_field_impl)
        .collect::<Vec<_>>()
        .join("");
    TPL_IMPL.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${GETTER_SETTER}", &fields)
}
fn format_entity_trait(meta: &EntityMeta) -> String {
    let values = meta.fields
        .iter()
        .filter(|field| !field.pkey)
        .map(|field| format!("ast::Value::from(&self.{})", field.field_name))
        .collect::<Vec<_>>()
        .join(", ");
    TPL_TRAIT.to_string()
        .replace("${ENTITY_NAME}", &meta.entity_name)
        .replace("${VALUES}", &values)
}
fn format_entity_field(meta: &FieldMeta) -> String {
    TPL_FIELD.to_string().replace("${FIELD}", &meta.field_name).replace("${TYPE}", &meta.ty)
}
fn format_entity_field_impl(meta: &FieldMeta) -> String {
    let getter = TPL_GETTER.to_string()
        .replace("${FIELD}", &meta.field_name)
        .replace("${TYPE}", &meta.ty);
    let setter = TPL_SETTER.to_string()
        .replace("${FIELD}", &meta.field_name)
        .replace("${TYPE}", &meta.ty);
    format!("{}{}", &setter, &getter)
}
