use orm::Entity;
use orm::EntityMeta;
use orm::Insert;
// use orm::Select;
// use orm::Cond;
// use orm::JoinCond;
use orm::Db;
use orm;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

use mysql::Value;
use entity::*;

fn open_db() -> Db {
    orm::open("root", "root", "172.16.16.224", 3306, "test", orm_meta()).unwrap()
}

#[test]
fn test(){
    insert_test();
}
pub fn insert_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");

    let mut insert = Insert::into::<Test>();
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();
    assert!(res == 1);
    assert!(t.get_id() == 1);
}

// pub fn insert_test_2() {
//     let db = open_db();
//     db.rebuild();
//     let mut t = Test::default();
//     t.set_int_val(100);
//     t.set_str_val("hello world");
//     db.insert(&t).unwrap();
//     let id = t.get_id();
//     let t = db.get::<Test>(id).unwrap().unwrap();
//     assert!(t.get_int_val() == 100);
//     assert!(t.get_str_val() == "hello world");
// }

// pub fn update_test() {
//     let db = open_db();
//     db.rebuild();
//     let mut t = Test::default();
//     t.set_int_val(100);
//     t.set_str_val("hello world");
//     db.insert(&t).unwrap();
//     let id = t.get_id();
//     let mut t = db.get::<Test>(id).unwrap().unwrap();
//     assert!(t.get_int_val() == 100);
//     assert!(t.get_str_val() == "hello world");
//     t.set_int_val(200);
//     db.update(&t).unwrap();
//     let mut t = db.get::<Test>(id).unwrap().unwrap();
//     assert!(t.get_int_val() == 200);
//     assert!(t.get_str_val() == "hello world");
// }

// pub fn delete_test() {
//     let db = open_db();
//     db.rebuild();
//     let mut t = Test::default();
//     t.set_int_val(100);
//     t.set_str_val("hello world");
//     db.insert(&t).unwrap();
//     let id = t.get_id();
//     let mut opt = db.get::<Test>(id).unwrap();
//     assert!(opt.is_some());
//     db.delete(opt.as_ref().unwrap()).unwrap();
//     let mut opt = db.get::<Test>(id).unwrap();
//     assert!(opt.is_none());
// }

// pub fn select_refer_test(){
//     let db = open_db();
//     db.rebuild();

//     let mut t = Test::default();
//     t.set_int_val(100);
//     t.set_str_val("hello world");
//     t.set_ptr(&Ptr::default());
//     t.get_ptr().set_int_val(200);
//     db.insert(&t).unwrap();

//     let id = t.get_id();
//     let mut select = Select::from::<Test>();
//     select.with("ptr");
//     select.wher(&Cond::by_id(id));
//     let session = db.open_session();
//     let res = session.query::<Test>(&select).unwrap();
//     let ref t = res[0];
//     assert!(t.get_int_val() == 100);
//     assert!(t.get_str_val() == "hello world");
//     assert!(t.get_ptr().get_int_val() == 200);
// }

// pub fn select_join_test(){
//     let db = open_db();
//     db.rebuild();

//     let mut t = Test::default();
//     t.set_int_val(100);
//     t.set_str_val("hello world");
//     db.insert(&t).unwrap();

//     let id = t.get_id();
//     let mut select = Select::from::<Test>();
//     select.join::<Test>(&JoinCond::by_eq("id", "id"));
//     println!("{}", select.get_sql());
//     // let session = db.open_session();
//     // let res = session.query::<Test>(&select).unwrap();
//     // let ref t = res[0];
//     // assert!(t.get_int_val() == 100);
//     // assert!(t.get_str_val() == "hello world");
//     // assert!(t.get_ptr().get_int_val() == 200);
// }