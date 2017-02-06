extern crate ast;

use ast::Entity;
use ast::sql;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;

fn main() {
    // println!("{:?}", sql::sql_create_table(Person::get_meta()));

    // p.set_id(10);
    // println!("{:?}", p.get_params());
    let db = ast::open("root", "root", "172.16.16.213", 3306, "test").unwrap();
    curd_test(&db);
}

fn curd_test(db: &ast::DB) {
    let mut p = Person::default();
    p.set_age(100);
    p.set_name("Tom".to_string());
    db.drop_table::<Person>().unwrap();
    let ret = db.create_table::<Person>().unwrap();
    let mut p = db.insert(&p).unwrap();
    println!("{:?}", p);
    let id = p.get_id();
    p.set_name("Dick".to_string());
    let ret = db.update(&p).unwrap();
    let p = db.get::<Person>(p.get_id()).unwrap().unwrap();
    println!("{:?}", p);
    db.delete(p).unwrap();
    let p = db.get::<Person>(id).unwrap();
    println!("{:?}", p);
}
