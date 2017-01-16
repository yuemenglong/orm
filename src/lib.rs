extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;
extern crate regex;
extern crate mysql;
extern crate rustc_serialize;

pub use rustc_serialize::json as json;

mod formatter;
mod visitor;
mod anno;
mod meta;
mod types;
mod db;
mod cond;
mod entity;
pub mod init;

use formatter::Formatter;
use visitor::Visitor;

pub use meta::*;
pub use entity::Entity;
// pub use db::DB;

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

pub fn build(src: &str) -> String {
    let parse_session = create_parse_session();
    let krate =
        parse::parse_crate_from_source_str("stdin".to_string(), src.to_string(), &parse_session)
            .unwrap();
    // TODO 重构visitor全部用方法 不用类
    let mut visitor = Visitor::new();
    visitor.visit_krate(&krate);
    let formatter = Formatter::new();
    let ret = formatter.format_meta(&visitor.meta);
    ret
}

fn test(){
    static T:i32 = 1;
    println!("{:?}", T);
}
