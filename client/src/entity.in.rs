struct Person{
    age:i32,
    #[len(32)]
    #[nullable(false)]
    name:String,
    #[pointer]
    addr:Address,
}

struct Address{
	road: String,
	no: u64,	
}