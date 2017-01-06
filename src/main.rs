extern crate syntex_syntax as syntax;
extern crate syntex_errors as errors;

use syntax::codemap::{CodeMap};
use syntax::parse::{self, ParseSess};
use errors::emitter::{ColorConfig};
use errors::{Handler};

use std::rc::Rc;

fn create_parse_session() ->ParseSess{
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    parse_session
}

fn main(){
    let parse_session = create_parse_session();
    let krate = parse::parse_crate_from_source_str("stdin".to_string(), "fn main(){}".to_string(), &parse_session).unwrap();
    // println!("{:?}", krate.module.items);
    println!("{:?}", krate.module.items.len());
    for item in krate.module.items{
        println!("{:?}", item.node);
    }
}