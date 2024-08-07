use std::{clone, fmt::format, fs::File, io::{Error, ErrorKind, Read}, rc::Rc};
struct User{
    user_id : i32,
    posts: Vec<String>
}

impl User {
    fn set_id(&mut self, id: i32){
        self.user_id = id;
    }
}

fn move_testing(obj : &mut mytype) -> &mut mytype{
    obj.a += 1;
    obj.b += 1;
    println!("obj: {:?}", obj);
    obj
}

#[derive(Debug)]
#[derive(Clone)]
struct mytype{
    a: i32,
    b: i32
}

//todo make a linked list
#[derive(Debug)]
enum List {
    Next(i32,Rc<List>),
    Nil,
}


fn main() {
    let mut list = Rc::new(List::Nil);
    for i in 0..10 {
        list = Rc::new(List::Next(i, list.clone()));
    }
    
    while let List::Next(value, next) = list.as_ref() {
        println!("Value: {}", value);
        list = Rc::clone(&next);
    }

    // while let List::Next(value, next) = list {
    //     println!("Value: {}", value);
    //     list = Rc::clone(&next);
    // }
    // let a = List::Next(1, Rc::clone(&list));

    // println!("List: {:?}", a);
    println!("=====================");

    let mut a = mytype{a: 1, b: 1};
    println!("obj: {:?}", move_testing(&mut a));
    println!("obj: {:?}", move_testing(&mut a));

    let mut user = User{
        user_id: 0,
        posts: vec!["Post1".to_string(), "Post2".to_string()]
    };

    //let user_posts = user.posts;
    user.user_id = 1;
    user.set_id(1);

    let mut stack = vec![];
    stack.push(1);
    stack.push(2);
    while let Some(top) = stack.pop(){
        println!("Top: {}", top);
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

    let msg = match result_func(){
        Ok(s) => s,
        Err(e) => format!("Error: {}", e),
    };

    println!("Result: {msg}");
}

fn result_func() -> Result<String, std::io::Error> {
    let mut s = String::new();
    File::open("hello.txt")?.read_to_string(&mut s)?;
    Ok(s)
}


enum Message {
    Hello {id: i32},
}


pub struct Point {
    x: i32,
    y: i32,
    z: i32
}

