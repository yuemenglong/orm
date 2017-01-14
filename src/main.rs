extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;
extern crate regex;
extern crate mysql;
extern crate rustc_serialize;

mod formatter;
mod visitor;
mod anno;
mod meta;
mod types;
mod db;
mod cond;
mod entity;

#[macro_use]
mod lazy_static_macro;

pub use formatter::Formatter;
pub use visitor::Visitor;
pub use db::DB;

use meta::*;

use syntax::codemap::CodeMap;
use syntax::parse::{self, ParseSess};
use errors::emitter::ColorConfig;
use errors::Handler;

use std::rc::Rc;

fn create_parse_session() -> ParseSess {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    parse_session
}

static SRC: &'static str = "
struct Person{
    #[len(32)]
    name:String,
    age: i32,
}
";

use std::collections::HashMap;

lazy_static! {
    static ref HASHMAP: HashMap<u32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0, "foo");
        m.insert(1, "bar");
        m.insert(2, "baz");
        m
    };
    static ref COUNT: usize = HASHMAP.len();
}

fn main() {
    let parse_session = create_parse_session();
    let krate =
        parse::parse_crate_from_source_str("stdin".to_string(), SRC.to_string(), &parse_session)
            .unwrap();
    // println!("{:?}", krate.module.items);
    // println!("{:?}", krate.module.items.len());
    let mut visitor = Visitor::new();
    visitor.visit_krate(&krate);
    let formatter = Formatter::new();
    let ret = formatter.format_meta(&visitor.meta);
    println!("{}", ret);
    // let ret = formatter.format_krate(&krate);
    // println!("{:?}", visitor.meta);
    // println!("{}", ret);
}
