use mysql::Pool;
use mysql::Error;
use mysql::Value;

use mysql::prelude::GenericConnection;
use meta;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;

// use cond::Cond;
use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;
use meta::FieldMeta;
use meta::Cascade;

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
        for entity_meta in meta.get_entities().iter() {
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
        for entity_meta in meta.get_entities().iter() {
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
        let ret = self.handle(entity, Cascade::Insert);
        entity.cascade_reset();
        ret
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let ret = self.handle(entity, Cascade::Update);
        entity.cascade_reset();
        ret
    }
    pub fn delete<E: Entity>(&self, entity: E) -> Result<(), Error> {
        let ret = self.handle(&entity, Cascade::Delete);
        entity.cascade_reset();
        ret
    }
    pub fn get<E: Entity>(&self, id: u64) -> Result<E, Error> {
        let mut inner = EntityInner::default(E::meta(), E::orm_meta());
        inner.field_map.insert("id".to_string(), Value::from(id));
        try!(do_get(&mut inner, self.pool.get_conn().as_mut().unwrap()));
        Ok(E::new(Rc::new(RefCell::new(inner))))
    }
    pub fn handle<E: Entity>(&self, entity: &E, op: Cascade) -> Result<(), Error> {
        let mut conn = self.pool.get_conn();
        let mut session = Session::new(conn.as_mut().unwrap());
        session.handle(entity.inner().clone(), op.clone())
    }
}

pub struct Session<'a, C>
    where C: GenericConnection + 'a
{
    conn: &'a mut C,
}

impl<'a, C> Session<'a, C>
    where C: GenericConnection + 'a
{
    pub fn new(conn: &'a mut C) -> Session<'a, C> {
        Session { conn: conn }
    }
    pub fn handle(&mut self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        if op == Cascade::NULL {
            return Ok(());
        }
        {
            // pointer
            let pointer_fields = Self::map_to_vec(&a_rc.borrow().pointer_map);
            try!(self.each_handle_refer(a_rc.clone(), &pointer_fields, op.clone()));
            for (field, b_rc) in pointer_fields {
                a_rc.borrow_mut().set_pointer(&field, Some(b_rc));
            }
        }
        {
            // self
            try!(self.handle_self(a_rc.clone(), op.clone()));
        }
        {
            // one one
            let one_one_fields = Self::map_to_vec(&a_rc.borrow().one_one_map);
            for &(ref field, ref b_rc) in one_one_fields.iter() {
                a_rc.borrow_mut().set_one_one(field, Some(b_rc.clone()));
            }
            try!(self.each_handle_refer(a_rc.clone(), &one_one_fields, op.clone()));
        }
        {
            // one many
            let one_many_fields =
                a_rc.borrow().one_many_map.clone().into_iter().collect::<Vec<_>>();
            for &(ref field, ref vec) in one_many_fields.iter() {
                a_rc.borrow_mut().set_one_many(field, vec.clone());
            }
            try!(self.each_handle_refer_vec(a_rc.clone(), &one_many_fields, op.clone()));
        }
        {
            // many many
            // 实体表
            let many_many_fields = a_rc.borrow()
                .many_many_map
                .clone()
                .into_iter()
                .map(|(field, pair_vec)| {
                    let b_vec = pair_vec.into_iter().map(|(_, b_rc)| b_rc).collect::<Vec<_>>();
                    (field, b_vec)
                })
                .collect::<Vec<_>>();
            try!(self.each_handle_refer_vec(a_rc.clone(), &many_many_fields, op.clone()));
            for (ref field, ref b_vec) in many_many_fields {
                a_rc.borrow_mut().set_many_many(field, b_vec.clone());
            }
            // 中间表
            let middle_fields = a_rc.borrow()
                .many_many_map
                .clone()
                .into_iter()
                .map(|(field, pair_vec)| {
                    let m_vec = pair_vec.into_iter().map(|(m_rc, _)| m_rc).collect::<Vec<_>>();
                    (field, m_vec)
                })
                .collect::<Vec<_>>();
            try!(self.each_handle_refer_vec(a_rc.clone(), &middle_fields, op.clone()));
        }
        {
            // cache
            let cache = mem::replace(&mut a_rc.borrow_mut().cache, Vec::new());
            try!(self.each_handle_refer(a_rc.clone(), &cache, op.clone()));
        }
        Ok(())
    }
    fn handle_self(&mut self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        match op {
            Cascade::Insert => a_rc.borrow_mut().do_insert(self.conn),
            Cascade::Update => a_rc.borrow_mut().do_update(self.conn),
            Cascade::Delete => a_rc.borrow_mut().do_delete(self.conn),
            Cascade::NULL => Ok(()),
        }
    }
    fn map_to_vec(map: &HashMap<String, Option<EntityInnerPointer>>)
                  -> Vec<(String, EntityInnerPointer)> {
        map.iter()
            .map(|(field, opt)| opt.clone().map(|b_rc| (field.to_string(), b_rc.clone())))
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect::<Vec<_>>()
    }
    fn each_handle_refer(&mut self,
                         a_rc: EntityInnerPointer,
                         vec: &Vec<(String, EntityInnerPointer)>,
                         op: Cascade)
                         -> Result<(), Error> {
        for &(ref field, ref b_rc) in vec.iter() {
            try!(self.handle_refer(a_rc.clone(), b_rc.clone(), field, op.clone()));
        }
        Ok(())
    }
    fn each_handle_refer_vec(&mut self,
                             a_rc: EntityInnerPointer,
                             vecs: &Vec<(String, Vec<EntityInnerPointer>)>,
                             op: Cascade)
                             -> Result<(), Error> {
        for &(ref field, ref vec) in vecs.iter() {
            let pairs =
                vec.iter().map(|b_rc| (field.to_string(), b_rc.clone())).collect::<Vec<_>>();
            try!(self.each_handle_refer(a_rc.clone(), &pairs, op.clone()));
        }
        Ok(())
    }
    fn handle_refer(&mut self,
                    a_rc: EntityInnerPointer,
                    b_rc: EntityInnerPointer,
                    field: &str,
                    op: Cascade)
                    -> Result<(), Error> {
        let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), field, op);
        Self::take_cascade(b_rc.clone());
        self.handle(b_rc, cascade)
    }
    fn take_cascade(b_rc: EntityInnerPointer) -> Option<Cascade> {
        mem::replace(&mut b_rc.borrow_mut().cascade, Some(Cascade::NULL))
    }
    fn calc_cascade(a_rc: EntityInnerPointer,
                    b_rc: EntityInnerPointer,
                    field: &str,
                    op: Cascade)
                    -> Cascade {
        // 1. 对象动态级联
        // 2. 配置动态级联
        // 3. 配置静态级联
        let a = a_rc.borrow();
        let a_b_meta = a.meta.field_map.get(field).unwrap();
        if b_rc.borrow().cascade.is_some() {
            return b_rc.borrow().cascade.unwrap().clone();
        } else if a_b_meta.get_refer_cascade().is_some() {
            return a_b_meta.get_refer_cascade().clone().unwrap();
        } else if a_b_meta.has_cascade_insert() && op == Cascade::Insert {
            return Cascade::Insert;
        } else if a_b_meta.has_cascade_update() && op == Cascade::Update {
            return Cascade::Update;
        } else if a_b_meta.has_cascade_delete() && op == Cascade::Delete {
            return Cascade::Delete;
        } else {
            return Cascade::NULL;
        }
    }
}

fn do_get<C>(inner: &mut EntityInner, conn: &mut C) -> Result<(), Error>
    where C: GenericConnection
{
    try!(inner.do_get(conn));
    Ok(())
}
