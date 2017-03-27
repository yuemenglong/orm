#[macro_use]
use macros;

use mysql::Pool;
use mysql::Error;
use mysql::Value;
use mysql::Row;

use mysql::prelude::GenericConnection;

use itertools::Itertools;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::DerefMut;

// use cond::Cond;
use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;

use meta;
use meta::OrmMeta;
use meta::EntityMeta;
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
        let ret = self.execute(entity, Cascade::Insert);
        entity.cascade_reset();
        ret
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let ret = self.execute(entity, Cascade::Update);
        entity.cascade_reset();
        ret
    }
    pub fn delete<E: Entity>(&self, entity: E) -> Result<(), Error> {
        let ret = self.execute(&entity, Cascade::Delete);
        entity.cascade_reset();
        ret
    }
    pub fn get<E: Entity>(&self, id: u64) -> Result<Option<E>, Error> {
        let mut conn = self.pool.get_conn();
        let session = Session::new(conn.unwrap());
        let vec = try!(session.select(id, E::meta(), E::orm_meta()));
        match vec.len() {
            0 => Ok(None),
            _ => Ok(Some(E::from_inner(vec[0].clone()))),
        }
    }
    pub fn execute<E: Entity>(&self, entity: &E, op: Cascade) -> Result<(), Error> {
        let mut conn = self.pool.get_conn();
        let session = Session::new(conn.unwrap());
        session.execute(entity.inner().clone(), op.clone())
    }
}

pub struct Session<C>
    where C: GenericConnection
{
    conn: RefCell<C>,
}

// execute insert update delete
impl<C> Session<C>
    where C: GenericConnection
{
    pub fn new(conn: C) -> Session<C> {
        Session { conn: RefCell::new(conn) }
    }
    pub fn execute(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
        if op == Cascade::NULL {
            return Ok(());
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
            let cache = mem::replace(&mut a_rc.borrow_mut().cache, Vec::new());
            try!(self.each_execute_refer(a_rc.clone(), &cache, op.clone()));
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

// select
impl<C> Session<C>
    where C: GenericConnection
{
    pub fn select(&self,
                  id: u64,
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
        tables.insert(0, meta.table_name.clone());
        let tables = tables.iter().map(|l| format!("\t{}", l)).collect::<Vec<_>>().join("\n");
        let cond = format!("\t{}.id = {}", &meta.table_name, id);
        let sql = format!("SELECT \n{} \nFROM \n{} \nWHERE \n{}", fields, tables, cond);
        println!("{}", sql);

        let mut conn = self.conn.borrow_mut();
        let query_result = try!(conn.query(sql));

        let mut map: HashMap<String, EntityInnerPointer> = HashMap::new();
        let mut vec = Vec::new();
        for row in query_result {
            let mut row = try!(row);
            match Self::take_entity(&mut row, table_alias, meta, orm_meta, &mut map) {
                Some(rc) => vec.push(rc), 
                None => {}
            }
        }
        let vec =
            vec.into_iter().unique_by(|rc| rc.borrow().get_id_u64().unwrap()).collect::<Vec<_>>();
        Ok(vec)
    }
    fn take_entity(mut row: &mut Row,
                   table_alias: &str,
                   meta: &'static EntityMeta,
                   orm_meta: &'static OrmMeta,
                   mut map: &mut HashMap<String, EntityInnerPointer>)
                   -> Option<EntityInnerPointer> {
        let mut a = EntityInner::default(meta, orm_meta);
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

        Self::take_entity_pointer(a_rc.clone(), &mut row, table_alias, &mut map);
        Self::take_entity_one_one(a_rc.clone(), &mut row, table_alias, &mut map);
        Self::take_entity_one_many(a_rc.clone(), &mut row, table_alias, &mut map);
        Self::take_entity_many_many(a_rc.clone(), &mut row, table_alias, &mut map);
        Some(a_rc)
    }
    fn take_entity_pointer(a_rc: EntityInnerPointer,
                           mut row: &mut Row,
                           table_alias: &str,
                           mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_pointer_fields() {
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match Self::take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => a.set_pointer(&a_b_field, Some(b_rc)),
                None => a.set_pointer(&a_b_field, None),
            }
        }
    }
    fn take_entity_one_one(a_rc: EntityInnerPointer,
                           mut row: &mut Row,
                           table_alias: &str,
                           mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_one_one_fields() {
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match Self::take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => a.set_one_one(&a_b_field, Some(b_rc)),
                None => a.set_one_one(&a_b_field, None),
            }
        }
    }
    fn take_entity_one_many(a_rc: EntityInnerPointer,
                            mut row: &mut Row,
                            table_alias: &str,
                            mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_one_many_fields() {
            let b_entity = a_b_meta.get_refer_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            match Self::take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
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
    fn take_entity_many_many(a_rc: EntityInnerPointer,
                             mut row: &mut Row,
                             table_alias: &str,
                             mut map: &mut HashMap<String, EntityInnerPointer>) {
        let mut a = a_rc.borrow_mut();
        for a_b_meta in a.meta.get_many_many_fields() {
            let b_entity = a_b_meta.get_refer_entity();
            let mid_entity = a_b_meta.get_many_many_middle_entity();
            let a_b_field = a_b_meta.get_field_name();
            let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
            let mid_meta = a.orm_meta.entity_map.get(&mid_entity).unwrap();
            let b_table_alias = format!("{}_{}", table_alias, a_b_field);
            let mid_table_alias = format!("{}__{}", table_alias, a_b_field);
            match Self::take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
                Some(b_rc) => {
                    let key = format!("MANY_MANY@{}_{}",
                                      b_table_alias,
                                      b_rc.borrow().get_id_u64().unwrap());
                    if !map.contains_key(&key) {
                        let mid_rc = Self::take_entity(&mut row,
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

#[derive(Debug, Default)]
struct SelectResult {
    id: u64,
    entity: Option<EntityInnerPointer>,
    vec: Vec<SelectResult>,
}
