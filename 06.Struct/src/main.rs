struct User {
    username: String,
    email: String,
    sign_in_count: u64,
    active: bool,
}
#[derive(Debug)]
struct Rectangle{
    width: u32,
    height: u32,
}

//declare function in struct would need to use impl keywork
impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }

    fn can_hold(&self, other: &Rectangle) -> bool{
        self.width > other.width && self.height > other.height
    }
    //
    fn square(size : u32) -> Rectangle{
        Rectangle{
            width : size,
            height : size
        }
    }
}

fn print_split_line() {
    println!("=================="); 
}

fn build_user(email: String, username: String) -> User {
    User {
        email,
        username,
        active: true,
        sign_in_count: 1,
    }
}

// fn area(dim:(u32,u32)) -> u32 {
//     dim.0 * dim.1
// }

fn area(rect: &Rectangle) -> u32 {
    rect.width * rect.height
}

fn main() {
    let user1 = build_user(String::from("dd@gmail.com"), String::from("dd"));
    println!("user1 email: {} , {}", user1.email, user1.username);
    print_split_line();
    //let rect = (30,50);
    //println!("Area of rectangle is {}", area(rect));
    let rect = Rectangle{width: 30, height: 50};
    println!("Area of rectangle is {}", area(&rect));
    //println!("Area of rectangle is {:?}", rect);
    println!("Area of rectangle is {:#?}", rect);
    
    print_split_line(); //struct function

    println!("Struct function Area of rectangle is {}", rect.area());
    let small_rect = Rectangle{width: 10, height: 10};
    let bigge_rect = Rectangle{width: 10, height: 100};
    println!("{}",rect.can_hold(&small_rect));
    println!("{}",rect.can_hold(&bigge_rect));

    print_split_line(); 
    println!("Area of rectangle is {:#?}", Rectangle::square(10));
}
