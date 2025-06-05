use std::collections::HashMap;

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

macro_rules! if_any {
    ($($condition:expr),+ ;$block:block) => {
        if $($condition) || +
            $block
    };
}

macro_rules! hashmap {
    ($($key:literal => $value:expr,)*) => {
        {
            let mut map = HashMap::new();
            $(map.insert($key,$value);)*
            map
        }
    };
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
}
