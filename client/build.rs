extern crate orm;

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = Path::new(&dir).join("src/entity.in.rs");
    let mut src = String::new();
    File::open(path).unwrap().read_to_string(&mut src).unwrap();
    let build = orm::build(&src);

    let path = Path::new(&dir).join("src/entity.rs");
    std::fs::remove_file(path.clone()).map_err(|err| println!("{:?}", err));
    let mut file = File::create(path).unwrap();
    file.write_all(build.as_bytes()).unwrap();
}
