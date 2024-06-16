fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn main() {
    let spaces = "    !";
    print_type_of(&spaces);
    println!("The length of spaces is: {}", spaces);
    //shadowing the spaces variable to store the length of the spaces string.
    let spaces = spaces.len();
    print_type_of(&spaces);
    println!("The length of spaces is: {}", spaces);
    
}
