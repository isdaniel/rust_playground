use std::io::{Read, Write};
use std::str;


fn main() {
    let mut stream = std::net::TcpStream::connect("localhost:3001").unwrap();
    stream.write("Hello, world!".as_bytes()).unwrap();
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    println!("Response: {}", str::from_utf8(&buffer).unwrap());
}
