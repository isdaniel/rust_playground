use std::collections::HashMap;

#[macro_export]
macro_rules! ops {
    ($v1:literal plus $v2:literal) => {
        $v1 + $v2
    };
    ($v1:literal minus $v2:literal) => {
        $v1 - $v2
    };
    ($v1:literal mutiply $v2:literal) => {
        $v1 * $v2
    };
    ($v1:literal divide $v2:literal) => {
        $v1 / $v2
    };
}

#[macro_export]
macro_rules! if_any {
    ($($condition:expr),+ ;$block:block) => {
        if $($condition) || +
            $block
    };
}

#[macro_export]
macro_rules! hashmap {
    ($($key:literal => $value:expr,)*) => {
        {
            let mut map = HashMap::new();
            $(map.insert($key,$value);)*
            map
        }
    };
}

#[macro_export]
macro_rules! digit {
    (zero) => {
        "0"
    };
    (one) => {
        "1"
    };
    (two) => {
        "2"
    };
    (three) => {
        "3"
    };
    (four) => {
        "4"
    };
    (five) => {
        "5"
    };
    (six) => {
        "6"
    };
    (seven) => {
        "7"
    };
    (eight) => {
        "8"
    };
    (nine) => {
        "9"
    };
}

#[macro_export]
macro_rules! number {
    ($($num:ident)+) => {
        concat!($(digit!($num)),+)
    };
}

pub trait FunctionCallRecorder {
    fn record_call(fn_name: &str, args: &str);
    fn record_return(fn_name: &str, result: &str);
}

/*
fn add(val1: i32, val2: i32) -> i32 {
    val1 + val2
}

fn add_nores(val1: i32, val2: i32)  {
    let _ = val1 + val2;
}
*/
#[macro_export]
macro_rules! function_call_with_aop {
    ($fn:ident ( $($args:expr),* )) => {{
        println!("Calling function : {}  with args: {:?}",stringify!($fn),($($args),*));
        let result = $fn($($args),*);
        println!("function name:{} returned: {:?}",stringify!($fn), result);
        result
    }};

    ($recorder:ty, $fn:ident ( $($arg:expr),* )) => {{
        let args_str = format!("{:?}", ($($arg,)*));
        <$recorder>::record_call(stringify!($fn), &args_str);
        let result = $fn($($arg),*);
        let result_str = format!("{:?}", result);
        <$recorder>::record_return(stringify!($fn), &result_str);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_ops_macro() {
        assert_eq!(ops!(10 plus 5), 15);
        assert_eq!(ops!(10 minus 5), 5);
        assert_eq!(ops!(3 mutiply 4), 12);
        assert_eq!(ops!(20 divide 5), 4);
    }

    #[test]
    fn test_if_any_macro_executes_block() {
        let mut x = 0;
        if_any!(false, true; {
            x = 42;
        });
        assert_eq!(x, 42);
    }

    #[test]
    fn test_hashmap_macro() {
        let map = hashmap!(
            "a" => 1,
            "b" => 2,
        );
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get("c"), None);
    }

    #[test]
    fn test_digit_macro() {
        assert_eq!(digit!(zero), "0");
        assert_eq!(digit!(five), "5");
        assert_eq!(digit!(nine), "9");
    }

    #[test]
    fn test_number_macro() {
        let s = number!(nine three seven two zero);
        assert_eq!(s, "93720");
        let n: u32 = s.parse().unwrap();
        assert_eq!(n, 93720);

        let s2 = number!(one two four six eight zero);
        let n2: u32 = s2.parse().unwrap();
        assert_eq!(n2, 124680);
    }

    #[test]
    fn test_function_call_with_aop_macro() {
        LOGS.lock().unwrap().clear();

        let result = function_call_with_aop!(TestRecorder, add(5, 3));
        assert_eq!(result, 8);

        let logs = LOGS.lock().unwrap().clone();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0], "CALL: add (5, 3)");
        assert_eq!(logs[1], "RETURN: add 8");
    }

    fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    struct TestRecorder;

    lazy_static::lazy_static! {
        static ref LOGS: Mutex<Vec<String>> = Mutex::new(vec![]);
    }

    impl FunctionCallRecorder for TestRecorder {
        fn record_call(fn_name: &str, args: &str) {
            LOGS.lock().unwrap().push(format!("CALL: {} {}", fn_name, args));
        }

        fn record_return(fn_name: &str, result: &str) {
            LOGS.lock().unwrap().push(format!("RETURN: {} {}", fn_name, result));
        }
    }

}