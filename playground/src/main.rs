struct User{
    user_id : i32,
    posts: Vec<String>
}

impl User {
    fn set_id(&mut self, id: i32){
        self.user_id = id;
    }
}

fn main() {
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
}

enum Message {
    Hello {id: i32},
}


pub struct Point {
    x: i32,
    y: i32,
    z: i32
}

