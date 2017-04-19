use std::mem;
use meta::OrmMeta;
use visitor;
use rustc_serialize::json;

pub unsafe fn init_meta(json: &'static str) -> &'static OrmMeta {
    let mut meta: OrmMeta = json::decode(json).unwrap();
    visitor::fix_meta(&mut meta);
    leak(meta)
}

pub fn leak<T>(v: T) -> &'static T {
    unsafe {
        let b = Box::new(v);
        let p: *const T = &*b;
        mem::forget(b); // leak our reference, so that `b` is never freed
        &*p
    }
}
