use orm::Entity;
use orm::EntityMeta;
use orm::Insert;
use orm::Select;
use orm::Execute;
use orm::Cond;
use orm::JoinCond;
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
fn test() {
    insert_test();
}
pub fn insert_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");

    let mut insert = Execute::insert();
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();
    assert!(res == 1);
    assert!(t.get_id() == 1);
}

pub fn insert_refer_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    t.set_ptr(&Ptr::default());
    t.get_ptr().set_int_val(200);
    t.set_oo(&Oo::default());
    t.get_oo().set_int_val(300);
    t.set_om(vec![Om::default(), Om::default()]);
    t.get_om().get_mut(0).unwrap().set_int_val(400);
    t.get_om().get_mut(1).unwrap().set_int_val(500);

    let mut insert = Insert::new();
    insert.with("ptr");
    insert.with("oo");
    insert.with("om");
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();
    assert!(t.get_id() == 1);
    assert!(t.get_ptr_id() == 1);
    assert!(t.get_ptr().get_id() == 1);
    assert!(t.get_oo().get_test_id() == 1);
    assert!(t.get_oo().get_id() == 1);
    assert!(res == 5);
}

pub fn insert_refer_exists_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    t.set_ptr(&Ptr::default());
    t.get_ptr().set_int_val(200);
    t.set_oo(&Oo::default());
    t.get_oo().set_int_val(300);

    let mut insert = Insert::new();
    insert.with("ptr");
    insert.with("oo");
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();
    assert!(t.get_id() == 1);
    assert!(t.get_ptr_id() == 1);
    assert!(t.get_ptr().get_id() == 1);
    assert!(t.get_oo().get_test_id() == 1);
    assert!(t.get_oo().get_id() == 1);
    assert!(res == 3);
}


pub fn insert_select_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    t.set_ptr(&Ptr::default());
    t.get_ptr().set_int_val(200);
    t.set_oo(&Oo::default());
    t.get_oo().set_int_val(300);
    t.set_om(vec![Om::default(), Om::default()]);
    t.get_om().get_mut(0).unwrap().set_int_val(400);
    t.get_om().get_mut(1).unwrap().set_int_val(500);

    let mut insert = Insert::new();
    insert.with("ptr");
    insert.with("oo");
    insert.with("om");
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();
    assert!(t.get_id() == 1);
    assert!(t.get_ptr_id() == 1);
    assert!(t.get_ptr().get_id() == 1);
    assert!(t.get_oo().get_test_id() == 1);
    assert!(t.get_oo().get_id() == 1);
    assert!(res == 5);

    let mut select = Select::<Test>::new();
    select.wher(&Cond::by_id(t.get_id()));
    select.with("ptr");
    select.with("oo");
    select.with("om");
    let t = select.query(&mut db.get_conn()).unwrap().remove(0);
    assert!(t.get_int_val() == 100);
    assert!(t.get_ptr().get_int_val() == 200);
    assert!(t.get_oo().get_int_val() == 300);
    assert!(t.get_om()[0].get_int_val() == 400);
    assert!(t.get_om()[1].get_int_val() == 500);
    assert!(res == 5);

    t.debug();
}

pub fn join_test() {
    let db = open_db();
    db.rebuild();

    let mut t = Test::default();
    t.set_int_val(100);
    t.set_str_val("hello world");
    t.set_ptr(&Ptr::default());
    t.get_ptr().set_int_val(200);
    t.set_oo(&Oo::default());
    t.get_oo().set_int_val(300);
    t.set_om(vec![Om::default(), Om::default()]);
    t.get_om().get_mut(0).unwrap().set_int_val(400);
    t.get_om().get_mut(1).unwrap().set_int_val(500);

    let mut insert = Insert::new();
    insert.with("ptr");
    insert.with("oo");
    insert.with("om");
    let res = insert.execute(&mut db.get_conn(), &t).unwrap();

    let mut select = Select::<Test>::new();
    select.wher(&Cond::by_id(t.get_id()));
    select.with("ptr");
    select.with("oo");
    select.with("om");
    {
        let mut join = select.join::<Test>(&JoinCond::by_eq("id", "id"));
        join.wher(&Cond::by_gt("id", 0));
        join.with("om");
    }
    let res = select.query_ex(&mut db.get_conn()).unwrap();
    let t = &res[0][0];
    let t2 = &res[1][0];
    assert!(t.get_int_val() == 100);
    assert!(t.get_ptr().get_int_val() == 200);
    assert!(t.get_oo().get_int_val() == 300);
    assert!(t.get_om()[0].get_int_val() == 400);
    assert!(t.get_om()[1].get_int_val() == 500);

    assert!(t2.get_int_val() == 100);
    assert!(t2.get_om()[0].get_int_val() == 400);
    assert!(t2.get_om()[1].get_int_val() == 500);

    t.debug();
    t2.debug();
}


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
//     // let session = db.open_session();
//     // let res = session.query::<Test>(&select).unwrap();
//     // let ref t = res[0];
//     // assert!(t.get_int_val() == 100);
//     // assert!(t.get_str_val() == "hello world");
//     // assert!(t.get_ptr().get_int_val() == 200);
// }
