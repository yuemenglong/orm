#[macro_use]
use macros;

use mysql::Pool;
use mysql::Error;
use mysql::PooledConn;

use meta::OrmMeta;
use entity::Entity;
use session::Session;

pub struct DB {
    pub pool: Pool,
}

impl DB {
    pub fn rebuild(&self, meta: &OrmMeta) -> Result<u64, Error> {
        try!(self.drop_tables(meta));
        Ok(try!(self.create_tables(meta)))
    }
    pub fn create_tables(&self, meta: &OrmMeta) -> Result<u64, Error> {
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
    pub fn drop_tables(&self, meta: &OrmMeta) -> Result<u64, Error> {
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
    pub fn open_session(&self) -> Session {
        let conn = self.pool.get_conn();
        Session::new(conn.unwrap())
    }
    pub fn get_conn(&self) -> Result<PooledConn, Error> {
        self.pool.get_conn()
    }
    pub fn insert<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let session = self.open_session();
        session.insert(entity)
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<(), Error> {
        let session = self.open_session();
        session.update(entity)
    }
    pub fn delete<E: Entity>(&self, entity: E) -> Result<(), Error> {
        let session = self.open_session();
        session.delete(entity)
    }
    pub fn get<E: Entity>(&self, id: u64) -> Result<Option<E>, Error> {
        let session = self.open_session();
        session.get::<E>(id)
    }
}
