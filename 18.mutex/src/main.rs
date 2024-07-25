use std::sync::{Mutex,Arc};

fn main() {
    let m = Mutex::new(5);

    {
        let mut num: std::sync::MutexGuard<i32> = m.lock().unwrap();
        *num = 6;
        //out of scope will release lock
    }

    println!("m = {:?}", m);
    println!("====================");
    let counter = Arc::new(Mutex::new(0));   
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = std::thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Result: {}", *counter.lock().unwrap());
    println!("====================");
}
