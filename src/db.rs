#[macro_use]
use macros;

use mysql::Pool;
use mysql::Error;

use itertools::Itertools;

use entity::Entity;

use meta;
use meta::OrmMeta;
use meta::Cascade;
use cond::Cond;
use session::Session;

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
        let mut cond = Cond::new::<E>();
        let vec = try!(session.select(cond.id(id), E::meta(), E::orm_meta()));
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
