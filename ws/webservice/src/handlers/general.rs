use crate::{errors::MyError, models::course::Course};
use crate::state::AppState;
use actix_web::{web,HttpResponse};
use chrono::Utc;

pub async fn health_check_handler(app_state : web::Data<AppState>) -> Result<HttpResponse,MyError>{
    let health_check_response = &app_state.health_check_response;
    let mut visitor_count = app_state.visit_count.lock().unwrap();
    let res = format!("{} - Visitor Count: {}", health_check_response, *visitor_count);
    *visitor_count +=1;
    Ok(HttpResponse::Ok().json(&res))
}