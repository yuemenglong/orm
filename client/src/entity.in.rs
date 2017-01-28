struct Person{
    age:i32,
    #[len(32)]
    #[nullable(false)]
    name:String,
}