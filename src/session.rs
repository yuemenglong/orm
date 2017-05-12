// #[macro_use]
// use macros;

// use mysql::Error;
// use mysql::Row;
// use mysql::conn::GenericConnection;

// use itertools::Itertools;

// use std::rc::Rc;
// use std::cell::Cell;
// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::ops::DerefMut;

// use cond::Cond;
// use entity::Entity;
// use entity::EntityInner;
// use entity::EntityInnerPointer;
// use select::Select;

// use meta::OrmMeta;
// use meta::EntityMeta;
// use meta::Cascade;

// #[derive(Clone, Copy, PartialEq)]
// pub enum SessionStatus {
//     Normal,
//     Closed,
// }

// // impl From<Cascade> for SessionStatus {
// //     fn from(c: Cascade) -> SessionStatus {
// //         match c {
// //             Cascade::NULL => SessionStatus::Normal,
// //             Cascade::Insert => SessionStatus::Insert,
// //             Cascade::Update => SessionStatus::Update,
// //             Cascade::Delete => SessionStatus::Delete,
// //         }
// //     }
// // }

// pub struct Session {
//     conn: Rc<RefCell<PooledConn>>,
//     cache: Rc<RefCell<Vec<EntityInnerPointer>>>,
//     status: Rc<Cell<SessionStatus>>,
// }
// impl Session {
//     pub fn new(conn: PooledConn) -> Session {
//         Session {
//             conn: Rc::new(RefCell::new(conn)),
//             cache: Rc::new(RefCell::new(Vec::new())),
//             status: Rc::new(Cell::new(SessionStatus::Normal)),
//         }
//     }
//     pub fn insert<E>(&self, entity: &E) -> Result<(), Error>
//         where E: Entity
//     {
//         self.execute_inner(entity.inner(), Cascade::Insert)
//     }
//     pub fn update<E>(&self, entity: &E) -> Result<(), Error>
//         where E: Entity
//     {
//         self.execute_inner(entity.inner(), Cascade::Update)
//     }
//     pub fn delete<E>(&self, entity: &E) -> Result<(), Error>
//         where E: Entity
//     {
//         self.execute_inner(entity.inner(), Cascade::Delete)
//     }
//     pub fn select<E>(&self, cond: &Cond) -> Result<Vec<E>, Error>
//         where E: Entity
//     {
//         self.select_inner(E::meta(), E::orm_meta(), cond).map(|vec| {
//             vec.into_iter()
//                 .map(E::from_inner)
//                 .collect::<Vec<E>>()
//         })
//     }
//     pub fn query<E>(&self, select: &Select) -> Result<Vec<E>, Error>
//         where E: Entity
//     {
//         self.query_inner(select).map(|vec| vec.into_iter().map(E::from_inner).collect())
//     }
//     pub fn get<E>(&self, id: u64) -> Result<Option<E>, Error>
//         where E: Entity
//     {
//         self.get_inner(E::meta(), E::orm_meta(), &Cond::by_id(id))
//             .map(|opt| opt.map(E::from_inner))
//     }

//     pub fn close(&self) -> Result<(), Error> {
//         let res = self.flush_cache();
//         self.status.set(SessionStatus::Closed);
//         res
//     }
//     pub fn status(&self) -> SessionStatus {
//         self.status.get()
//     }
// }

// impl Session {
//     pub fn push_cache(&self, rc: EntityInnerPointer) {
//         self.cache.borrow_mut().push(rc);
//     }
//     fn flush_cache(&self) -> Result<(), Error> {
//         // 一定要调用非guard函数！！，因为guard会调用flush_cache导致死循环
//         let result = self.cache.borrow().iter().fold(Ok(()), |result, rc| {
//             if result.is_err() {
//                 return result;
//             }
//             // 如果对象上没有级联标记，默认进行UPDATE
//             let op = rc.borrow().cascade.map_or(Cascade::Update, |op| op);
//             self.execute_inner(rc.clone(), op)
//         });
//         // 重置动态级联标记
//         for rc in self.cache.borrow().iter() {
//             rc.borrow_mut().cascade_reset();
//         }
//         self.cache.borrow_mut().clear();
//         result
//     }
// }

