use entity::Entity;
use entity::EntityInner;
use entity::EntityInnerPointer;
use value::FieldValue;

use meta::EntityMeta;
use meta::OrmMeta;

use mysql::prelude::GenericConnection;
use mysql::Error;
use mysql::Value;

use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub struct Insert {
    withs: Vec<(String, Insert)>,
}

impl Insert {
    pub fn new() -> Self {
        Insert { withs: Vec::new() }
    }
    pub fn default<E>() -> Self
        where E: Entity
    {
        Self::default_meta(E::meta(), E::orm_meta())
    }
    fn default_meta(meta: &EntityMeta, orm_meta: &OrmMeta) -> Self {
        let withs = meta.field_vec.iter().filter_map(|field| {
            let field_meta = meta.field_map.get(field).unwrap();
            if !field_meta.is_type_refer() || !field_meta.has_cascade_insert() {
                return None;
            }
            let entity_name = field_meta.get_refer_entity();
            let entity_meta = orm_meta.entity_map.get(&entity_name).unwrap();
            let insert = Self::default_meta(entity_meta, orm_meta);
            return Some((field.to_string(), insert));
        }).collect();
        Insert { withs: withs }
    }
    pub fn with(&mut self, field: &str) -> &mut Insert {
        let insert = Insert::new();
        self.withs.push((field.to_string(), insert));
        &mut self.withs.last_mut().unwrap().1
    }
    pub fn execute<C, E>(&self, conn: &mut C, entity: &E) -> Result<u64, Error>
        where C: GenericConnection,
              E: Entity
    {
        self.execute_inner(conn, entity.inner())
    }
    pub fn execute_inner<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
        where C: GenericConnection
    {
        if !rc.borrow().meta.is_id_auto() && !rc.borrow().is_value_null("id") {
            panic!("Id Not Auto And Has No Value");
        }
        // pointer
        let r1 = try!(self.execute_pointer(conn, rc.clone()));
        let r2 = try!(self.execute_self(conn, rc.clone()));
        let r3 = try!(self.execute_one_one(conn, rc.clone()));
        let r4 = try!(self.execute_one_many(conn, rc.clone()));
        Ok(r1 + r2 + r3 + r4)
    }
    pub fn execute_pointer<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
        where C: GenericConnection
    {
        self.withs
            .iter()
            .filter_map(|&(ref field, ref ins)| {
                // 是pointer
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_pointer() {
                    return None;
                }
                // 有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .map_or(None, |v| v.as_entity().map(|b_rc| (field, ins, b_rc)))
            })
            .fold(Ok(0), |acc, (field, ins, b_rc)| {
                if acc.is_err() {
                    return acc;
                }
                let res = ins.execute_inner(conn, b_rc.clone());
                if res.is_err() {
                    return res;
                }
                // a.b_id = b.id
                let (left, right) = rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                let b_id = b_rc.borrow().field_map.get(&right).map(|v| v.clone());
                if b_id.is_some() {
                    rc.borrow_mut().field_map.insert(left, b_id.unwrap());
                }

                let acc = acc.unwrap() + res.unwrap();
                Ok(acc)
            })
    }
    pub fn execute_one_one<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
        where C: GenericConnection
    {
        self.withs
            .iter()
            .filter_map(|&(ref field, ref ins)| {
                // 是one_one
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_one_one() {
                    return None;
                }
                // 且有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .map_or(None, |v| v.as_entity().map(|b_rc| (field, ins, b_rc)))
            })
            .fold(Ok(0), |acc, (field, ins, b_rc)| {
                if acc.is_err() {
                    return acc;
                }
                // b.a_id = a.id
                let (left, right) = rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                let a_id = rc.borrow().field_map.get(&left).map(|v| v.clone());
                if a_id.is_some() {
                    b_rc.borrow_mut().field_map.insert(right, a_id.unwrap());
                }

                let res = ins.execute_inner(conn, b_rc.clone());
                if res.is_err() {
                    return res;
                }
                let acc = acc.unwrap() + res.unwrap();
                Ok(acc)
            })
    }
    pub fn execute_one_many<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
        where C: GenericConnection
    {
        self.withs
            .iter()
            .filter_map(|&(ref field, ref ins)| {
                // one_many
                if !rc.borrow().meta.field_map.get(field).unwrap().is_refer_one_many() {
                    return None;
                }
                // 且有值的
                rc.borrow()
                    .field_map
                    .get(field)
                    .map(|v| (field, ins, v.as_vec()))
            })
            .fold(Ok(0), |acc, (field, ins, vec)| {
                if acc.is_err() {
                    return acc;
                }
                // b.a_id = a.id
                let (left, right) = rc.borrow().meta.field_map.get(field).unwrap().get_refer_lr();
                let a_id = rc.borrow().field_map.get(&left).map(|v| v.clone());
                if a_id.is_some() {
                    for b_rc in vec.iter() {
                        b_rc.borrow_mut()
                            .field_map
                            .insert(right.to_string(), a_id.clone().unwrap());
                    }
                }

                let res = vec.iter().fold(Ok(0), |acc, b_rc| {
                    if acc.is_err() {
                        return acc;
                    }
                    let res = ins.execute_inner(conn, b_rc.clone());
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

    pub fn execute_self<C>(&self, conn: &mut C, rc: EntityInnerPointer) -> Result<u64, Error>
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
