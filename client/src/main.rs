extern crate ast;

use ast::Entity;
use ast::EntityMeta;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

mod entity;
use entity::*;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;
fn main() {
    let db = ast::open("root", "root", "172.16.16.241", 3306, "test").unwrap();
    // select(Person::meta());
    refer_test(&db);
}

fn refer_test(db: &ast::DB) {
    db.rebuild(orm_meta()).unwrap();
    let mut person = Person::default();
    person.set_name("Tom");

    let mut addr = Address::default();
    person.set_addr(&addr);
    addr.set_road("中原路");
    person.get_addr().set_no(123);
    // person.debug();

    let mut account = Account::default();
    account.set_bank("中国银行");
    person.set_account(&account);
    // person.debug();

    db.insert(&person).unwrap();
    person.debug();

    // person.set_name("Bob");
    // db.update(&person).unwrap();
    // person.debug();

    // person.clear_addr();
    // person.debug();

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
    println!("{:?}", p);
    let id = p.get_id();
    p.set_name("Dick");
    let ret = db.update(&p).unwrap();
    let p = db.get::<Person>(p.get_id()).unwrap();
    println!("{:?}", p);
    db.delete(p).unwrap();
    let p = db.get::<Person>(id).unwrap();
    println!("{:?}", p);
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
    let entity_name = meta.entity_name.to_string();
    let table_name = meta.table_name.to_string();
    let alias = entity_name.to_string();
    let mut fields = select_field(meta, alias.as_ref());
    let mut tables = format!("{} as {}", table_name, entity_name);
    // println!("{:?}", sql);
    for a_b_meta in meta.get_pointer_fields() {
        let join_alias = format!("{}_{}", entity_name, a_b_meta.get_field_name());
        let join_table = a_b_meta.get_refer_table();
        // println!("{:?}", join_alias);
        // a.b_id = b.id
        let join_sql = format!(" left join {} as {} on {}.{} = {}.id",
                               join_table,
                               join_alias,
                               alias,
                               a_b_meta.get_pointer_id(),
                               join_alias);
        tables.push_str(join_sql.as_ref());
        let join_meta = orm_meta().entity_map.get(&a_b_meta.get_refer_entity()).unwrap();
        let join_fields = select_field(join_meta, join_alias.as_ref());
        fields.push_str(", ");
        fields.push_str(join_fields.as_ref());
    }
    let sql = format!("select {} from {}", fields, tables);
    println!("{:?}", sql);
}