// impl Session {
//     fn batch_inner(&self, vec: &Vec<EntityInnerPointer>, op: Cascade) -> Result<(), Error> {
//         self.guard(op.clone(), || {
//             let result = self.batch_impl(vec, op);
//             // 重置动态级联标记
//             for rc in vec.iter() {
//                 rc.borrow_mut().cascade_reset();
//             }
//             result
//         })
//     }
//     fn execute_inner(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         let result = self.guard(op.clone(), || self.execute_impl(a_rc.clone(), op));
//         a_rc.borrow_mut().cascade_reset();
//         result
//     }
//     pub fn query_inner(&self, select: &Select) -> Result<Vec<EntityInnerPointer>, Error> {
//         select.query_inner(self.conn.borrow_mut().deref_mut()).map(|vec| {
//             for rc in vec.iter() {
//                 rc.borrow_mut().set_session_recur(self.clone());
//             }
//             vec
//         })
//     }
//     pub fn select_inner(&self,
//                         meta: &'static EntityMeta,
//                         orm_meta: &'static OrmMeta,
//                         cond: &Cond)
//                         -> Result<Vec<EntityInnerPointer>, Error> {
//         self.guard(SessionStatus::Select,
//                    || self.select_impl(meta, orm_meta, cond))
//     }
//     pub fn get_inner(&self,
//                      meta: &'static EntityMeta,
//                      orm_meta: &'static OrmMeta,
//                      cond: &Cond)
//                      -> Result<Option<EntityInnerPointer>, Error> {
//         self.guard(SessionStatus::Select, || {
//             self.select_impl(meta, orm_meta, cond).map(|mut vec| match vec.len() {
//                 0 => None,
//                 _ => Some(vec.swap_remove(0)),
//             })
//         })
//     }
//     pub fn clone(&self) -> Session {
//         Session {
//             conn: self.conn.clone(),
//             cache: self.cache.clone(),
//             status: self.status.clone(),
//         }
//     }
// }

// // execute insert update delete
// impl Session {
//     fn batch_impl(&self, vec: &Vec<EntityInnerPointer>, op: Cascade) -> Result<(), Error> {
//         vec.iter().fold(Ok(()), |result, rc| {
//             if result.is_err() {
//                 return result;
//             }
//             return self.execute_impl(rc.clone(), op.clone());
//         })
//     }
//     fn execute_impl(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         if op == Cascade::NULL {
//             return Ok(());
//         }
//         {
//             // 一上来就设为持久态
//             a_rc.borrow_mut().set_session(self.clone());
//             // 本次事务内不会再操作该对象了
//             a_rc.borrow_mut().cascade_null();
//         }
//         try!(self.execute_pointer(a_rc.clone(), op.clone()));
//         try!(self.execute_self(a_rc.clone(), op.clone()));
//         try!(self.execute_one_one(a_rc.clone(), op.clone()));
//         try!(self.execute_one_many(a_rc.clone(), op.clone()));
//         try!(self.execute_many_many(a_rc.clone(), op.clone()));
//         try!(self.execute_middle(a_rc.clone(), op.clone()));
//         Ok(())
//     }
//     fn execute_pointer(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         // pointer
//         let meta = a_rc.borrow().meta;
//         let pointer_map = a_rc.borrow().pointer_map.clone();
//         let mut pointer_fields = Vec::new();
//         for field_meta in meta.get_pointer_fields() {
//             let field = field_meta.get_field_name();
//             let b_rc = pointer_map.get(&field);
//             if b_rc.is_none() {
//                 continue;
//             }
//             let b_rc = b_rc.unwrap();
//             if b_rc.is_none() {
//                 continue;
//             }
//             let b_rc = b_rc.as_ref().unwrap();
//             let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), &field, op.clone());
//             if cascade == Cascade::NULL {
//                 continue;
//             }
//             pointer_fields.push((field.to_string(), b_rc.clone()));
//         }
//         let res = try!(self.each_execute_refer(a_rc.clone(), &pointer_fields, op.clone()));
//         // 更新关系
//         for (field, b_rc) in pointer_fields {
//             a_rc.borrow_mut().set_pointer(&field, Some(b_rc));
//         }
//         Ok(res)
//     }
//     fn execute_one_one(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         let meta = a_rc.borrow().meta;
//         let one_one_map = a_rc.borrow().one_one_map.clone();
//         let mut one_one_fields = Vec::new();
//         for field_meta in meta.get_one_one_fields() {
//             let field = field_meta.get_field_name();
//             let b_rc = one_one_map.get(&field);
//             if b_rc.is_none() {
//                 continue;
//             }
//             let b_rc = b_rc.unwrap();
//             if b_rc.is_none() {
//                 continue;
//             }
//             let b_rc = b_rc.as_ref().unwrap();

