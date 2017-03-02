extern crate ast;

use ast::Entity;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;
fn main() {
    let db = ast::open("root", "root", "172.16.16.241", 3306, "test").unwrap();
    refer_test(&db);
}

fn refer_test(db: &ast::DB) {
    db.rebuild(meta()).unwrap();
    let mut person = Person::default();
    person.set_name("Tom");

    let mut addr = Address::default();
    person.set_addr(&addr);
    addr.set_road("中原路");
    person.get_addr().set_no(123);
    println!("{:?}", person);

    let mut account = Account::default();
    account.set_bank("中国银行");
    person.set_account(&account);
    println!("{:?}", person);
    
    db.insert(&person).unwrap();
    println!("{:?}", person);

    person.set_name("Bob");
    db.update(&person).unwrap();
    println!("{:?}", person);
}

fn curd_test(db: &ast::DB) {
    let mut p = Person::default();
    p.set_age(100);
    p.set_name("Tom");
    db.drop_table::<Person>().unwrap();
    let ret = db.create_table::<Person>().unwrap();
    db.insert(&p).unwrap();
    println!("{:?}", p);
    let id = p.get_id();
    p.set_name("Dick");
    let ret = db.update(&p).unwrap();
    let p = db.get::<Person>(p.get_id()).unwrap().unwrap();
    println!("{:?}", p);
    db.delete(p).unwrap();
    let p = db.get::<Person>(id).unwrap();
    println!("{:?}", p);
}

// 多对多的场景
// 1. 