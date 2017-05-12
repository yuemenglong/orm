use meta::Cascade;
use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;

use std::collections::HashMap;
use std::collections::HashSet;

use mysql::Error;
use mysql::Value;
use mysql::conn::GenericConnection;

use value::FieldValue;

// Execute::insert::<E>().update("sdf")
// Execute::insert::<E>().update(rc)

#[derive(Debug)]
pub struct Execute {
    cascade: Cascade,
    field_withs: Vec<(String, Execute)>,
    entity_withs: HashMap<u64, Execute>,
}

impl Execute {
    pub fn insert() -> Self {
        Execute {
            cascade: Cascade::Insert,
            field_withs: Vec::new(),
            entity_withs: HashMap::new(),
        }
    }
    pub fn update() -> Self {
        Execute {
            cascade: Cascade::Update,
            field_withs: Vec::new(),
            entity_withs: HashMap::new(),
        }
    }
    pub fn delete() -> Self {
        Execute {
            cascade: Cascade::Delete,
            field_withs: Vec::new(),
            entity_withs: HashMap::new(),
        }
    }
}

impl Execute {
    pub fn execute<E, C>(&self, conn: &mut C, entity: &E) -> Result<u64, Error>
        where C: GenericConnection,
              E: Entity
    {
        self.execute_inner(conn, entity.inner())
    }
    pub fn execute_inner<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
        where C: GenericConnection
    {
        self.execute_impl(conn, rc, &mut HashSet::new())
    }
    fn execute_impl<C>(&self,
                       conn: &mut C,
                       rc: EntityInnerPointer,
                       set: &mut HashSet<u64>)
                       -> Result<u64, Error>
        where C: GenericConnection
    {
        let r1 = try!(self.execute_pointer(conn, rc.clone(), set));
        let r2 = match self.cascade {
            Cascade::Insert => try!(self.execute_insert_self(conn, rc.clone(), set)),
            _ => unreachable!(),
        };
        let r3 = try!(self.execute_one_one(conn, rc.clone(), set));
        let r4 = try!(self.execute_one_many(conn, rc.clone(), set));
        Ok(r1 + r2 + r3 + r4)
    }
}

impl Execute {
    fn execute_insert_self<C>(&self,
                              conn: &mut C,
                              rc: EntityInnerPointer,
                              set: &mut HashSet<u64>)
                              -> Result<u64, Error>
        where C: GenericConnection
    {
        let table = rc.borrow().meta.table.clone();
        let valid_fields = rc.borrow()
            .meta
            .field_vec
            .iter()
            .filter(|&field| {
                let field_meta = rc.borrow().meta.field_map.get(field).unwrap();
                if rc.borrow().meta.is_id_auto() && field_meta.is_type_id() {
                    return false;
                }
                rc.borrow().field_map.get(field).is_some() && !field_meta.is_type_refer()
            })
            .collect::<Vec<_>>();
        let params = valid_fields.iter()
            .map(|&field| {
                (field.to_string(), rc.borrow().field_map.get(field).expect(&expect!()).as_value())
            })
            .collect::<Vec<_>>();
        let fields = valid_fields.iter()
            .map(|field| format!("`{}` = :{}", field, field))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("INSERT INTO `{}` SET {}", table, fields);
        log!("{}", sql);
        log!("{:?}", params);
        conn.prep_exec(sql, params).map(|res| {
            if rc.borrow().meta.is_id_auto() {
                rc.borrow_mut()
                    .field_map
                    .insert("id".to_string(),
                            FieldValue::from(Value::from(res.last_insert_id())));
            }
            res.affected_rows()
        })
    }
}

