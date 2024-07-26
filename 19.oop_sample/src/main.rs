use oop_sample::*;

fn main() {
    let screen: Screen = Screen{
        components: vec![
            Box::new(SelectBox {
                width: 75,
                height: 10,
                options: vec![
                    String::from("Yes"),
                    String::from("Maybe"),
                    String::from("No"),
                ],
            }),
            Box::new(Button {
                width: 50,
                height: 10,
                label: String::from("OK"),
            }),
        ],
    };
    screen.run();

    println!("=====================");

    let mut post = Post::new();
    post.add_text("I ate a salad for lunch today");
    assert_eq!("", post.content());
    post.request_review();
    assert_eq!("", post.content());
    println!("Post content: {}", post.content());
    post.approve();
    assert_eq!("I ate a salad for lunch today", post.content());
    println!("Post content: {}", post.content());
} 
