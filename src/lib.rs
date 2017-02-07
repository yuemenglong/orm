extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;
extern crate regex;
extern crate mysql;
extern crate rustc_serialize;

pub use rustc_serialize::json;

mod formatter;
mod visitor;
mod anno;
mod db;
mod cond;
mod entity;
pub mod sql;
pub mod init;
pub mod meta;

pub use entity::Entity;
pub use entity::EntityInner;
pub use mysql::Value;
pub use mysql::Row;
pub use db::DB;

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
    let meta = visitor::visit_krate(&krate);
    let ret = formatter::format_meta(&meta);
    ret
}

pub fn open(user: &str, pwd: &str, host: &str, port: u16, db: &str) -> Result<DB, mysql::Error> {
    let conn_str = format!("mysql://{}:{}@{}:{}/{}", user, pwd, host, port, db);
    match mysql::Pool::new(conn_str.as_ref()) {
        Ok(pool) => Ok(DB { pool: pool }),
        Err(err) => Err(err),
    }
}
