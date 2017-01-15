use std;
use std::cell::Cell;
use std::ops::Deref;

use syntax;
use syntax::ast::ItemKind::*;
use syntax::print::pprust::*;
use syntax::ast::VariantData;
use rustc_serialize;

use meta::*;

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
    pub fn format_meta(&self, meta: &OrmMeta) -> String {
        let json = rustc_serialize::json::encode(&meta).unwrap();
        println!("{}", json);
        let content = meta.entities
            .iter()
            .map(|entity| self.format_entity(entity))
            .collect::<Vec<_>>()
            .join("\n\n");
        content
    }
    fn format_entity(&self, meta: &EntityMeta) -> String {
        let _indent = Indent::new(self);
        let content = meta.fields
            .iter()
            .map(|field| self.format_entity_field(field))
            .collect::<Vec<_>>()
            .join("\n");
        format!("#[derive(Debug, Clone, Default)]\npub struct {} {{\n{}\n}}", meta.entity_name, content)
    }
    fn format_entity_field(&self, meta: &FieldMeta) -> String {
        let indent_str = self.indent_str();
        format!("{}pub {}: {},", indent_str, meta.field_name, meta.raw_ty)
    }
}
