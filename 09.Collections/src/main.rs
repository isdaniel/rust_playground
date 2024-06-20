use std::fmt::Debug;
use std::collections::HashMap;

enum SpreadsheetsCell {
    Int(i32),
    Float(f64),
    Text(String),
}

impl SpreadsheetsCell {
    fn to_string(&self) -> String {
        match self {
            SpreadsheetsCell::Int(i) => i.to_string(),
            SpreadsheetsCell::Float(f) => f.to_string(),
            SpreadsheetsCell::Text(s) => s.clone(),
        }
    }
}

fn main() {
    //let list: Vec<i32> = Vec::new();
    let mut v = vec![1,2,3];
    v.push(4);

    for i in &v {
        println!("{}", i);
    }

    println!("The second element is {}", &v[1]);
    match v.get(1) {
        Some(second) => println!("The second element is {}", second),
        None => println!("There is no second element."),
    }
    //&v[100] //error, panic!

    println!("==================");
    let row = vec![
        SpreadsheetsCell::Int(3),
        SpreadsheetsCell::Text(String::from("blue")),
        SpreadsheetsCell::Float(10.12),
    ];

    for i in &row {
        println!("{}", i.to_string());
    }
    println!("=================="); //string
    let data = "initial contents";
    let mut s = data.to_string();
    s.push_str("!!");
    s.push('@');
    println!("{s}");

    let s1 = String::from("Hello, ");
    let s2 = String::from("world!");

    //fn add(self, s: &str) -> String 
    let s3 = s1 + &s2; // note s1 has been moved here and can no longer be used
    println!("{s3}");
    //println!("{s1}"); //error

    let s1 = String::from("tic");
    let s2 = String::from("tac");
    let s3 = String::from("toe");

    let s = format!("{s1}-{s2}-{s3}");
    println!("{s}");
    println!("=================="); 
    let hello = "Здравствуйте";
    let s = &hello[0..4];
    //let s = &hello[0..1]; //error, panic
    println!("{s}");
    println!("=================="); //hashmap
    let mut scores:HashMap<String,i32> = HashMap::new();
    scores.insert(String::from("Blue"), 10);


    let teams = vec![String::from("Blue"), String::from("Yellow")];
    let initial_scores = vec![10,50];

    let scores: HashMap<_,_> = teams.iter().zip(initial_scores.iter()).collect();

    let field_name = String::from("Favorite color");
    let field_value = String::from("Blue");

    let mut map = HashMap::new();
    // map.insert(field_name, field_value); //lead to error. 
    map.insert(&field_name, &field_value); 
    println!("{field_name}{field_value}"); 

    let scores = map.get(&field_name);
    match scores {
        Some(score) => println!("{score}"),
        None => println!("None"),
    }

    for (key, value) in &map {
        println!("{key} : {value}");
    }
    let binding = String::from("Red");
    map.insert(&field_name, &binding); //update value by key
    println!("{:?}",map);
    map.entry(&String::from("Yellow")).or_insert(&String::from("Yellow Value")); //Try to insert 50 if Yellow is not exist
    
    let text = "hello world wonderful world";
    let mut map = HashMap::new();

    for word in text.split_whitespace(){
        let count = map.entry(word).or_insert(0);
        *count += 1;
    } 
    
    println!("{:?}",map);
}   
