extern crate ast;

use std::fs::File;
use std::io::Read;
use std::path::Path;

fn main() {
    let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = Path::new(&path).join("src/entity.in.rs");
    // println!("{:?}", path);
    let mut src = String::new();
    File::open(path).unwrap().read_to_string(&mut src).unwrap();
    println!("{:?}", src);
    ast::build(&src);
    //.read_to_string(&mut src).unwrap();
    // let path = Path::new(.as_ref()).join("entity.in.rs");
    // println!("{:?}", path);
    // println!("{:?}", file!());
    // println!("{:?}", std::env::current_dir());
    // println!("{:?}", std::env::var("CARGO_MANIFEST_DIR"));
    // println!("{:?}", std::env::vars());
    // for var in std::env::vars(){
    //     println!("{:?}", var);
    // }
    // println!("{:?}", Path::new(file!()).parent().unwrap().to_str());
    // let mut src: String = String::new();
    // match File::open("./entity.in.rs") {
    //     Ok(ref mut file) => {
    //         file.read_to_string(&mut src).unwrap();
    //     }
    //     Err(err) => {
    //         println!("{:?}", err);
    //     }
    // }
    // println!("{:?}", src);
}
