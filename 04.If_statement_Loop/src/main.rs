fn print_split_line() {
    println!("=================="); 
}

fn main() {
    let number = 3;

    if number % 4 == 0{
        println!("Number is divisible by 4");
    } else if number % 3 == 0 {
        println!("Number is divisible by 3");
    } else if number % 2 == 0 {
        println!("Number is divisible by 2");
    } else {
        println!("Number is not divisible by 4, 3, or 2");
    }

    print_split_line();

    match number{
        n if n % 4 == 0 => println!("Number is divisible by 4"),
        n if n % 3 == 0 => println!("Number is divisible by 3"),
        n if n % 2 == 0 => println!("Number is divisible by 2"),
        _ => println!("Number is not divisible by 4, 3, or 2"),
    }
    
    print_split_line();

    let condition = true;

    let number = if condition {
        5
    } else {
        // "hello" // error: expected type `i32`, found reference `&str`
        6
    };

    println!("The value of number is: {}", number);
    print_split_line();
    let mut count = 0;
    loop {
        if count == 10 {
            break;
        }
        println!("Again! {}", count);
        count+=1;
    }
    print_split_line();
    let res = loop {
        if count == 15 {
            break count * 2;
        }
        count+=1;
    };
    println!("The value of res is: {}", res);
    print_split_line();

    let mut number = 3;

    while number != 0 {
        println!("{}!", number);
        number -= 1;
    }
    print_split_line();
    let arr = [10, 20, 30, 40, 50];
    //arr.iter().for_each(|x| println!("The value is: {}", x));
    for element in arr.iter() {
        println!("The value is: {}", element);
    }

    for number in (1..10).rev() {
        println!("{}!", number);
    }
}
