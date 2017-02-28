struct Person {
    age: i32,
    #[len(32)]
    #[nullable(false)]
    name: String,
    #[pointer]
    #[cascade(insert, update, delete)]
    addr: Address,
    #[one_one]
    #[cascade(insert, update, delete)]
    account: Account,
}

struct Address {
    road: String,
    no: u64,
}

struct Account {
    bank: String,
    no: String,
}
