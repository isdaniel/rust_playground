use ctrlc;
use std::sync::mpsc::*;
use std::sync::{Arc, Mutex};
use std::thread::{self};
use std::time::Duration;
use std::vec;


enum Message {
    NewJob,
    Terminate
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewJob => {
                    println!("Worker {} got a job; executing.", id);
                    let _ = Duration::from_secs(1);
                },
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });
        
        Worker {
            id,
            thread: Some(thread),
        }
    }
}

fn main() {
    let (sender, receiver) = channel();
    let (main_sender, main_receiver) = channel();
    let mut workers = vec![];
    let receiver = Arc::new(Mutex::new(receiver));
    for i in 1..10 {
        workers.push(Worker::new(i,Arc::clone(&receiver)));
        sender.send(Message::NewJob).expect("Could not send signal on channel.");
    }
    
    ctrlc::set_handler(move || {
        for _ in &workers {
            sender.send(Message::Terminate).expect("Could not send signal on channel.");
        }
        
        for worker in &mut workers {
            match worker.thread.take() {
                Some(handle) => handle.join().unwrap(),
                None => {
                    println!("Worker {} already terminated.", worker.id);
                },
            }
        }
        println!("Got it! Exiting...");
        main_sender.send(()).expect("main_sender could not send signal on channel.");
    }).expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    main_receiver.recv().expect("main_receiver could not receive signal on channel.");
}
