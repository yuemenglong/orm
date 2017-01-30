use mysql::Pool;
use mysql::Error;
use mysql::Value;
use meta;
use std::fmt::Debug;

// use cond::Cond;
use entity::Entity;
use sql::*;

pub struct DB {
    pub pool: Pool,
}

impl DB {
    pub fn create_tables(&self, meta: &meta::OrmMeta) -> Result<u64, Error> {
        let mut ret = 0;
        for entity_meta in meta.entities.iter() {
            let sql = sql_create_table(entity_meta);
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
            let sql = sql_drop_table(entity_meta);
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
        let sql = sql_create_table(E::get_meta());
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn drop_table<E: Entity>(&self) -> Result<u64, Error> {
        let sql = sql_drop_table(E::get_meta());
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn insert<E: Entity + Clone>(&self, entity: &E) -> Result<E, Error> {
        let sql = sql_insert(E::get_meta());
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, entity.get_params());
        match res {
            Ok(res) => {
                let mut ret = (*entity).clone();
                ret.set_id(res.last_insert_id());
                Ok(ret)
            }
            Err(err) => Err(err),
        }
    }
    pub fn update<E: Entity>(&self, entity: &E) -> Result<u64, Error> {
        let sql = sql_update(E::get_meta());
        println!("{}", sql);
        let mut params = entity.get_params();
        params.push(("id".to_string(), Value::from(entity.get_id())));
        let res = self.pool.prep_exec(sql, params);
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn get<E: Entity + Default>(&self, id: u64) -> Result<Option<E>, Error> {
        let sql = sql_get(E::get_meta());
        println!("{}", sql);
        let res = self.pool.first_exec(sql, vec![("id", id)]);
        if let Err(err) = res {
            return Err(err);
        }
        let option = res.unwrap();
        if let None = option {
            return Ok(None);
        }
        let mut row = option.unwrap();
        let mut entity = E::default();
        // println!("{:?}", row.get::<u64, &str>("attr_id"));
        entity.set_values(&mut row, "");
        Ok(Some(entity))
    }
    pub fn delete<E: Entity>(&self, entity: E) -> Result<u64, Error> {
        let sql = sql_delete(E::get_meta());
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
