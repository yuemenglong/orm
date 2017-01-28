extern crate ast;

use ast::Entity;
use ast::sql;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;

fn main(){
    // println!("{:?}", sql::sql_create_table(Person::get_meta()));
    let mut p = Person::default();
    p.set_age(100);
    p.set_name("Tom".to_string());
    // p.set_id(10);
    // println!("{:?}", p.get_params());
    let db = ast::open("root", "root", "192.168.31.203", 3306, "test").unwrap();
    db.drop_table::<Person>().unwrap();
    let ret = db.create_table::<Person>().unwrap();
    println!("{:?}", ret);
    let mut p = db.insert(&p).unwrap();
    p.set_name("Dick".to_string());
    let ret = db.update(&p).unwrap();
}
