//trait is a way to define a set of methods that a type must have in order to belong to a particular group
pub trait Summary{
    fn summarize(&self) -> String;

    fn default_summarize(&self) -> String{
        format!("default_summarize...")
    }
}

pub struct NewsArticle {
    pub headline: String,
    pub location: String,
    pub author: String,
    pub content: String,
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("{}, by {} ({})", self.headline, self.author, self.location)
    }
}

pub struct Tweet {
    pub username: String,
    pub content: String,
    pub reply: bool,
    pub retweet: bool,
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("{}: {}", self.username, self.content)
    }
}

//item implements the Summary trait
pub fn Notify(item: impl Summary){
    println!("Breaking news! {}", item.summarize());
}

//T implements the Summary trait (trait bound)
pub fn Notify2<T: Summary>(item: T){
    println!("Breaking news! {}", item.summarize());
}