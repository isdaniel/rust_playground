use actix_web::{web,App,HttpResponse,HttpServer,Responder};
use super::handlers::*;

pub fn general_routes(cfg:&mut web::ServiceConfig){
    cfg.route("/health", web::get().to(health_check_handler));
}

pub fn course_routes(cfg:&mut web::ServiceConfig){
    cfg.route("/courses", web::post().to(new_course_handler))
        .route("/courses/{teacher_id}", web::get().to(get_courses_for_teacher_handler))
        .route("/courses/{user_id}/{teacher_id}", web::get().to(new_course_detail_handler));
}