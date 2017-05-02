use orm::Entity;
use orm::EntityMeta;
use orm::Select;
use orm::Cond;
use orm::DB;
use orm;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

use mysql::Value;
use entity::*;

fn openDB() -> DB {
    orm::open("root", "root", "172.16.16.224", 3306, "test", orm_meta()).unwrap()
}

#[test]
fn test(){
    insert_test();
    update_test();
    delete_test();
}

pub fn insert_test() {
    let db = openDB();
    db.rebuild();
    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    db.insert(&t).unwrap();
    let id = t.get_id();
    let t = db.get::<Test>(id).unwrap().unwrap();
    assert!(t.get_int_val() == 100);
    assert!(t.get_str_val() == "hello world");
}

pub fn update_test() {
    let db = openDB();
    db.rebuild();
    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    db.insert(&t).unwrap();
    let id = t.get_id();
    let mut t = db.get::<Test>(id).unwrap().unwrap();
    assert!(t.get_int_val() == 100);
    assert!(t.get_str_val() == "hello world");
    t.set_int_val(200);
    db.update(&t).unwrap();
    let mut t = db.get::<Test>(id).unwrap().unwrap();
    assert!(t.get_int_val() == 200);
    assert!(t.get_str_val() == "hello world");
}

pub fn delete_test() {
    let db = openDB();
    db.rebuild();
    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    db.insert(&t).unwrap();
    let id = t.get_id();
    let mut opt = db.get::<Test>(id).unwrap();
    assert!(opt.is_some());
    db.delete(opt.as_ref().unwrap()).unwrap();
    let mut opt = db.get::<Test>(id).unwrap();
    assert!(opt.is_none());
}