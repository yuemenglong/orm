extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;

use syntax::codemap::CodeMap;
use syntax::parse::{self, ParseSess};
use errors::emitter::ColorConfig;
use errors::Handler;
use syntax::ast::ItemKind::*;
use syntax::print::pprust::*;
use syntax::ast::VariantData;
use syntax::ast::MetaItemKind;
use syntax::ast::NestedMetaItemKind;
use syntax::ast::LitKind;

use std::rc::Rc;

fn create_parse_session() -> ParseSess {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    parse_session
}

static SRC: &'static str = "
struct Name {
    #[derive(Debug, asdf=\"123\")]
    field: i32
}
";

// fn visit_struct(variantData: &syntax::ast::VariantData, generics: &syntax::ast::Generics) {
fn visit_struct(item: &syntax::ast::Item) {
    if let Struct(ref variantData, ref generics) = item.node {
        println!("Struct Name: {:?}", item.ident.name.as_str());
        if let &VariantData::Struct(ref vec, id) = variantData {
            // println!("{:?}", vec);
            for field in vec {
                visit_struct_field(field);
            }
        } else {
            unreachable!();
        }
    } else {
        unreachable!();
    }
}

fn visit_struct_field(field: &syntax::ast::StructField) {
    println!("{:?} {:?}", field.ident, field.ty);
    // println!("{:?}", field.attrs);
    for attr in field.attrs.iter() {
        visit_struct_field_attr(attr);
    }
}

fn visit_struct_field_attr(attr: &syntax::ast::Attribute) {
    visit_meta_item(&attr.value);
}

fn visit_meta_item(item: &syntax::ast::MetaItem) {
    println!("MetaItem Name: {:?}", item.name);
    match item.node {
        MetaItemKind::Word => {
            println!("MetaItemKind::Word");
        }
        MetaItemKind::List(ref vec) => {
            println!("MetaItemKind::List");
            for item in vec {
                // println!("Item {:?}", item);
                visit_nest_meta_item(&item);
            }
        }
        MetaItemKind::NameValue(ref lit) => {
            println!("MetaItemKind::NameValue");
            visit_lit_meta_item(lit);
        }
    }
}

fn visit_lit_meta_item(lit: &syntax::ast::Lit) {
    match (lit.node) {
        LitKind::Str(ref symbol, ref strStyle) => {
            println!("{:?}", symbol.as_str());
        }
        _ => {}
    }
}

fn visit_nest_meta_item(item: &syntax::ast::NestedMetaItem) {
    match (item.node) {
        NestedMetaItemKind::MetaItem(ref item) => {
            visit_meta_item(&item);
        }
        _ => {}

    }
}

fn print_struct() {}

fn main() {
    let parse_session = create_parse_session();
    let krate =
        parse::parse_crate_from_source_str("stdin".to_string(), SRC.to_string(), &parse_session)
            .unwrap();
    // println!("{:?}", krate.module.items);
    println!("{:?}", krate.module.items.len());
    for item in krate.module.items {
        // println!("{:?}", item.node);
        match item.node {
            Fn(ref decl, unsafety, constness, _, ref generics, ref block) => {
                // println!("{:?}", (decl, unsafety, constness, abi, generics, block));
                // let s = fn_block_to_string(decl);
                let s = fun_to_string(decl, unsafety, constness.node, item.ident, generics);
                println!("{:?}", s);
                let s = block_to_string(block);
                println!("{:?}", s);
            }
            Struct(_, _) => {
                visit_struct(&item);
            }
            _ => {}
        }
    }
}
