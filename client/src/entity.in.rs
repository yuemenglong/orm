struct Person {
    age: i32,
    #[len(32)]
    // #[nullable(false)]
    name: String,
    #[pointer]
    addr: Address,
    // addr: Option<Address>, //refer
    // addr_id: Option<u64>,
}

struct Address {
    // road: String,
    no: u64,
}
