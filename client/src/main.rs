extern crate ast;

use ast::Entity;

mod entity;

fn main(){
    println!("{:?}", entity::Person::get_create_table());
}