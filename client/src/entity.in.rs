struct Person {
    age: i32,
    #[len(32)]
    #[nullable(false)]
    name: String,
    #[pointer]
    #[cascade(insert, update, delete)]
    addr: Address,
    #[pointer]
    addr2: Address,
    #[one_one]
    #[cascade(insert, update, delete)]
    account: Account,
    #[one_many]
    #[cascade(insert, update, delete)]
    children: Vec<Child>,
    #[many_many]
    teachers: Vec<Teacher>,
}

struct Teacher {
    name: String,
}

struct Child {
    name: String,
}

struct Address {
    road: String,
    no: u64,
}

struct Account {
    bank: String,
    no: String,
}
