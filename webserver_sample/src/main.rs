use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::fs;
use webserver_sample::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8811").unwrap();
    let pool = ThreadPool::new(4);
    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    match stream.read(&mut buffer) {
        Ok(_) => {
            let get = b"GET / HTTP/1.1\r\n";
            let sleep = b"GET /sleep HTTP/1.1\r\n";
            let (status_line, filename) = if buffer.starts_with(get) {
                ("HTTP/1.1 200 OK\r\n", "hello.html")
            } else if buffer.starts_with(sleep) {
                std::thread::sleep(std::time::Duration::from_secs(5));
                ("HTTP/1.1 200 OK\r\n", "hello.html")
            } else {
                ("HTTP/1.1 404 NOT FOUND\r\n", "404.html")
            };

            let contents = fs::read_to_string(filename).unwrap_or_else(|err| {
                println!("Error reading file: {}", err);
                String::from("File not found")
            });

            let response = format!("{}Content-Length: {}\r\n\r\n{}", status_line, contents.len(), contents);
            //println!("Response: {}", response);
            stream.write(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        },
        Err(e) => {
            println!("Error reading from stream: {}", e);
        }
    }
}