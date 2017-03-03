use mysql::Pool;
use mysql::Error;
use mysql::Value;

use mysql::prelude::GenericConnection;
use meta;
use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::cell::RefCell;

// use cond::Cond;
use entity::Entity;
use entity::EntityInner;
use meta::FieldMeta;

pub struct DB {
    pub pool: Pool,
}

impl DB {
    pub fn rebuild(&self, meta: &meta::OrmMeta) -> Result<u64, Error> {
        try!(self.drop_tables(meta));
        Ok(try!(self.create_tables(meta)))
    }
    pub fn create_tables(&self, meta: &meta::OrmMeta) -> Result<u64, Error> {
        let mut ret = 0;
        for entity_meta in meta.entities.iter() {
            let sql = entity_meta.sql_create_table();
            println!("{}", sql);
            match self.pool.prep_exec(sql, ()) {
                Ok(res) => ret += res.affected_rows(),
                Err(err) => {
                    return Err(err);
                }
            }
        }
        return Ok(ret);
    }
    pub fn drop_tables(&self, meta: &meta::OrmMeta) -> Result<u64, Error> {
        let mut ret = 0;
        for entity_meta in meta.entities.iter() {
            let sql = entity_meta.sql_drop_table();
            println!("{}", sql);
            match self.pool.prep_exec(sql, ()) {
                Ok(res) => ret += res.affected_rows(),
                Err(err) => {
                    return Err(err);
                }
            }
        }
        return Ok(ret);
    }
    pub fn create_table<E: Entity>(&self) -> Result<u64, Error> {
        let sql = E::meta().sql_create_table();
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn drop_table<E: Entity>(&self) -> Result<u64, Error> {
        let sql = E::meta().sql_drop_table();
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn insert<E: Entity + Clone>(&self, entity: &E) -> Result<(), Error> {
        let inner_rc = entity.inner();
        let mut inner = inner_rc.borrow_mut();
        do_insert(inner.deref_mut(), self.pool.get_conn().as_mut().unwrap())
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let inner_rc = entity.inner();
        let mut inner = inner_rc.borrow_mut();
        do_update(inner.deref_mut(), self.pool.get_conn().as_mut().unwrap())
    }
    pub fn get<E: Entity>(&self, id: u64) -> Result<E, Error> {
        let mut inner = EntityInner::new(E::meta());
        inner.field_map.insert("id".to_string(), Value::from(id));
        try!(do_get(&mut inner, self.pool.get_conn().as_mut().unwrap()));
        Ok(E::new(Rc::new(RefCell::new(inner))))
    }
    pub fn delete<E: Entity>(&self, entity: E) -> Result<u64, Error> {
        let sql = E::meta().sql_delete();
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, vec![("id", entity.get_id())]);
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    //     // pub fn select<'a, E: Entity>(&'a self, conds: Vec<Cond>) -> SelectBuilder<'a, E> {
    //     //     SelectBuilder::<'a, E> {
    //     //         pool: &self.pool,
    //     //         conds: RefCell::new(conds),
    //     //         phantom: PhantomData,
    //     //     }
    //     // }
}


fn do_insert<C>(inner: &mut EntityInner, conn: &mut C) -> Result<(), Error>
    where C: GenericConnection
{
    // 遍历pointer
    for (refer_field, opt) in inner.pointer_map.clone() {
        if opt.is_none() {
            // lazy load, 可以直接跳过，说明没有需要操作的
            continue;
        }
        let refer_inner_rc = opt.unwrap();
        let mut refer_inner = refer_inner_rc.borrow_mut();
        let refer_meta = inner.meta.field_map.get(&refer_field).unwrap();
        try!(do_cascade_insert(inner, refer_inner.deref_mut(), refer_meta, conn));
    }

    try!(inner.do_insert(conn));

    // 需要等a写入后才能写b，因为aid在b上，需要a先有id
    for (refer_field, opt) in inner.one_one_map.clone() {
        if opt.is_none() {
            // lazy load
            continue;
        }
        let refer_inner_rc = opt.unwrap();
        let mut refer_inner = refer_inner_rc.borrow_mut();
        let refer_meta = inner.meta.field_map.get(&refer_field).unwrap();
        try!(do_cascade_insert(inner, refer_inner.deref_mut(), refer_meta, conn));
    }
    Ok(())
}

fn do_cascade_insert<C>(a: &mut EntityInner,
                        b: &mut EntityInner,
                        a_b_meta: &FieldMeta,
                        conn: &mut C)
                        -> Result<(), Error>
    where C: GenericConnection
{
    if a_b_meta.has_cascade_insert() {
        if a_b_meta.is_refer_pointer() {
            try!(do_cascade_insert_pointer(a, b, a_b_meta, conn));
        } else if a_b_meta.is_refer_one_one() {
            try!(do_cascade_insert_one_one(a, b, a_b_meta, conn));
        }
    }
    Ok(())
}

fn do_cascade_insert_pointer<C>(a: &mut EntityInner,
                                b: &mut EntityInner,
                                a_b_meta: &FieldMeta,
                                conn: &mut C)
                                -> Result<(), Error>
    where C: GenericConnection
{
    // insert(b);
    try!(do_insert(b, conn));
    // a.b_id = b.id;
    let b_id_field = a_b_meta.get_pointer_id();
    let b_id = b.field_map.get("id").unwrap().clone();
    a.field_map.insert(b_id_field, b_id);
    Ok(())
}

fn do_cascade_insert_one_one<C>(a: &mut EntityInner,
                                b: &mut EntityInner,
                                a_b_meta: &FieldMeta,
                                conn: &mut C)
                                -> Result<(), Error>
    where C: GenericConnection
{
    // b.a_id = a.id;
    let a_id_field = a_b_meta.get_one_one_id();
    let a_id = a.field_map.get("id").unwrap().clone();
    b.field_map.insert(a_id_field, a_id);
    // insert(b);
    try!(do_insert(b, conn));
    Ok(())
}

fn do_update<C>(inner: &mut EntityInner, conn: &mut C) -> Result<(), Error>
    where C: GenericConnection
{
    // 遍历pointer
    for (refer_field, opt) in &inner.pointer_map {
        if opt.is_none() {
            // lazy load, 可以直接跳过，说明没有需要更新的
            continue;
        }
        let refer_inner_rc = opt.as_ref().unwrap().clone();
        // 拿到该引用对应的meta信息
        let refer_meta = inner.meta.field_map.get(refer_field).unwrap();
        // 判断是否需要级联更新
        if !refer_meta.has_cascade_update() {
            continue;
        }
        let refer_id_field = refer_meta.get_pointer_id();
        let mut refer_inner = refer_inner_rc.borrow_mut();
        try!(do_update(refer_inner.deref_mut(), conn));
        // 将refer的id写回原对象对应的refer_id, 在更新时似乎可以省略
        // let refer_id = refer_inner.field_map.get("id").unwrap().clone();
        // inner.field_map.insert(refer_id_field, refer_id);
    }
    // 更新自己
    try!(inner.do_update(conn));
    // 需要等a写入后才能写b，因为aid在b上，需要a先有id
    // update有可能导致insert，所以也一样
    for (refer_field, opt) in &inner.one_one_map {
        if opt.is_none() {
            // 说明没有更新到，基本可以不用管
            continue;
        }
        let refer_inner_rc = opt.as_ref().unwrap().clone();
        // 拿到该引用对应的meta信息
        let refer_meta = inner.meta.field_map.get(refer_field).unwrap();
        // 判断是否需要级联更新
        if !refer_meta.has_cascade_update() {
            continue;
        }
        let refer_id_field = refer_meta.get_one_one_id();
        let mut refer_inner = refer_inner_rc.borrow_mut();
        let self_id = inner.field_map.get("id").unwrap().clone();
        refer_inner.field_map.insert(refer_id_field.to_string(), self_id);
        try!(do_insert(refer_inner.deref_mut(), conn));
    }
    Ok(())
}

fn do_get<C>(inner: &mut EntityInner, conn: &mut C) -> Result<(), Error>
    where C: GenericConnection
{
    try!(inner.do_get(conn));
    Ok(())
}
