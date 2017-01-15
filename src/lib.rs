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

lazy_static! {
    static ref META: OrmMeta = {
        let json = r#"{"entities":[{"entity_name":"Person","table_name":"Person","pkey":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"fields":[{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false},{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false}],"field_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}},"column_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}}}],"entity_map":{"Person":{"entity_name":"Person","table_name":"Person","pkey":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"fields":[{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false},{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false}],"field_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}},"column_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}}}},"table_map":{"Person":{"entity_name":"Person","table_name":"Person","pkey":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"fields":[{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false},{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false}],"field_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}},"column_map":{"age":{"field_name":"age","column_name":"age","ty":"i32","db_ty":"`age` INTEGER NOT NULL","raw_ty":"i32","nullable":false,"len":0,"pkey":false,"extend":false},"id":{"field_name":"id","column_name":"id","ty":"u64","db_ty":"`id` BIGINT PRIMARY KEY AUTOINCREMENT","raw_ty":"Option<u64>","nullable":false,"len":0,"pkey":true,"extend":true},"name":{"field_name":"name","column_name":"name","ty":"String","db_ty":"`name` VARCHAR(32) NOT NULL","raw_ty":"String","nullable":false,"len":32,"pkey":false,"extend":false}}}}}"#;
        rustc_serialize::json::decode(json).unwrap()
    };
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
    // println!("{}", ret);
    // let ret = formatter.format_krate(&krate);
    // println!("{:?}", visitor.meta);
    // println!("{}", ret);
}
