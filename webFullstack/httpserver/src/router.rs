use std::{io::Write, net::TcpStream};
use http::httprequest::*;
use http::httpresponse::*;
use super::handler::*;

pub struct Router;

impl Router {
    pub fn route(req: HttpRequest, stream: &mut impl Write){
        match req.method{
            Method::GET => match &req.resource {
                Resource::Path(s) =>{
                    let route:Vec<&str> = s.split("/").collect();
                    match route[1] {
                        "api" => {
                            let resp: HttpResponse = WebServiceHandler::handle(&req);
                            let _ = resp.send_response(stream);
                        },
                        _ => {
                            let resp: HttpResponse = StaticPageHandler::handle(&req);
                            let _ = resp.send_response(stream);
                        }
                    }
                },
                _ => {
                    let resp: HttpResponse = PageNotFoundHandler::handle(&req);
                    let _ = resp.send_response(stream);
                }
            }
            _ => {
                let resp: HttpResponse = PageNotFoundHandler::handle(&req);
                let _ = resp.send_response(stream);
            }
        }
    }
}