//             let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), &field, op.clone());
//             if cascade == Cascade::NULL {
//                 continue;
//             }
//             // 这里set是为了更新关系id
//             a_rc.borrow_mut().set_one_one(&field, Some(b_rc.clone()));
//             one_one_fields.push((field.to_string(), b_rc.clone()));
//         }
//         self.each_execute_refer(a_rc.clone(), &one_one_fields, op.clone())
//     }
//     fn execute_one_many(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         let meta = a_rc.borrow().meta;
//         let one_many_map = a_rc.borrow().one_many_map.clone();
//         let mut one_many_fields = Vec::new();
//         for field_meta in meta.get_one_many_fields() {
//             let field = field_meta.get_field_name();
//             let vec_opt = one_many_map.get(&field);
//             if vec_opt.is_none() {
//                 continue;
//             }
//             let vec = vec_opt.unwrap();
//             for b_rc in vec.iter() {
//                 let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), &field, op.clone());
//                 if cascade == Cascade::NULL {
//                     continue;
//                 }
//                 one_many_fields.push((field.to_string(), b_rc.clone()));
//             }
//             // 这里set是为了更新关系id
//             a_rc.borrow_mut().set_one_many(&field, vec.clone());
//         }
//         self.each_execute_refer(a_rc.clone(), &one_many_fields, op.clone())
//     }
//     fn execute_many_many(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         let meta = a_rc.borrow().meta;
//         let many_many_map = a_rc.borrow().many_many_map.clone();
//         let mut many_many_fields = Vec::new();
//         // 先做对象表
//         for field_meta in meta.get_many_many_fields() {
//             let field = field_meta.get_field_name();
//             let vec_opt = many_many_map.get(&field);
//             if vec_opt.is_none() {
//                 continue;
//             }
//             let vec = vec_opt.unwrap();
//             for &(_, ref b_rc) in vec.iter() {
//                 let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), &field, op.clone());
//                 if cascade == Cascade::NULL {
//                     continue;
//                 }
//                 many_many_fields.push((field.to_string(), b_rc.clone()));
//             }
//         }
//         try!(self.each_execute_refer(a_rc.clone(), &many_many_fields, op.clone()));
//         // 这里set是为了更新关系id
//         for (field, vec) in many_many_map.into_iter() {
//             let vec = vec.into_iter().map(|(_, b_rc)| b_rc).collect::<Vec<_>>();
//             a_rc.borrow_mut().set_many_many(&field, vec);
//         }
//         Ok(())
//     }
//     fn execute_middle(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         let meta = a_rc.borrow().meta;
//         let many_many_map = a_rc.borrow().many_many_map.clone();
//         let mut many_many_fields = Vec::new();
//         // 后做中间表
//         for field_meta in meta.get_many_many_fields() {
//             let field = field_meta.get_field_name();
//             let vec_opt = many_many_map.get(&field);
//             if vec_opt.is_none() {
//                 continue;
//             }
//             let vec = vec_opt.unwrap();
//             for &(ref m_rc, _) in vec.iter() {
//                 if m_rc.is_none() {
//                     continue;
//                 }
//                 let m_rc = m_rc.as_ref().unwrap();
//                 let cascade = Self::calc_cascade(a_rc.clone(), m_rc.clone(), &field, op.clone());
//                 if cascade == Cascade::NULL {
//                     continue;
//                 }
//                 many_many_fields.push((field.to_string(), m_rc.clone()));
//             }
//         }
//         self.each_execute_refer(a_rc.clone(), &many_many_fields, op.clone())
//     }
//     fn execute_self(&self, a_rc: EntityInnerPointer, op: Cascade) -> Result<(), Error> {
//         match op {
//             Cascade::Insert => a_rc.borrow_mut().do_insert(self.conn.borrow_mut().deref_mut()),
//             Cascade::Update => a_rc.borrow_mut().do_update(self.conn.borrow_mut().deref_mut()),
//             Cascade::Delete => a_rc.borrow_mut().do_delete(self.conn.borrow_mut().deref_mut()),
//             Cascade::NULL => Ok(()),
//         }
//     }
//     fn each_execute_refer(&self,
//                           a_rc: EntityInnerPointer,
//                           vec: &Vec<(String, EntityInnerPointer)>,
//                           op: Cascade)
//                           -> Result<(), Error> {
//         for &(ref field, ref b_rc) in vec.iter() {
//             try!(self.execute_refer(a_rc.clone(), b_rc.clone(), field, op.clone()));
//         }
//         Ok(())
//     }
//     fn execute_refer(&self,
//                      a_rc: EntityInnerPointer,
//                      b_rc: EntityInnerPointer,
//                      field: &str,
//                      op: Cascade)
//                      -> Result<(), Error> {
//         let cascade = Self::calc_cascade(a_rc.clone(), b_rc.clone(), field, op);
//         self.execute_impl(b_rc, cascade)
//     }
//     fn calc_cascade(a_rc: EntityInnerPointer,
//                     b_rc: EntityInnerPointer,
//                     field: &str,
//                     op: Cascade)
//                     -> Cascade {
//         // 1. 对象动态级联
//         // 2. 配置动态级联
//         // 3. 配置静态级联
//         let a = a_rc.borrow();
//         let a_b_meta = a.meta.field_map.get(field).unwrap();
//         if b_rc.borrow().cascade.is_some() {
//             return b_rc.borrow().cascade.unwrap().clone();
//         } else if a_b_meta.get_refer_rt_cascade().is_some() {
//             return a_b_meta.get_refer_rt_cascade().clone().unwrap();
//         } else if a_b_meta.has_cascade_insert() && op == Cascade::Insert {
//             return Cascade::Insert;
//         } else if a_b_meta.has_cascade_update() && op == Cascade::Update {
//             return Cascade::Update;
//         } else if a_b_meta.has_cascade_delete() && op == Cascade::Delete {
//             return Cascade::Delete;
//         } else {
//             return Cascade::NULL;
//         }
//     }
// }

