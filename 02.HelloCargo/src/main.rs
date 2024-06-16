use std::{cmp::Ordering, io};
use rand::Rng;

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
}
