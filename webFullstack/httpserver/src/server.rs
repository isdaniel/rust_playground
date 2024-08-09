use std::{io::Read, net::TcpListener};

use super::router::Router;
use http::httprequest::HttpRequest;


pub struct Server<'a> {
    socker_addr: &'a str,
}

impl<'a> Server<'a>{
    pub fn new(socker_addr : &'a str) -> Self{
        Server{
            socker_addr
        }
    }

    pub fn run(&self){
        let conection_lisiner = TcpListener::bind(self.socker_addr).unwrap();
        println!("Server is running on {}", self.socker_addr);

        for steam in conection_lisiner.incoming(){
            let mut stream = steam.unwrap();
            println!("Connection established!");
            let mut buffer = [0; 1024];
            stream.read(&mut buffer).unwrap();
            let req : HttpRequest = String::from_utf8(buffer.to_vec()).unwrap().into();
            Router::route(req,&mut stream);
        }
    }
}
