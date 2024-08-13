use core::fmt;
use actix_web::{error,http::StatusCode,HttpResponse};
use serde::Serialize;

#[derive(Debug,Serialize)]
pub enum MyError{
    DBError(String),
    ActixError(String),
    NotFoundError(String),
    InputInvalidError(String)
}

#[derive(Debug,Serialize)]
pub struct MyErrorResponder{
    pub error_message: String
}

impl MyError {
    pub fn error_response(&self) -> String{
        match self {
            MyError::DBError(message) =>{
                println!("DB Error: {:?}", message);
                "Database error".into()
            }
            MyError::ActixError(message) =>{
                println!("Actix Error: {:?}", message);
                "internal server error".into()
            }
            MyError::NotFoundError(message) =>{
                println!("Not Found Error: {:?}", message);
                message.into()
            }
            MyError::InputInvalidError(message) =>{
                println!("Input Invalid Error: {:?}", message);
                message.into()
            }
        }
    }
}

impl error::ResponseError for MyError{
    fn status_code(&self) -> StatusCode {
        match self {
            MyError::DBError(_) | MyError::ActixError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MyError::NotFoundError(_) => StatusCode::NOT_FOUND,    
            MyError::InputInvalidError(_) => StatusCode::BAD_REQUEST
        }
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .json(MyErrorResponder{error_message: self.error_response()})
            .into()
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self)
    }
}

impl From<actix_web::error::Error> for MyError{
    fn from(error: actix_web::error::Error) -> Self{
        MyError::ActixError(error.to_string())
    }
}
