use meta::*;
use std::cell::RefMut;

fn attach_type(field_meta: &mut RefMut<FieldMeta>) {
    let ty_pattern = Regex::new(r"(^Option<([^<>]+)>$)|(^[^<>]+$)").unwrap();
    let attach = match ty_pattern.captures(&field_meta.raw_ty) {
        Some(captures) => {
            match captures.get(3) {
                Some(_) => (field_meta.raw_ty.clone(), false),
                None => (captures.get(2).unwrap().as_str().to_string(), true),
            }
        }
        None => {
            panic!("Unsupport Type: {}", field_meta.raw_ty);
        }
    };
    field_meta.ty = attach.0;
    field_meta.nullable = attach.1;
}
fn attach_db_type(field_meta: &mut RefMut<FieldMeta>) {
    let postfix = match field_meta.nullable {
        true => "",
        false => " NOT NULL",
    };
    field_meta.db_ty = match field_meta.ty.as_ref() {
        "i32" => format!("INTEGER{}", postfix),
        "i64" => format!("BIGINT{}", postfix),
        "String" => format!("VARCHAR({}){}", field_meta.len, postfix),
        _ => {
            panic!("Unsupported Type: {}", field_meta.ty);
        }
    }
}
