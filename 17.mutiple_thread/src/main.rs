use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let thread1: thread::JoinHandle<()> = thread::spawn(|| {
        for i in 1..10 {
            println!("Hi number {} from the spawned thread!", i);
            thread::sleep(Duration::from_millis(1));
        }
    });

    for i in 1..5 {
        println!("Hi number {} from the main thread!", i);
        thread::sleep(Duration::from_millis(1));
    }

    thread1.join().unwrap();
    println!("=====================");

    let v = vec![1, 2, 3];
    //move keyword is used to move the ownership of the variable v to the spawned thread
    let handle = thread::spawn(move || {
        println!("Here's a vector: {:?}", v);
    });
    handle.join().unwrap();
    println!("=====================");
    
    let (tx,rx) = mpsc::channel();

    thread::spawn(move || {
        let val = String::from("message from thread1!");
        tx.send(val).unwrap();
    });


    match rx.recv(){
        Ok(msg) => println!("Got: {}", msg),
        Err(_) => println!("Error"),    
    }
    println!("=====================");

    let (tx,rx) = mpsc::channel();

    thread::spawn(move || {
        let vals = vec![
            String::from("hi"),
            String::from("from"),
            String::from("thread2"),
        ];
        for val in vals {
            tx.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    for received in rx {
        println!("Got: {}", received);
    }
    println!("=====================");

    let (tx,rx) = mpsc::channel();
    let tx1 = mpsc::Sender::clone(&tx);
    thread::spawn(move || {
        let vals = vec![
            String::from("1. hi"),
            String::from("1. from"),
            String::from("1. tx"),
        ];
        for val in vals {
            tx.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    thread::spawn(move || {
        let vals = vec![
            String::from("more"),
            String::from("messages"),
            String::from("for tx1"),
        ];
        for val in vals {
            tx1.send(val).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    });

    for received in rx {
        println!("Got: {}", received);
    }
}
