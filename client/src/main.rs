extern crate ast;
extern crate mysql;

use ast::Entity;
use ast::EntityMeta;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

// use mysql;
use mysql::Value;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;
fn main() {
    let db = ast::open("root", "root", "192.168.31.203", 3306, "test").unwrap();
    refer_test(&db);
    db.get::<Person>(1).unwrap();
}

fn refer_test(db: &ast::DB) {
    db.rebuild(orm_meta()).unwrap();
    let mut person = Person::default();
    person.set_name("Tom");

    let mut addr = Address::default();
    person.set_addr(&addr);
    addr.set_road("中原路");
    person.get_addr().set_no(123);

    let mut account = Account::default();
    account.set_bank("中国银行");
    person.set_account(&account);
    db.insert(&person).unwrap();

    person.set_name("Bob");
    person.get_addr().cascade_null();
    person.get_account().cascade_null();
    db.update(&person).unwrap();
    person.debug();

    // let mut account = Account::default();
    // account.set_no("123456");
    // account.cascade_insert();
    // person.set_account(&account);
    // person.clear_addr();
    // db.update(&person).unwrap();

    // let mut child1 = Child::default();
    // child1.set_name("xuan");
    // child1.cascade_insert();
    // let mut child2 = Child::default();
    // child2.set_name("yuan");
    // child2.cascade_insert();
    // person.set_children(vec![child1, child2]);
    // db.update(&person).unwrap();

    // person.set_children(vec![]);
    // person.get_account().cascade_null();
    // person.cascade_addr_null();
    // db.update(&person).unwrap();

    // person.set_addr2(&Address::default());
    // person.get_addr2().set_road("1");
    // person.cascade_addr_null();
    // person.cascade_account_null();
    // person.cascade_children_null();
    // db.update(&person).unwrap();

    // person.debug();
    // db.delete(person).unwrap();

    // person.clear_account();
    // person.debug();
    // account.debug();
}

fn curd_test(db: &ast::DB) {
    let mut p = Person::default();
    p.set_age(100);
    p.set_name("Tom");
    db.drop_table::<Person>().unwrap();
    let ret = db.create_table::<Person>().unwrap();
    db.insert(&p).unwrap();
    let id = p.get_id();
    p.set_name("Dick");
    let ret = db.update(&p).unwrap();
    let p = db.get::<Person>(p.get_id()).unwrap();
    db.delete(p).unwrap();
    let p = db.get::<Person>(id).unwrap();
}

fn select_field(meta: &EntityMeta, alias: &str) -> String {
    let fields = meta.get_non_refer_fields()
        .into_iter()
        .map(|field| {
            let column_name = field.get_column_name();
            let field_name = field.get_field_name();
            format!("{}.{} as {}${}", alias, column_name, alias, field_name)
        })
        .collect::<Vec<_>>()
        .join(", ");
    fields
}

fn select(meta: &EntityMeta) {
    // let entity_name = meta.entity_name.to_string();
    // let table_name = meta.table_name.to_string();
    // let alias = entity_name.to_string();
    // let mut fields = select_field(meta, alias.as_ref());
    // let mut tables = format!("{} as {}", table_name, entity_name);
    // // println!("{:?}", sql);
    // for a_b_meta in meta.get_pointer_fields() {
    //     let join_alias = format!("{}_{}", entity_name, a_b_meta.get_field_name());
    //     let join_table = a_b_meta.get_refer_table();
    //     // println!("{:?}", join_alias);
    //     // a.b_id = b.id
    //     let join_sql = format!(" left join {} as {} on {}.{} = {}.id",
    //                            join_table,
    //                            join_alias,
    //                            alias,
    //                            a_b_meta.get_pointer_id(),
    //                            join_alias);
    //     tables.push_str(join_sql.as_ref());
    //     let join_meta = orm_meta().entity_map.get(&a_b_meta.get_refer_entity()).unwrap();
    //     let join_fields = select_field(join_meta, join_alias.as_ref());
    //     fields.push_str(", ");
    //     fields.push_str(join_fields.as_ref());
    // }
    // let sql = format!("select {} from {}", fields, tables);
    // println!("{:?}", sql);
}
