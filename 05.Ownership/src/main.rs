fn print_split_line() {
    println!("=================="); 
}

fn main() {
    let mut s = String::from("hello");
    s.push_str(", world!");
    println!("{}", s);
    print_split_line();

    //move reference type
    let s1 = String::from("Hello");
    let s2 = s1;
    //println!("{}", s1); // Error: value borrowed here after move
    println!("{}", s2);

    let s = String::from("hello");
    let s = take_owership(s);
    //println!("{}", s); // Error: value borrowed here after move
    let x = 5;
    make_copy(x);
    println!("copy from value {}", x);
    print_split_line();

    // let s1 = gives_owner_ship();
    // let s2 = String::from("hello");
    // let s3 = take_and_give_back(s2);
    
    //referance
    let s1 = String::from("Hello World!");
    let len = string_length(&s1);
    //s1 still valid, because we don't move ownership, and it's borrowed.
    println!("The length of '{}' is {}", s1, len);
    print_split_line();

    let mut s1 = s1;
    let len = string_mut_length(&mut s1);
    println!("The length of '{}' is {}", s1, len);
    print_split_line();

    //slice of string
    let mut s = String::from("hello world");
    let word = first_works(&s);
    println!("first word is {}, second word is {}", &s[..word], &s[word..]);
    println!("first word is {}", first_works_slice(&s));

    let arr = [1,2,3,4,5];
    let slice: &[i32] = &arr[1..3];
    sample(slice);
}

fn sample(arr : &[i32]){
    for i in arr.iter() {
        print!("{} ", i);
    }
}

fn take_owership(mut str : String) {
    println!("{}", str);
    str.push_str(" world!!");
}

fn make_copy(num : i32){
    println!("{}", num);
}

fn gives_owner_ship() -> String {
    let s = String::from("hello");
    s
}

fn take_and_give_back(str : String) -> String {
    str
}

fn string_length(str : &String) -> usize {
    //str.push_str(" world!!"); //Error: cannot borrow `*str` as mutable, as it is behind a `&` reference
    str.len()
}


fn string_mut_length(str : &mut String) -> usize {
    str.push_str(" world!!"); 
    str.len()
}

//rust not allow to return reference of local variable (to avoid dangling reference)
// fn dangle() -> &String {
//     let s = String::from("hello");
//     &s
// }

fn first_works(s:&str) -> usize {
    let bytes = s.as_bytes();
    for (i,&item) in bytes.iter().enumerate() {
        if item == b' ' {
            return i;
            
        }
    }

    s.len()
}

//parameter is &str, so it's slice of string
fn first_works_slice(s:&str) -> &str {
    let bytes = s.as_bytes();
    for (i,&item) in bytes.iter().enumerate() {
        if item == b' ' {
            return &s[..i]
            
        }
    }

    &s[..]
}