// // select
// impl Session {
//     fn select_impl(&self,
//                    meta: &'static EntityMeta,
//                    orm_meta: &'static OrmMeta,
//                    cond: &Cond)
//                    -> Result<Vec<EntityInnerPointer>, Error> {
//         let table_alias = &meta.table_name;
//         let mut tables = Vec::new();
//         let mut fields = Vec::new();
//         Self::gen_sql(&meta.entity_name,
//                       &table_alias,
//                       orm_meta,
//                       &mut tables,
//                       &mut fields);

//         let fields = fields.into_iter()
//             .map(|vec| vec.iter().map(|line| format!("\t{}", line)).collect::<Vec<_>>().join(",\n"))
//             .collect::<Vec<_>>()
//             .join(",\n\n");
//         tables.insert(0, format!("{} AS {}", &meta.table_name, table_alias));
//         let tables = tables.iter().map(|line| format!("\t{}", line)).collect::<Vec<_>>().join("\n");
//         // let cond = format!("\t{}.id = {}", &meta.table_name, id);
//         let sql = format!("SELECT \n{} \nFROM \n{} \nWHERE \n\t{}",
//                           fields,
//                           tables,
//                           cond.to_sql(table_alias));
//         log!("{}", sql);
//         log!("\t{:?}", cond.to_params(table_alias));

//         let mut conn = self.conn.borrow_mut();
//         let query_result = try!(conn.prep_exec(sql, cond.to_params(table_alias)));

