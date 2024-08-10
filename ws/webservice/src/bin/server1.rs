use std::io;
use actix_web::{web,App,HttpResponse,HttpServer,Responder};


pub fn general_routes(cfg:&mut web::ServiceConfig){
    cfg.route("/health", web::get().to(health_check_handler));
}

pub async fn health_check_handler() -> impl Responder{
    HttpResponse::Ok().json("Actix WebService is running")
}

#[actix_rt::main]
async fn main() -> io::Result<()>{
    let app = move || App::new().configure(general_routes);
    let _ = HttpServer::new(app).bind("127.0.0.1:3005")?.run().await;
    Ok(())
}