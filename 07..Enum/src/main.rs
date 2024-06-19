#[derive(Debug)]
enum Message{
    Quit,
    Move{x:i32,y:i32},
    Write(String),
    ChangeColor(i32,i32,i32),
}

impl Message{
    fn call(&self){
        println!("{:?}",self);
    }
}
#[derive(Debug)]
enum USState{
    Alabama,
    Alaska,
    // --snip--
}

enum Coin {
    Penny,
    Nickel,
    Dime,
    Quarter(USState),
}

fn value_in_cents(coin: Coin) -> u32{
    match coin{
        Coin::Penny => {
            println!("Lucky Penny!");
            1
        },
        Coin::Nickel => 5,
        Coin::Dime => 10,
        Coin::Quarter(state) => {
            println!("State quarter from {:?}!", state);
            25
        },
    }
}



fn main() {
    let q = Message::Quit;
    let m = Message::Move{x:1,y:2};
    let w = Message::Write(String::from("hello"));
    let c = Message::ChangeColor(1,2,3);
    m.call();
    println!("=================="); //Option enum
    //Option enum is a type that can be used when you want to express that a value could be something or it could be nothing.
    let some_number = Some(5);
    let some_string = Some("a string");
    let absent_number: Option<i32> = None;
    println!("=================="); //match

    let number = Some(5);
    let number2 = Some(5);
    let sum = match (number, number2) {
        (Some(n1), Some(n2)) => Some(n1 + n2),
        _ => None,
    };

    println!("number + number2: {:?} , convert type {}", sum, number.expect("Convert to i32 failed!"));
    
    let name = String::from("Hello World!");
    println!("The length of name is: {}", match name.chars().nth(6){
        Some(v) => v.to_string(),  
        None => "No character found!".to_string(),
    });

    println!("{}", value_in_cents(Coin::Quarter(USState::Alaska))); 
}