//         let mut map: HashMap<String, EntityInnerPointer> = HashMap::new();
//         let mut vec = Vec::new();
//         for row in query_result {
//             let mut row = try!(row);
//             match self.take_entity(&mut row, table_alias, meta, orm_meta, &mut map) {
//                 Some(rc) => vec.push(rc), 
//                 None => {}
//             }
//         }
//         let vec =
//             vec.into_iter().unique_by(|rc| rc.borrow().get_id_u64().unwrap()).collect::<Vec<_>>();
//         Ok(vec)
//     }
//     fn take_entity(&self,
//                    mut row: &mut Row,
//                    table_alias: &str,
//                    meta: &'static EntityMeta,
//                    orm_meta: &'static OrmMeta,
//                    mut map: &mut HashMap<String, EntityInnerPointer>)
//                    -> Option<EntityInnerPointer> {
//         // 关系是空的，这样才能判断出lazy的情况
//         let mut a = EntityInner::new(meta, orm_meta);
//         // 一上来就设为持久态
//         a.set_session(self.clone());
//         a.set_values(&mut row, &table_alias);
//         let id = a.get_id_u64();
//         if id.is_none() {
//             return None;
//         }
//         let id = id.unwrap();
//         let key = format!("{}_{}", table_alias, id);
//         let a_rc = match map.get(&key) {
//             Some(rc) => rc.clone(),
//             None => Rc::new(RefCell::new(a)),
//         };
//         map.insert(key, a_rc.clone());

//         self.take_entity_pointer(a_rc.clone(), &mut row, table_alias, &mut map);
//         self.take_entity_one_one(a_rc.clone(), &mut row, table_alias, &mut map);
//         self.take_entity_one_many(a_rc.clone(), &mut row, table_alias, &mut map);
//         self.take_entity_many_many(a_rc.clone(), &mut row, table_alias, &mut map);
//         Some(a_rc)
//     }
//     fn take_entity_pointer(&self,
//                            a_rc: EntityInnerPointer,
//                            mut row: &mut Row,
//                            table_alias: &str,
//                            mut map: &mut HashMap<String, EntityInnerPointer>) {
//         let mut a = a_rc.borrow_mut();
//         for a_b_meta in a.meta.get_pointer_fields() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             let b_entity = a_b_meta.get_refer_entity();
//             let a_b_field = a_b_meta.get_field_name();
//             let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_alias = format!("{}_{}", table_alias, a_b_field);
//             match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
//                 Some(b_rc) => a.set_pointer(&a_b_field, Some(b_rc)),
//                 None => a.set_pointer(&a_b_field, None),
//             }
//         }
//     }
//     fn take_entity_one_one(&self,
//                            a_rc: EntityInnerPointer,
//                            mut row: &mut Row,
//                            table_alias: &str,
//                            mut map: &mut HashMap<String, EntityInnerPointer>) {
//         let mut a = a_rc.borrow_mut();
//         for a_b_meta in a.meta.get_one_one_fields() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             let b_entity = a_b_meta.get_refer_entity();
//             let a_b_field = a_b_meta.get_field_name();
//             let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_alias = format!("{}_{}", table_alias, a_b_field);
//             match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
//                 Some(b_rc) => a.set_one_one(&a_b_field, Some(b_rc)),
//                 None => a.set_one_one(&a_b_field, None),
//             }
//         }
//     }
//     fn take_entity_one_many(&self,
//                             a_rc: EntityInnerPointer,
//                             mut row: &mut Row,
//                             table_alias: &str,
//                             mut map: &mut HashMap<String, EntityInnerPointer>) {
//         let mut a = a_rc.borrow_mut();
//         for a_b_meta in a.meta.get_one_many_fields() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             let b_entity = a_b_meta.get_refer_entity();
//             let a_b_field = a_b_meta.get_field_name();
//             let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_alias = format!("{}_{}", table_alias, a_b_field);
//             match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
//                 Some(b_rc) => {
//                     let key = format!("ONE_MANY@{}_{}",
//                                       b_table_alias,
//                                       b_rc.borrow().get_id_u64().unwrap());
//                     if !map.contains_key(&key) {
//                         a.push_one_many(&a_b_field, b_rc.clone());
//                     }
//                     map.entry(key).or_insert(b_rc);
//                 }
//                 None => {}
//             }
//         }
//     }
//     fn take_entity_many_many(&self,
//                              a_rc: EntityInnerPointer,
//                              mut row: &mut Row,
//                              table_alias: &str,
//                              mut map: &mut HashMap<String, EntityInnerPointer>) {
//         let mut a = a_rc.borrow_mut();
//         for a_b_meta in a.meta.get_many_many_fields() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             let b_entity = a_b_meta.get_refer_entity();
//             let mid_entity = a_b_meta.get_many_many_middle_entity();
//             let a_b_field = a_b_meta.get_field_name();
//             let b_meta = a.orm_meta.entity_map.get(&b_entity).unwrap();
//             let mid_meta = a.orm_meta.entity_map.get(&mid_entity).unwrap();
//             let b_table_alias = format!("{}_{}", table_alias, a_b_field);
//             let mid_table_alias = format!("{}__{}", table_alias, a_b_field);
//             match self.take_entity(&mut row, &b_table_alias, &b_meta, &a.orm_meta, &mut map) {
//                 Some(b_rc) => {
//                     let key = format!("MANY_MANY@{}_{}",
//                                       b_table_alias,
//                                       b_rc.borrow().get_id_u64().unwrap());
//                     if !map.contains_key(&key) {
//                         let mid_rc = self.take_entity(&mut row,
//                                          &mid_table_alias,
//                                          &mid_meta,
//                                          &a.orm_meta,
//                                          &mut map)
//                             .unwrap();
//                         a.push_many_many(&a_b_field, (mid_rc, b_rc.clone()));
//                     }
//                     map.entry(key).or_insert(b_rc);
//                 }
//                 None => {}
//             }
//         }
//     }
//     fn gen_sql(entity: &str,
//                table_alias: &str,
//                orm_meta: &'static OrmMeta,
//                mut tables: &mut Vec<String>,
//                mut columns: &mut Vec<Vec<String>>) {
//         let meta = orm_meta.entity_map.get(entity).unwrap();
//         let self_columns = Self::gen_sql_columns(meta, table_alias);
//         columns.push(self_columns);

