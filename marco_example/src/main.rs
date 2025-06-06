use marco_example::*;
use std::collections::HashMap;

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
}
