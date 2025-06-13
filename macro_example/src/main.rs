use marco_example::*;
use std::collections::HashMap;

pub struct PrintlnRecorder;

impl FunctionCallRecorder for PrintlnRecorder {
    fn record_call(fn_name: &str, args: &str) {
        println!("[PrintlnRecorder] Calling function `{}` with args: {}", fn_name, args);
    }

    fn record_return(fn_name: &str, result: &str) {
        println!("[PrintlnRecorder] Function `{}` returned: {}", fn_name, result);
    }
}

fn add(val1: i32, val2: i32) -> i32 {
    val1 + val2
}

fn add_nores(val1: i32, val2: i32)  {
    let _ = val1 + val2;
}

fn main() {
    println!("{}, {}", ops!(100 mutiply 10), ops!(100 plus 10));

    if_any!(false, 0 == 1, true; {
        println!("Yay, the if statement worked.");
    });

    let value = "hello";
    let my_hashmap = hashmap!(
        "hash" => "map",
        "Key" => value,
    );

    println!("{my_hashmap:#?}");

    let my_number = number!(nine three seven two zero).parse::<u32>().unwrap();
    let my_other_number = number!(one two four six eight zero).parse::<u32>().unwrap();
    println!("{}", my_number + my_other_number); // = 218400

    let _ = function_call_with_aop!(add(1, 2));
    let _ = function_call_with_aop!(add_nores(1, 2));

    let _ = function_call_with_aop!(PrintlnRecorder,add_nores(1, 2));
}
