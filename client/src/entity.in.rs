#[table(tbl_test)]
struct Test {
    int_val: i32,
    str_val: String,

    // #[refer(test_id, test_id)]
    // #[cascade(insert)]
    // test: Test,

    // #[pointer]
    // #[cascade(insert)]
    // ptr: Ptr,

    // #[one_one]
    // #[cascade(insert)]
    // oo: Oo,

    // #[one_many]
    // #[cascade(insert)]
    // om: Om,
}

struct Ptr {
    int_val: i64,
}
struct Oo {
    int_val: i64,
}
struct Om {
    int_val: i64,
}