//         Self::gen_sql_pointer(table_alias, meta, orm_meta, tables, columns);
//         Self::gen_sql_one_one(table_alias, meta, orm_meta, tables, columns);
//         Self::gen_sql_one_many(table_alias, meta, orm_meta, tables, columns);
//         Self::gen_sql_many_many(table_alias, meta, orm_meta, tables, columns);
//     }
//     fn gen_sql_pointer(table_alias: &str,
//                        meta: &'static EntityMeta,
//                        orm_meta: &'static OrmMeta,
//                        mut tables: &mut Vec<String>,
//                        mut columns: &mut Vec<Vec<String>>) {
//         for a_b_meta in meta.get_pointer_fields().into_iter() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             // a join b on a.b_id = b.id
//             let a_b_field = a_b_meta.get_field_name();
//             let b_entity = a_b_meta.get_refer_entity();
//             let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_name = &b_meta.table_name;
//             let a_b_id_field = a_b_meta.get_pointer_id();
//             let a_b_id_meta = meta.field_map.get(&a_b_id_field).unwrap();
//             let a_b_id_column = a_b_id_meta.get_column_name();
//             let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
//             let join_table = format!("LEFT JOIN {} AS {} ON {}.{} = {}.id",
//                                      &b_table_name,
//                                      &b_table_alias,
//                                      &table_alias,
//                                      &a_b_id_column,
//                                      &b_table_alias);
//             tables.push(join_table);
//             Self::gen_sql(&b_entity,
//                           &b_table_alias,
//                           orm_meta,
//                           &mut tables,
//                           &mut columns);
//         }
//     }
//     fn gen_sql_one_one(table_alias: &str,
//                        meta: &'static EntityMeta,
//                        orm_meta: &'static OrmMeta,
//                        mut tables: &mut Vec<String>,
//                        mut columns: &mut Vec<Vec<String>>) {
//         for a_b_meta in meta.get_one_one_fields().into_iter() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             // a join b on a.id = b.a_id
//             let a_b_field = a_b_meta.get_field_name();
//             let b_entity = a_b_meta.get_refer_entity();
//             let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_name = &b_meta.table_name;
//             let b_a_id_field = a_b_meta.get_one_one_id();
//             let b_a_id_meta = b_meta.field_map.get(&b_a_id_field).unwrap();
//             let b_a_id_column = b_a_id_meta.get_column_name();
//             let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
//             let join_table = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
//                                      &b_table_name,
//                                      &b_table_alias,
//                                      &table_alias,
//                                      &b_table_alias,
//                                      &b_a_id_column);
//             tables.push(join_table);
//             Self::gen_sql(&b_entity,
//                           &b_table_alias,
//                           orm_meta,
//                           &mut tables,
//                           &mut columns);
//         }
//     }
//     fn gen_sql_one_many(table_alias: &str,
//                         meta: &'static EntityMeta,
//                         orm_meta: &'static OrmMeta,
//                         mut tables: &mut Vec<String>,
//                         mut columns: &mut Vec<Vec<String>>) {
//         for a_b_meta in meta.get_one_many_fields().into_iter() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             // a join b on a.id = b.a_id
//             let a_b_field = a_b_meta.get_field_name();
//             let b_entity = a_b_meta.get_refer_entity();
//             let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
//             let b_table_name = &b_meta.table_name;
//             let b_a_id_field = a_b_meta.get_one_many_id();
//             let b_a_id_meta = b_meta.field_map.get(&b_a_id_field).unwrap();
//             let b_a_id_column = b_a_id_meta.get_column_name();
//             let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
//             let join_table = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
//                                      &b_table_name,
//                                      &b_table_alias,
//                                      &table_alias,
//                                      &b_table_alias,
//                                      &b_a_id_column);
//             tables.push(join_table);
//             Self::gen_sql(&b_entity,
//                           &b_table_alias,
//                           orm_meta,
//                           &mut tables,
//                           &mut columns);
//         }
//     }
//     fn gen_sql_many_many(table_alias: &str,
//                          meta: &'static EntityMeta,
//                          orm_meta: &'static OrmMeta,
//                          mut tables: &mut Vec<String>,
//                          mut columns: &mut Vec<Vec<String>>) {
//         for a_b_meta in meta.get_many_many_fields().into_iter() {
//             if !a_b_meta.is_fetch_eager() {
//                 continue;
//             }
//             // a join a_b on a.id = a_b.a_id join b on a_b.b_id = b.id
//             let a_b_field = a_b_meta.get_field_name();
//             let b_entity = a_b_meta.get_refer_entity();
//             let mid_entity = a_b_meta.get_many_many_middle_entity();
//             let b_meta = orm_meta.entity_map.get(&b_entity).unwrap();
//             let mid_meta = orm_meta.entity_map.get(&mid_entity).unwrap();
//             let b_table_name = &b_meta.table_name;
//             let mid_table_name = &mid_meta.table_name;
//             let mid_a_id_field = a_b_meta.get_many_many_id();
//             let mid_b_id_field = a_b_meta.get_many_many_refer_id();
//             let mid_a_id_meta = mid_meta.field_map.get(&mid_a_id_field).unwrap();
//             let mid_b_id_meta = mid_meta.field_map.get(&mid_b_id_field).unwrap();
//             let mid_a_id_column = mid_a_id_meta.get_column_name();
//             let mid_b_id_column = mid_b_id_meta.get_column_name();
//             let mid_table_alias = format!("{}__{}", &table_alias, &a_b_field);
//             let b_table_alias = format!("{}_{}", &table_alias, &a_b_field);
//             let join_mid = format!("LEFT JOIN {} AS {} ON {}.id = {}.{}",
//                                    &mid_table_name,
//                                    &mid_table_alias,
//                                    &table_alias,
//                                    &mid_table_alias,
//                                    &mid_a_id_column);
//             let join_b = format!("LEFT JOIN {} AS {} ON {}.{} = {}.id",
//                                  &b_table_name,
//                                  &b_table_alias,
//                                  &mid_table_alias,
//                                  &mid_b_id_column,
//                                  &b_table_alias);
//             tables.push(join_mid);
//             Self::gen_sql(&mid_entity,
//                           &mid_table_alias,
//                           orm_meta,
//                           &mut tables,
//                           &mut columns);
//             tables.push(join_b);
//             Self::gen_sql(&b_entity,
//                           &b_table_alias,
//                           orm_meta,
//                           &mut tables,
//                           &mut columns);
//         }
//     }
//     fn gen_sql_columns(meta: &'static EntityMeta, table_alias: &str) -> Vec<String> {
//         meta.get_non_refer_fields()
//             .iter()
//             .map(|field_meta| {
//                 let column_name = field_meta.get_column_name();
//                 let field_name = field_meta.get_field_name();
//                 format!("{}.{} as {}${}",
//                         &table_alias,
//                         &column_name,
//                         &table_alias,
//                         &field_name)
//             })
//             .collect()
//     }
// }
