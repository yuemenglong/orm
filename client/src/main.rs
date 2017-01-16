extern crate ast;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::fs::OpenOptions;
use std::io::Write;

mod entity;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = Path::new(&dir).join("src/entity.in.rs");
    let mut src = String::new();
    File::open(path).unwrap().read_to_string(&mut src).unwrap();
    let build = ast::build(&src);

    let path = Path::new(&dir).join("src/entity.rs");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .unwrap();
    file.write_all(build.as_bytes()).unwrap();
}
