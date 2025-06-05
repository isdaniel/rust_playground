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

fn main() {
    println!("{}, {}", ops!(100 mutiply 10), ops!(100 plus 10));
}
