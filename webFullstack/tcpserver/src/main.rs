use std::{io::{Read, Write}, net::TcpListener};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3001").unwrap();
    println!("Server listening on port 3001");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Connection established!");
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        println!("Data received: {}", String::from_utf8_lossy(&buffer));
        stream.write(&mut buffer).unwrap();        
    }
}
