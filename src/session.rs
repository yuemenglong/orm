#[macro_use]
use macros;

use mysql::Pool;
use mysql::Error;
use mysql::Value;
use mysql::Row;

use mysql::PooledConn;

use itertools::Itertools;

use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::ops::DerefMut;

use cond::Cond;
use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;

use meta;
use meta::OrmMeta;
use meta::EntityMeta;
use meta::FieldMeta;
use meta::Cascade;

#[derive(Clone, Copy, PartialEq)]
pub enum SessionStatus {
    Normal,
    Closed,
    Insert,
    Update,
    Select,
    Delete,
}

pub struct Session {
    conn: Rc<RefCell<PooledConn>>,
    cache: Rc<RefCell<Vec<EntityInnerPointer>>>,
    status: Rc<Cell<SessionStatus>>,
}
impl Session {
    pub fn new(conn: PooledConn) -> Session {
        Session {
            conn: Rc::new(RefCell::new(conn)),
            cache: Rc::new(RefCell::new(Vec::new())),
            status: Rc::new(Cell::new(SessionStatus::Normal)),
        }
    }
}

impl Session {
    pub fn execute(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        let status = match op {
            Cascade::NULL => SessionStatus::Normal,
            Cascade::Insert => SessionStatus::Insert,
            Cascade::Update => SessionStatus::Update,
            Cascade::Delete => SessionStatus::Delete,
        };
        let old = self.status.get();
        self.status.set(status);
        let result = self.execute_impl(a_rc, op);
        self.status.set(old);
        return result;
    }
    pub fn select(&self,
                  cond:&Cond,
                  meta: &'static EntityMeta,
                  orm_meta: &'static OrmMeta)
                  -> Result<Vec<EntityInnerPointer>, Error> {
        let old = self.status.get();
        self.status.set(SessionStatus::Select);
        let result = self.select_impl(cond, meta, orm_meta);
        self.status.set(old);
        return result;
    }
    pub fn clone(&self) -> Session {
        Session {
            conn: self.conn.clone(),
            cache: self.cache.clone(),
            status: self.status.clone(),
        }
    }
    pub fn close(&self) {
        self.close_impl();
    }
    pub fn status(&self) -> SessionStatus {
        self.status.get()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}

impl Session {
    fn close_impl(&self) -> Result<(), Error> {
        self.status.set(SessionStatus::Closed);
        try!(self.batch_impl(self.cache.borrow().deref(), Cascade::Update));
        for rc in self.cache.borrow().iter() {
            rc.borrow_mut().clear_session();
        }
        self.cache.borrow_mut().clear();
        Ok(())
    }
}

// execute insert update delete
impl Session {
    fn batch_impl(&self, vec: &Vec<EntityInnerPointer>, op: Cascade) -> Result<(), Error> {
        vec.iter().fold(Ok(()), |result, rc| {
            if result.is_err() {
                return result;
            }
            return self.execute_impl(rc.clone(), op.clone());
        })
    }
    fn execute_impl(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        if op == Cascade::NULL {
            return Ok(());
        }
        {
            // 一上来就设为持久态
            a_rc.borrow_mut().set_session(self.clone());
        }
        {
            // pointer
            let pointer_fields = Self::map_to_vec(&a_rc.borrow().pointer_map);
            try!(self.each_execute_refer(a_rc.clone(), &pointer_fields, op.clone()));
            for (field, b_rc) in pointer_fields {
                a_rc.borrow_mut().set_pointer(&field, Some(b_rc));
            }
        }
        {
            // self
            try!(self.execute_self(a_rc.clone(), op.clone()));
        }
        {
            // one one
            let one_one_fields = Self::map_to_vec(&a_rc.borrow().one_one_map);
            for &(ref field, ref b_rc) in one_one_fields.iter() {
                a_rc.borrow_mut().set_one_one(field, Some(b_rc.clone()));
            }
            try!(self.each_execute_refer(a_rc.clone(), &one_one_fields, op.clone()));
        }
        {
            // one many
            let one_many_fields =
                a_rc.borrow().one_many_map.clone().into_iter().collect::<Vec<_>>();
            for &(ref field, ref vec) in one_many_fields.iter() {
                a_rc.borrow_mut().set_one_many(field, vec.clone());
            }
            try!(self.each_execute_refer_vec(a_rc.clone(), &one_many_fields, op.clone()));
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
            try!(self.each_execute_refer_vec(a_rc.clone(), &many_many_fields, op.clone()));
            // 重新set回去
            for (ref field, ref b_vec) in many_many_fields {
                a_rc.borrow_mut().set_many_many(field, b_vec.clone());
            }
        }
        {
            // 中间表
            let middle_fields = a_rc.borrow()
                .many_many_map
                .clone()
                .into_iter()
                .map(|(field, pair_vec)| {
                    let m_vec = pair_vec.into_iter()
                        .filter_map(|(m_opt, _)| m_opt)
                        .collect::<Vec<_>>();
                    (field, m_vec)
                })
                .collect::<Vec<_>>();
            try!(self.each_execute_refer_vec(a_rc.clone(), &middle_fields, op.clone()));
        }
        {
            // cache
            // let cache = mem::replace(&mut a_rc.borrow_mut().cache, Vec::new());
            // try!(self.each_execute_refer(a_rc.clone(), &cache, op.clone()));
        }
        Ok(())
    }
    fn execute_self(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        match op {
            Cascade::Insert => a_rc.borrow_mut().do_insert(self.conn.borrow_mut().deref_mut()),
            Cascade::Update => a_rc.borrow_mut().do_update(self.conn.borrow_mut().deref_mut()),
            Cascade::Delete => a_rc.borrow_mut().do_delete(self.conn.borrow_mut().deref_mut()),
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
    fn each_execute_refer(&self,
                          a_rc: EntityInnerPointer,
                          vec: &Vec<(String, EntityInnerPointer)>,
                          op: Cascade)
                          -> Result<(), Error> {
        for &(ref field, ref b_rc) in vec.iter() {
            try!(self.execute_refer(a_rc.clone(), b_rc.clone(), field, op.clone()));
        }
        Ok(())
    }
    fn each_execute_refer_vec(&self,
                              a_rc: EntityInnerPointer,
                              vecs: &Vec<(String, Vec<EntityInnerPointer>)>,
                              op: Cascade)
                              -> Result<(), Error> {
        for &(ref field, ref vec) in vecs.iter() {
            let pairs =
                vec.iter().map(|b_rc| (field.to_string(), b_rc.clone())).collect::<Vec<_>>();
            try!(self.each_execute_refer(a_rc.clone(), &pairs, op.clone()));
        }
        Ok(())
    }
    fn execute_refer(&self,
                     a_rc: EntityInnerPointer,
                     b_rc: EntityInnerPointer,
                     field: &str,
                     op: Cascade)
                     -> Result<(), Error> {
        let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), field, op);
        Self::clear_cascade(b_rc.clone());
        self.execute(b_rc, cascade)
    }
    fn clear_cascade(b_rc: EntityInnerPointer) -> Option<Cascade> {
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
        } else if a_b_meta.get_refer_rt_cascade().is_some() {
            return a_b_meta.get_refer_rt_cascade().clone().unwrap();
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

// select
impl Session {
    fn select_impl(&self,
                   cond:&Cond,
                   meta: &'static EntityMeta,
                   orm_meta: &'static OrmMeta)
                   -> Result<Vec<EntityInnerPointer>, Error> {
        let table_alias = &meta.table_name;
        let mut tables = Vec::new();
        let mut fields = Vec::new();
        Self::gen_sql(&meta.entity_name,
                      &table_alias,
                      orm_meta,
                      &mut tables,
                      &mut fields);

        let fields = fields.into_iter()
            .map(|vec| vec.iter().map(|l| format!("\t{}", l)).collect::<Vec<_>>().join(",\n"))
            .collect::<Vec<_>>()
            .join(",\n\n");
        tables.insert(0, format!("{} AS {}", &meta.table_name, table_alias));
        let tables = tables.iter().map(|l| format!("\t{}", l)).collect::<Vec<_>>().join("\n");
        // let cond = format!("\t{}.id = {}", &meta.table_name, id);
        let sql = format!("SELECT \n{} \nFROM \n{} \nWHERE \n\t{}", fields, tables, cond.to_sql());
        println!("{}", sql);
        println!("{:?}", cond.to_params());

        let mut conn = self.conn.borrow_mut();
        let query_result = try!(conn.prep_exec(sql, cond.to_params()));

        let mut map: HashMap<String, EntityInnerPointer> = HashMap::new();
        let mut vec = Vec::new();
        for row in query_result {
            let mut row = try!(row);
            match self.take_entity(&mut row, table_alias, meta, orm_meta, &mut map) {
                Some(rc) => vec.push(rc), 
                None => {}
            }
        }
        let vec =
            vec.into_iter().unique_by(|rc| rc.borrow().get_id_u64().unwrap()).collect::<Vec<_>>();
        Ok(vec)
    }
    fn take_entity(&self,
                   mut row: &mut Row,
                   table_alias: &str,
                   meta: &'static EntityMeta,
                   orm_meta: &'static OrmMeta,
                   mut map: &mut HashMap<String, EntityInnerPointer>)
                   -> Option<EntityInnerPointer> {
        // 关系是空的，这样才能判断出lazy的情况
        let mut a = EntityInner::new(meta, orm_meta);
        // 一上来就设为持久态
        a.set_session(self.clone());
        a.set_values(&mut row, &table_alias);
        let id = a.get_id_u64();
        if id.is_none() {
            return None;
        }
        let id = id.unwrap();
        let key = format!("{}_{}", table_alias, id);
        let a_rc = match map.get(&key) {
            Some(rc) => rc.clone(),
            None => Rc::new(RefCell::new(a)),
        };
        map.insert(key, a_rc.clone());

        self.take_entity_pointer(a_rc.clone(), &mut row, table_alias, &mut map);
        self.take_entity_one_one(a_rc.clone(), &mut row, table_alias, &mut map);
        self.take_entity_one_many(a_rc.clone(), &mut row, table_alias, &mut map);
        self.take_entity_many_many(a_rc.clone(), &mut row, table_alias, &mut map);
        Some(a_rc)
    }
    fn take_entity_pointer(&self,
                           a_rc: EntityInnerPointer,
                           mut row: &mut Row,
                           table_alias: &str,
                           mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_pointer_fields() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => a.set_pointer(&a_b_field, Some(b_rc)),
                None => a.set_pointer(&a_b_field, None),
            }
        }
    }
    fn take_entity_one_one(&self,
                           a_rc: EntityInnerPointer,
                           mut row: &mut Row,
                           table_alias: &str,
                           mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_one_one_fields() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => a.set_one_one(&a_b_field, Some(b_rc)),
                None => a.set_one_one(&a_b_field, None),
            }
        }
    }
    fn take_entity_one_many(&self,
                            a_rc: EntityInnerPointer,
                            mut row: &mut Row,
                            table_alias: &str,
                            mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_one_many_fields() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => {
                    let key = format!("ONE_MANY@{}_{}",
                                      b_table_alias,
                                      b_rc.borrow().get_id_u64().unwrap());
                    if !map.contains_key(&key) {
                        a.push_one_many(&a_b_field, b_rc.clone());
                    }
                    map.entry(key).or_insert(b_rc);
                }
                None => {}
            }
        }
    }
    fn take_entity_many_many(&self,
                             a_rc: EntityInnerPointer,
                             mut row: &mut Row,
                             table_alias: &str,
                             mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_many_many_fields() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            let b_entity = a_b_meta.get_refer_entity();
            let mid_entity = a_b_meta.get_many_many_middle_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let mid_meta = a.orm_meta.entity_map.get(&mid_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            let mid_table_alias = format!("{}__{}", table_alias, a_b_field);
            match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => {
                    let key = format!("MANY_MANY@{}_{}",
                                      b_table_alias,
                                      b_rc.borrow().get_id_u64().unwrap());
                    if !map.contains_key(&key) {
                        let mid_rc = self.take_entity(&mut row,
                                         &mid_table_alias,
                                         &mid_meta,
                                         &a.orm_meta,
                                         &mut map)
                            .unwrap();
                        a.push_many_many(&a_b_field, (mid_rc, b_rc.clone()));
                    }
                    map.entry(key).or_insert(b_rc);
                }
                None => {}
            }
        }
    }
    fn gen_sql(entity: &str,
               table_alias: &str,
               orm_meta: &'static OrmMeta,
               mut tables: &mut Vec<String>,
               mut columns: &mut Vec<Vec<String>>) {
        let meta = orm_meta.entity_map.get(entity).unwrap();
        let self_columns = Self::gen_sql_columns(meta, table_alias);
        columns.push(self_columns);

        Self::gen_sql_pointer(table_alias, meta, orm_meta, tables, columns);
        Self::gen_sql_one_one(table_alias, meta, orm_meta, tables, columns);
        Self::gen_sql_one_many(table_alias, meta, orm_meta, tables, columns);
        Self::gen_sql_many_many(table_alias, meta, orm_meta, tables, columns);
    }
    fn gen_sql_pointer(table_alias: &str,
                       meta: &'static EntityMeta,
                       orm_meta: &'static OrmMeta,
                       mut tables: &mut Vec<String>,
                       mut columns: &mut Vec<Vec<String>>) {
        for a_b_meta in meta.get_pointer_fields().into_iter() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            // a join b on a.b_id = b.id
            let a_b_field = a_b_meta.get_field_name();
            let b_entity = a_b_meta.get_refer_entity();
            let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_name = &b_meta.table_name;
            let a_b_id_field = a_b_meta.get_pointer_id();
            let a_b_id_meta = meta.field_map.get(&a_b_id_field).unwrap();
            let a_b_id_column = a_b_id_meta.get_column_name();
            let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
            let join_table = format!("LEFT JOIN {} AS {} ON {}.{} = {}.id",
                                     &b_table_name,
                                     &b_table_alias,
                                     &table_alias,
                                     &a_b_id_column,
                                     &b_table_alias);
            tables.push(join_table);
            Self::gen_sql(&b_entity,
                          &b_table_alias,
                          orm_meta,
                          &mut tables,
                          &mut columns);
        }
    }
    fn gen_sql_one_one(table_alias: &str,
                       meta: &'static EntityMeta,
                       orm_meta: &'static OrmMeta,
                       mut tables: &mut Vec<String>,
                       mut columns: &mut Vec<Vec<String>>) {
        for a_b_meta in meta.get_one_one_fields().into_iter() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            // a join b on a.id = b.a_id
            let a_b_field = a_b_meta.get_field_name();
            let b_entity = a_b_meta.get_refer_entity();
            let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_name = &b_meta.table_name;
            let b_a_id_field = a_b_meta.get_one_one_id();
            let b_a_id_meta = b_meta.field_map.get(&b_a_id_field).unwrap();
            let b_a_id_column = b_a_id_meta.get_column_name();
            let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
            let join_table = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
                                     &b_table_name,
                                     &b_table_alias,
                                     &table_alias,
                                     &b_table_alias,
                                     &b_a_id_column);
            tables.push(join_table);
            Self::gen_sql(&b_entity,
                          &b_table_alias,
                          orm_meta,
                          &mut tables,
                          &mut columns);
        }
    }
    fn gen_sql_one_many(table_alias: &str,
                        meta: &'static EntityMeta,
                        orm_meta: &'static OrmMeta,
                        mut tables: &mut Vec<String>,
                        mut columns: &mut Vec<Vec<String>>) {
        for a_b_meta in meta.get_one_many_fields().into_iter() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            // a join b on a.id = b.a_id
            let a_b_field = a_b_meta.get_field_name();
            let b_entity = a_b_meta.get_refer_entity();
            let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_name = &b_meta.table_name;
            let b_a_id_field = a_b_meta.get_one_many_id();
            let b_a_id_meta = b_meta.field_map.get(&b_a_id_field).unwrap();
            let b_a_id_column = b_a_id_meta.get_column_name();
            let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
            let join_table = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
                                     &b_table_name,
                                     &b_table_alias,
                                     &table_alias,
                                     &b_table_alias,
                                     &b_a_id_column);
            tables.push(join_table);
            Self::gen_sql(&b_entity,
                          &b_table_alias,
                          orm_meta,
                          &mut tables,
                          &mut columns);
        }
    }
    fn gen_sql_many_many(table_alias: &str,
                         meta: &'static EntityMeta,
                         orm_meta: &'static OrmMeta,
                         mut tables: &mut Vec<String>,
                         mut columns: &mut Vec<Vec<String>>) {
        for a_b_meta in meta.get_many_many_fields().into_iter() {
            if !a_b_meta.is_fetch_eager() {
                continue;
            }
            // a join a_b on a.id = a_b.a_id join b on a_b.b_id = b.id
            let a_b_field = a_b_meta.get_field_name();
            let b_entity = a_b_meta.get_refer_entity();
            let mid_entity = a_b_meta.get_many_many_middle_entity();
            let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
            let mid_meta = orm_meta.entity_map.get(&mid_entity).unwrap();
            let b_table_name = &b_meta.table_name;
            let mid_table_name = &mid_meta.table_name;
            let mid_a_id_field = a_b_meta.get_many_many_id();
            let mid_b_id_field = a_b_meta.get_many_many_refer_id();
            let mid_a_id_meta = mid_meta.field_map.get(&mid_a_id_field).unwrap();
            let mid_b_id_meta = mid_meta.field_map.get(&mid_b_id_field).unwrap();
            let mid_a_id_column = mid_a_id_meta.get_column_name();
            let mid_b_id_column = mid_b_id_meta.get_column_name();
            let mid_table_alias = format!("{}__{}", &table_alias, &a_b_field);
            let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
            let join_mid = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
                                   &mid_table_name,
                                   &mid_table_alias,
                                   &table_alias,
                                   &mid_table_alias,
                                   &mid_a_id_column);
            let join_b = format!("LEFT JOIN {} AS {} ON {}.{} = {}.id",
                                 &b_table_name,
                                 &b_table_alias,
                                 &mid_table_alias,
                                 &mid_b_id_column,
                                 &b_table_alias);
            tables.push(join_mid);
            Self::gen_sql(&mid_entity,
                          &mid_table_alias,
                          orm_meta,
                          &mut tables,
                          &mut columns);
            tables.push(join_b);
            Self::gen_sql(&b_entity,
                          &b_table_alias,
                          orm_meta,
                          &mut tables,
                          &mut columns);
        }
    }
    fn gen_sql_columns(meta: &'static EntityMeta, table_alias: &str) -> Vec<String> {
        let table_name = &meta.table_name;
        let entity_name = &meta.entity_name;
        meta.get_non_refer_fields()
            .iter()
            .map(|field_meta| {
                let column_name = field_meta.get_column_name();
                let field_name = field_meta.get_field_name();
                format!("{}.{} as {}${}",
                        &table_alias,
                        &column_name,
                        &table_alias,
                        &field_name)
            })
            .collect()
    }
}
