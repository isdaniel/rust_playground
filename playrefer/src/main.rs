use std::{borrow::BorrowMut, cell::RefCell, ops::{AddAssign, Deref, DerefMut}, rc::{Rc,Weak}};

#[derive(Debug)]
struct Owner{
    name : String,
    tools : RefCell<Vec<Weak<Tool>>>
}

#[derive(Debug)]
struct Tool{
    owner : Rc<Owner>
}

fn main() {
    let daniel = Rc::new(Owner{
        name : "Daniel".to_string(),
        tools : RefCell::new(vec![])
    });

    let pliers = Rc::new(Tool{
        owner : daniel.clone()
    });
    
    let wernch = Rc::new(Tool{
        owner : daniel.clone()
    });

    

    daniel.tools.borrow_mut().push(Rc::downgrade(&pliers));
    daniel.tools.borrow_mut().push(Rc::downgrade(&wernch));

   // println!("pliers : {:?}",daniel.tools.borrow()[0].upgrade().unwrap().owner.name);

   struct S<'bb> {
        x: &'bb mut i32,
    }
    let mut x = 5;
    let s = S { x: &mut x };
    // incr x by 1
    *s.x += 1;
    x+=1;
    println!("x : {x}");

    let mut s = String::from("hello");

    let r1 = &s; // 没问题
    let r2 = &s; // 没问题
    let r3 = &mut s; // 大问题

    println!("{}, {}, and {}", r1, r2, r3);
    //Box::new(5).borrow_mut();

}
