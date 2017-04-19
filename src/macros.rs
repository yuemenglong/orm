#[macro_export]
macro_rules! debug {
    ($($e:expr),+) => {{
        let vec = file!().split("\\").collect::<Vec<_>>();
        let file = vec.last().unwrap();
        println!("[{}:{}] {}", file, line!(), debug_format!($($e),+));
    }};
}

macro_rules! debug_format {
    ($e:expr) => {{
        format!("{:?}", $e)
    }};
    ($e:expr, $($x:expr),+) => {{
        format!("{:?}, {}", $e, debug_format!($($x),+))
    }};
}
