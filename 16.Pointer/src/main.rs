use crate::List::{Cons,Nil};
use crate::RcList::{ConsRc,Nil as RcNil};
use std::cell::RefCell;
use crate::ReCellList::{ConsList,Nil as ReCellListNil};
use std::ops::Deref;
use std::rc::Rc;

struct MyBox<T>(T);

impl<T> MyBox<T> {
    fn new(x: T) -> MyBox<T> {
        MyBox(x)
    }
}

impl<T> Deref for MyBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

fn main() {
    let b = Box::new(5);
    println!("b = {}",b);
    println!("===================");
    let list = Cons(1, Box::new(Cons(2, Box::new(Cons(3, Box::new(Nil))))));

    let a = 5;
    let b = &a;
    assert_eq!(5, a);
    assert_eq!(5, *b);

    let b = Box::new(a);  
    assert_eq!(5, *b);

    let b = MyBox::new(a);
    assert_eq!(5, *b);
    //*b == *(b.deref())
    println!("===================");
    let m = MyBox::new(String::from("Rust"));
    hello(&m);
    println!("===================");
    let c = CustomSmartPointer { data: String::from("my stuff") };
    let d = CustomSmartPointer { data: String::from("other stuff") };
    println!("CustomSmartPointer created.");
    println!("===================");

    let a = Rc::new(ConsRc(5, Rc::new(ConsRc(10, Rc::new(RcNil)))));

    println!("count after creating a = {}", Rc::strong_count(&a));
    // b & c both point to the same memory location as a
    let b = ConsRc(3, Rc::clone(&a));

    println!("count after creating b = {}", Rc::strong_count(&a));
    {
        let c = ConsRc(4, Rc::clone(&a));
        println!("count after creating c = {}", Rc::strong_count(&a));
    }
    println!("count after c goes out of scope = {}", Rc::strong_count(&a));
    println!("===================");
    //RefCell<T>

    let value = Rc::new(RefCell::new(5));
    let a = Rc::new(ConsList(Rc::clone(&value), Rc::new(ReCellListNil)));
    let b = ConsList(Rc::new(RefCell::new(6)), Rc::clone(&a));
    let c = ConsList(Rc::new(RefCell::new(10)), Rc::clone(&a));
    *value.borrow_mut() += 10;

    println!("a after = {:?}", a);
    println!("b after = {:?}", b);
    println!("c after = {:?}", c);
}

#[derive(Debug)]
enum ReCellList {
    ConsList(Rc<RefCell<i32>>, Rc<ReCellList>),
    Nil,
}


enum List {
    Cons(i32, Box<List>),
    Nil,
}

enum RcList {
    ConsRc(i32, Rc<RcList>),
    Nil,
}

fn hello(name: &str) {
    println!("Hello, {}!", name);
}


struct CustomSmartPointer {
    data: String,
}

impl Drop for CustomSmartPointer {
    fn drop(&mut self) {
        println!("Dropping CustomSmartPointer with data `{}`!", self.data);
    }
}
