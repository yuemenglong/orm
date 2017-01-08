extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;

mod formatter;
mod visitor;
pub use formatter::Formatter;
pub use visitor::Visitor;

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
struct Name {
    // #[derive(Debug, asdf=\"123\")]
    field: i32,
    id:i64,
}
";


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
    let ret = formatter.format_krate(&krate);
    println!("{:?}", visitor.meta);
    println!("{}", ret);
}
