use entity::Entity;
use cond::Cond;
use meta::EntityMeta;
use meta::OrmMeta;
use std::rc::Rc;
use std::cell::RefCell;

struct Select {
    meta: &'static EntityMeta,
    orm_meta: &'static OrmMeta,
    cond: Option<Cond>,
    joins: Vec<(String, Rc<RefCell<Select>>)>,
}

impl Select {
    fn from<E>() -> Self
        where E: Entity
    {
        Select {
            meta: E::meta(),
            orm_meta: E::orm_meta(),
            cond: None,
        }
    }
    fn wher(&mut self, cond: &Cond) -> &Self {
        self.cond = Some(cond);
        self
    }
    fn join(&mut self, field:&str)->Self{

    }
}
