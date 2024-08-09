use http::httprequest::{self, Resource};
use http::{httprequest::HttpRequest, httpresponse::HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;

pub trait Handler {
    fn handle(req: &HttpRequest) -> HttpResponse;
    fn load_file(file_path: &str) -> Option<String> {
        let default_path = format!("{}/public",env!("CARGO_MANIFEST_DIR"));
        let public_path = env::var("PUBLIC_PATH").unwrap_or(default_path);
        let full_path = format!("{}/{}", public_path, file_path);

        let contents = fs::read_to_string(full_path);
        contents.ok()
    }
}

pub struct WebServiceHandler;
pub struct PageNotFoundHandler;
pub struct StaticPageHandler;

#[derive(Serialize, Deserialize)]  
pub  struct OrderStatus {
    pub id: i32,
    pub date:String,
    pub status: String,
}


impl Handler for PageNotFoundHandler{
    fn handle(req: &HttpRequest) -> HttpResponse {
        HttpResponse::new("404", None, Self::load_file("404.html"))
    }
}

impl Handler for StaticPageHandler{
    fn handle(req: &HttpRequest) -> HttpResponse {
        let Resource::Path(s) = &req.resource else{todo!()};
        let route:Vec<&str> = s.split("/").collect();
        match route[1]{
            "" => HttpResponse::new("200", None, Self::load_file("index.html")),
            "health" => HttpResponse::new("200", None, Self::load_file("health.html")),
            path => match Self::load_file(path){
                Some(content) => {
                    let mut map: HashMap<&str, &str> = HashMap::new();
                    if path.ends_with(".css") {
                        map.insert("Content-Type", "text/css");
                    } else if path.ends_with(".js") {
                        map.insert("Content-Type", "application/javascript");
                    } else {
                        map.insert("Content-Type", "text/html");
                    }

                    HttpResponse::new("200",Some(map),Some(content))
                },
                None => HttpResponse::new("404", None, Self::load_file("404.html")),
            }
        }
    }
}

impl Handler for WebServiceHandler{
    fn handle(req: &HttpRequest) -> HttpResponse {
        let httprequest::Resource::Path(s) = &req.resource else{todo!()};
        let route : Vec<&str> = s.split("/").collect();

        match route[2]{
            "shipping" if route.len() > 2 && route[3] == "orders" => {
                let orders = Self::load_json();
                let json = serde_json::to_string(&orders).unwrap();
                let mut map: HashMap<&str, &str> = HashMap::new();
                map.insert("Content-Type", "application/json");
                HttpResponse::new("200", Some(map), Some(json))
            },
            _ => HttpResponse::new("404", None, Self::load_file("404.html")),
        }
    }
}


impl WebServiceHandler{
    fn load_json() -> Vec<OrderStatus>{
        let default_path = format!("{}/data",env!("CARGO_MANIFEST_DIR"));
        let data_path = env::var("DATA_PATH").unwrap_or(default_path);
        let full_path = format!("{}/{}", data_path, "orders.json");
        let json_contents = fs::read_to_string(full_path).expect("Unable to read file");
        let order : Vec<OrderStatus> = serde_json::from_str(&json_contents.as_str()).expect("Unable to parse json");
        order
    }
    
}