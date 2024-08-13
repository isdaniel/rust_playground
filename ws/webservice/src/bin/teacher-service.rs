use std::{io, sync::Mutex};
use actix_web::{web,App,HttpResponse,HttpServer,Responder};
use routers::*;
use state::AppState;
use std::env;


#[path = "../handlers/mod.rs"]
mod handlers;
#[path = "../routers.rs"]
mod routers;
#[path = "../state.rs"]
mod state;
#[path = "../models/mod.rs"]
mod models;
#[path = "../errors.rs"]
mod errors;


#[actix_rt::main]
async fn main() -> io::Result<()>{
    let shared_data = web::Data::new(AppState{
        health_check_response: "Actix WebService is running".to_string(),
        visit_count: Mutex::new(0),
        courses: Mutex::new(vec![])
    });
    let app = move || {
        App::new().app_data(shared_data.clone())
            .app_data(web::JsonConfig::default().error_handler(|err,_req| {
                errors::MyError::InputInvalidError(err.to_string()).into()
            }))
            .configure(general_routes)
            .configure(course_routes)
    };
    let _ = HttpServer::new(app).bind("127.0.0.1:3005")?.run().await;
    Ok(())
}