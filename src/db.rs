use mysql::Pool;
use mysql::Error;
// use std::cell::RefCell;

// use cond::Cond;
use entity::Entity;

pub struct DB {
    pub pool: Pool,
}


impl DB {
    pub fn create_table<E: Entity>(&self) -> Result<u64, Error> {
        let sql = E::sql_create_table();
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn drop_table<E: Entity>(&self) -> Result<u64, Error> {
        let sql = E::sql_drop_table();
        println!("{}", sql);
        let res = self.pool.prep_exec(sql, ());
        match res {
            Ok(res) => Ok(res.affected_rows()),
            Err(err) => Err(err),
        }
    }
    pub fn insert<E: Entity + Clone>(&self, entity: &E) -> Result<E, Error> {
        let sql = E::sql_insert();
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
//     pub fn update<E: Entity>(&self, entity: &E) -> Result<u64, Error> {
//         let sql = format!("UPDATE `{}` SET {} WHERE `id` = {}",
//                           E::get_name(),
//                           E::get_prepare(),
//                           entity.get_id().unwrap());
//         println!("{}", sql);
//         let res = self.pool.prep_exec(sql, entity.get_params());
//         match res {
//             Ok(res) => Ok(res.affected_rows()),
//             Err(err) => Err(err),
//         }
//     }
//     pub fn get<E: Entity>(&self, id: u64) -> Result<Option<E>, Error> {
//         let sql = format!("SELECT {} FROM `{}` WHERE `id` = {}",
//                           E::get_field_list(),
//                           E::get_name(),
//                           id);
//         println!("{}", sql);
//         let res = self.pool.first_exec(sql, ());
//         match res {
//             Ok(option) => Ok(option.map(|row| E::from_row(row))),
//             Err(err) => Err(err),
//         }
//     }
//     pub fn delete<E: Entity>(&self, entity: E) -> Result<u64, Error> {
//         let sql = format!("DELETE FROM `{}` WHERE `id` = {}",
//                           E::get_name(),
//                           entity.get_id().unwrap());
//         println!("{}", sql);
//         let res = self.pool.prep_exec(sql, ());
//         match res {
//             Ok(res) => Ok(res.affected_rows()),
//             Err(err) => Err(err),
//         }
//     }
//     // pub fn select<'a, E: Entity>(&'a self, conds: Vec<Cond>) -> SelectBuilder<'a, E> {
//     //     SelectBuilder::<'a, E> {
//     //         pool: &self.pool,
//     //         conds: RefCell::new(conds),
//     //         phantom: PhantomData,
//     //     }
//     // }
}
