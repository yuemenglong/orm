extern crate ast;

use ast::Entity;
use ast::sql;

mod entity;
use entity::*;

fn main(){
    println!("{:?}", sql::sql_create_table(Person::get_meta()));
    let mut p = Person::default();
    p.set_age(100);
    p.set_name("Tom".to_string());
    // p.set_id(10);
    println!("{:?}", p.get_params());
    let db = ast::open("root", "root", "172.16.16.213", 3306, "test").unwrap();
    let ret = db.create_table::<Person>().unwrap();
    println!("{:?}", ret);
    let ret = db.insert(&p).unwrap();
    println!("{:?}", ret);
}
