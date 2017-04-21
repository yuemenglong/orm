#[macro_use]
use macros;

use mysql::Pool;
use mysql::Error;
use mysql::PooledConn;

use meta::OrmMeta;
use entity::Entity;
use session::Session;

pub struct DB {
    pool: Pool,
    orm_meta: &'static OrmMeta,
}

impl DB {
    pub fn new(pool: Pool, orm_meta: &'static OrmMeta) -> Self {
        DB {
            pool: pool,
            orm_meta: orm_meta,
        }
    }
    pub fn rebuild(&self) -> Result<u64, Error> {
        try!(self.drop_tables());
        Ok(try!(self.create_tables()))
    }
    pub fn create_tables(&self) -> Result<u64, Error> {
        let mut ret = 0;
        for entity_meta in self.orm_meta.get_entities().iter() {
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
    pub fn drop_tables(&self) -> Result<u64, Error> {
        let mut ret = 0;
        for entity_meta in self.orm_meta.get_entities().iter() {
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
    pub fn open_session(&self) -> Session {
        let conn = self.pool.get_conn();
        Session::new(conn.unwrap())
    }
    pub fn insert<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let session = self.open_session();
        session.insert(entity)
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let session = self.open_session();
        session.update(entity)
    }
    pub fn delete<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let session = self.open_session();
        session.delete(entity)
    }
    pub fn get<E: Entity>(&self, id: u64) -> Result<Option<E>, Error> {
        let session = self.open_session();
        session.get::<E>(id)
    }
}
