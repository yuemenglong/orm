extern crate ast;

use ast::Entity;
use ast::sql;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;

fn main() {
    let db = ast::open("root", "root", "172.16.16.213", 3306, "test").unwrap();
    refer_test(&db);
}

fn refer_test(db: &ast::DB){
    db.rebuild(meta());
    let mut p = Person::default();
    p.set_name("Tom".to_string());
    let mut a = Address::default();
    p.set_addr(&a);
    println!("{:?}", p);
    db.insert(&p).unwrap();
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
