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
            let result = Worker::wait_for_message(&receiver, Duration::from_millis(500));
            match result {
                Some(Message::NewJob) => {
                    println!("Worker {} got a job; executing.", id);
                },
                Some(Message::Terminate) => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
                None => {
                    println!("unexpected error, Worker {} could not receive message.", id);
                    break;
                }
            }
        });
        
        Worker {
            id,
            thread: Some(thread),
        }
    }

    fn wait_for_message(receiver: &Arc<Mutex<Receiver<Message>>>,timeout: Duration) -> Option<Message>{
        let message = receiver.lock().unwrap();
        match message.recv_timeout(timeout) {
            Ok(message) => Some(message),
            Err(RecvTimeoutError::Timeout) => {
                Some(Message::NewJob)
            },
            Err(RecvTimeoutError::Disconnected) => {
                println!("Channel disconnected.");
                None
            }
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
        main_sender.send("Got it! Exiting...").expect("main_sender could not send signal on channel.");
    }).expect("Error setting Ctrl-C handler");

    println!("Waiting for Ctrl-C...");
    println!("{}", main_receiver.recv().expect("main_receiver could not receive signal on channel."));
}
