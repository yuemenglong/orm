struct Person {
    age: i32,
    #[len(32)]
    #[nullable(false)]
    name:String,
    #[pointer]
    #[cascade(insert, update, delete)]
    addr:Address,
}

struct Address {
    road: String,
    no: u64,
}
