extern crate orm;
extern crate mysql;

use orm::Entity;
use orm::EntityMeta;
use orm::Select;
use orm::Cond;

use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

use mysql::Value;

mod entity;
use entity::*;

mod test;

// grant all privileges on *.* to root@'%' identified by 'root';
// flush privileges;
fn main() {
    test::select_test();
}
