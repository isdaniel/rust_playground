use std::{cmp::Ordering, io};
use rand::Rng;

enum Message {
    Hello {id: i32},
}


pub struct Point {
    x: i32,
    y: i32,
    z: i32
}


fn main() {
    println!("Guess Numbre!");
    let secret_number = rand::thread_rng().gen_range(1..101); 
    //println!("secret_number is: {}", secret_number);
    loop {
        println!("Please input your guess.");
        //declare a mutable variable named guess to store the user input, default is immutable.
        // let foo = 1;
        // foo = 2; // error: cannot assign twice to immutable variable [E0384]
        let mut guess = String::new();
        
        io::stdin().read_line(&mut guess)
            .expect("Failed to read line");
    
        println!("Your guess number is: {}", guess);
    
        //shadowing the guess variable to convert the string to a number.
        let guess: u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue,
        };
        
        match guess.cmp(&secret_number) {
            Ordering::Less => println!("Too small!"),
            Ordering::Greater => println!("Too big!"),
            Ordering::Equal => {
                println!("you win! you got it!");
                break;
            },
        }
    }


    let origin = Point{x: 0, y: 0,z : 0};

    match origin {
        Point{x,..} => println!("x: {}", x),
    }

    let number = (2,4,8,6,10);
    match number {
        (first, .., last) => println!("First: {}, Last: {}", first, last),
    }

    let num = Some(4);

    match num {
        Some(x) if x < 5 => println!("Less than 5"),
        Some(x) => println!("Greater than 5"),
        None => ()
    }

    let msg = Message::Hello{id: 5};

    match msg {
        Message::Hello { 
            id : id_variable @ 3..=7,
        } => {
            println!("Found an id in range: {}", id_variable);
        }
        Message::Hello { id: 10..=12 } => {
            println!("Found an id in another range");
        }
        Message::Hello { id } => {
            println!("Found some other id: {}", id);
        }
    }
}
