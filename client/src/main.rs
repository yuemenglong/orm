extern crate ast;

use ast::Entity;

mod entity;

fn main(){
    println!("{:?}", entity::Person::get_create_table());
    let mut e = entity::Person::default();
    e.set_age(100);
    println!("{:?}", e.get_params());
}
