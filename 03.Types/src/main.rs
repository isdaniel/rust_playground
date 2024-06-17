use std::fmt::Debug;

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn print_split_line() {
    println!("=================="); 
}

/// Function that returns an integer
fn foo(x: i32) -> i32 {
    x + 5 // return 5;
}

fn main() {
    let spaces = "    !";
    print_type_of(&spaces);
    println!("The length of spaces is: {}", spaces);
    //shadowing the spaces variable to store the length of the spaces string.
    let spaces = spaces.len();
    print_type_of(&spaces);
    println!("The length of spaces is: {}", spaces);
    
    let _x = 1.0; //f64
    //print_type_of(&_x);
    let _y: f32 = 1.0; //f32
    //print_type_of(&_y);

    print_split_line();
    let tuple1: (i32, f64, &str) = (1, 2.0, "hello tuple");
    print_type_of(&tuple1);
    //println!("Tuple {} {} {}", tuple1.0, tuple1.1, tuple1.2);
    println!("Tuple: {:?}", tuple1);
    let (x, y, z) = tuple1;
    println!("value from tuple: {} {} {}", x, y, z);

    print_split_line();
    let arr: [i32; 5] = [1, 2, 3, 4, 5];
    let same_val_arr = [3; 5]; // [3, 3, 3, 3, 3]
    loop {
        println!("Array: {:?}", arr);
        println!("Array: {:?}", same_val_arr);
        break;
    }
    print_split_line();

    let x = 5;
    let y = {
        let x = 3;
        x + 1 
    };
    println!("The value of y is: {}", y);

    print_split_line();
    println!("The value of foo is: {}", foo(5));
}