impl Execute {
    fn execute_pointer<C>(&self,
                          conn: &mut C,
                          rc: EntityInnerPointer,
                          set: &mut HashSet<u64>)
                          -> Result<u64, Error>
        where C: GenericConnection
    {
        self.field_withs
            .iter()
            .filter_map(|&(ref field, ref execute)| {
                // 是pointer
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_pointer() {
                    return None;
                }
                // 有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .and_then(|v| {
                        v.as_entity().map(|b_rc| {
                            //到这里说明是有值的，需要再判断下是否在实体上标注过
                            let addr = b_rc.borrow().get_addr();
                            match self.entity_withs.get(&addr){
                                None=> (field, execute, b_rc),
                                Some(spec_execute)=>(field, spec_execute, b_rc),
                            }
                        })
                })
            }).fold(Ok(0), |acc, (field, execute, b_rc)| {
                if acc.is_err() {
                    return acc;
                }
                let res = execute.execute_impl(conn, b_rc.clone(), set);
                if res.is_err() {
                    return res;
                }
                // Update 和 Delete都不需要
                if self.cascade == Cascade::Insert {
                    // a.b_id = b.id
                    let (left, right) = rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                    let b_id = b_rc.borrow().field_map.get(&right).map(|v| v.clone());
                    if b_id.is_some() {
                        rc.borrow_mut().field_map.insert(left, b_id.unwrap());
                    }
                }

                let acc = acc.unwrap() + res.unwrap();
                Ok(acc)
            })

    }
    fn execute_one_one<C>(&self,
                          conn: &mut C,
                          rc: EntityInnerPointer,
                          set: &mut HashSet<u64>)
                          -> Result<u64, Error>
        where C: GenericConnection
    {
        self.field_withs
            .iter()
            .filter_map(|&(ref field, ref execute)| {
                // 是one_one
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_one_one() {
                    return None;
                }
                // 有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .and_then(|v| {
                        v.as_entity().map(|b_rc| {
                            //到这里说明是有值的，需要再判断下是否在实体上标注过
                            let addr = b_rc.borrow().get_addr();
                            match self.entity_withs.get(&addr){
                                None=> (field, execute, b_rc),
                                Some(spec_execute)=>(field, spec_execute, b_rc),
                            }
                        })
                })
            })
            .fold(Ok(0), |acc, (field, execute, b_rc)| {
                if acc.is_err() {
                    return acc;
                }
                // Update 和 Delete都不需要
                if self.cascade == Cascade::Insert {
                    // b.a_id = a.id
                    let (left, right) = rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                    let a_id = rc.borrow().field_map.get(&left).map(|v| v.clone());
                    if a_id.is_some() {
                        b_rc.borrow_mut().field_map.insert(right, a_id.unwrap());
                    }
                }

                let res = execute.execute_impl(conn, b_rc.clone(), set);
                if res.is_err() {
                    return res;
                }
                let acc = acc.unwrap() + res.unwrap();
                Ok(acc)
            })
    }
    fn execute_one_many<C>(&self,
                           conn: &mut C,
                           rc: EntityInnerPointer,
                           set: &mut HashSet<u64>)
                           -> Result<u64, Error>
        where C: GenericConnection
    {
        self.field_withs
            .iter()
            .filter_map(|&(ref field, ref execute)| {
                // one_many
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_one_many() {
                    return None;
                }
                // 有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .map(|v| {
                        let vec = v.as_vec();
                        vec.into_iter()
                            .map(|b_rc| {
                                let addr = b_rc.borrow().get_addr();
                                match self.entity_withs.get(&addr) {
                                    None => (field, execute, b_rc),
                                    Some(spec_execute) => (field, spec_execute, b_rc),
                                }
                            })
                            .collect::<Vec<_>>()
                    })
            })
            .fold(Ok(0), |acc, vec| {
                if acc.is_err() {
                    return acc;
                }
                // Update 和 Delete都不需要
                if self.cascade == Cascade::Insert {
                    for &(field, _, ref b_rc) in vec.iter() {
                        // b.a_id = a.id
                        let (left, right) =
                            rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                        let a_id = rc.borrow().field_map.get(&left).map(|v| v.clone());
                        if a_id.is_some() {
                            b_rc.borrow_mut()
                                .field_map
                                .insert(right.to_string(), a_id.clone().unwrap());
                        }
                    }
                }

                // let res = vec.iter().fold(Ok(0), |acc, b_rc| {
                let res = vec.iter().fold(Ok(0), |acc, &(ref field, ref execute, ref b_rc)| {
                    if acc.is_err() {
                        return acc;
                    }
                    let res = execute.execute_impl(conn, b_rc.clone(), set);
                    if res.is_err() {
                        return res;
                    }
                    Ok(acc.unwrap() + res.unwrap())
                });
                if res.is_err() {
                    return res;
                }
                Ok(acc.unwrap() + res.unwrap())
            })
    }
}
