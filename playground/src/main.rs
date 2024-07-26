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
}
