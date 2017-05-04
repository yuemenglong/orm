#[table(tbl_test)]
struct Test {
    int_val: i32,
    str_val: String,

    #[pointer]
    #[cascade(insert)]
    ptr: Ptr,
}

struct Ptr {
    int_val: i64,
}
