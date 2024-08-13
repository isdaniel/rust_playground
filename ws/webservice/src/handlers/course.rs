use crate::{errors::MyError, models::course::Course};
use crate::state::AppState;
use actix_web::{web,HttpResponse};
use chrono::Utc;


pub async fn new_course_handler(
    app_state : web::Data<AppState>, 
    new_course: web::Json<Course>
) -> Result<HttpResponse,MyError>{
    println!("Received course: {:?}", new_course);
    let course_count = app_state.courses
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .filter(|course| course.teacher_id == new_course.teacher_id)
        .collect::<Vec<Course>>()
        .len();
    let new_course = Course{
        teacher_id: new_course.teacher_id,
        id: Some(course_count + 1),
        name: new_course.name.clone(),
        time: Some(Utc::now().naive_utc())
    };
    app_state.courses.lock().unwrap().push(new_course);
    Ok(HttpResponse::Ok().json("Course created"))
}

pub async fn get_courses_for_teacher_handler(
    app_state : web::Data<AppState>,
    teacher_id: web::Path<usize>
) -> Result<HttpResponse,MyError>{
    let courses = app_state.courses
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .filter(|course| course.teacher_id == *teacher_id)
        .collect::<Vec<Course>>();

    match courses.len() {
        0 => Err(MyError::NotFoundError("couses not found".into())),
        _ => Ok(HttpResponse::Ok().json(courses))
    }
}

pub async fn new_course_detail_handler(
    app_state : web::Data<AppState>,
    params: web::Path<(usize,usize)>
) -> Result<HttpResponse,MyError>{
    let (teacher_id,course_id) = *params;
    let selected_course = app_state.courses
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .find(|c| c.teacher_id == teacher_id && c.id == Some(course_id))
        .ok_or("Course not found");

    match selected_course{
        Ok(course) => Ok(HttpResponse::Ok().json(course)),
        Err(e) => Err(MyError::NotFoundError(e.to_string()))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::ResponseError;
    use actix_web::{body::to_bytes,web};
    use crate::models::course::Course;
    use crate::handlers::general::health_check_handler;
    use crate::state::AppState;
    use std::sync::Mutex;

    #[actix_rt::test]
    async fn test_health_check_handler(){
        let app_state = web::Data::new(AppState{
            health_check_response: "Actix WebService is running".to_string(),
            visit_count: Mutex::new(0),
            courses: Mutex::new(vec![])
        });

        let resp = health_check_handler(app_state).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(body, r##""Actix WebService is running - Visitor Count: 0""##);
    }

    #[actix_rt::test]
    async fn test_new_course_handler(){
        let app_state = web::Data::new(AppState{
            health_check_response: "Actix WebService is running".to_string(),
            visit_count: Mutex::new(0),
            courses: Mutex::new(vec![])
        });
        let new_course = Course{
            teacher_id: 1,
            id: None,
            name: "Test Course".to_string(),
            time: None
        };

        let req = web::Json(new_course);
        let resp = new_course_handler(app_state.clone(),req).await.unwrap();
        let courses = app_state.courses.lock().unwrap().clone();
        let expect_status = resp.status();
        let body = to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(expect_status, StatusCode::OK);
        assert_eq!(body, r##""Course created""##);
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].teacher_id, 1);
        assert_eq!(courses[0].name, "Test Course");
        assert!(courses[0].id.is_some());
        assert!(courses[0].time.is_some());
    }

    #[actix_rt::test]
    async fn test_get_courses_for_teacher_handler(){
        let app_state = web::Data::new(AppState{
            health_check_response: "Actix WebService is running".to_string(),
            visit_count: Mutex::new(0),
            courses: Mutex::new(vec![])
        });

        let teacher_id = web::Path::from(1);
        let resp = get_courses_for_teacher_handler(app_state, teacher_id).await;
        //assert_eq!(resp.err().unwrap().to_string(), "NotFoundError: couses not found");
        assert_eq!(resp.err().unwrap().status_code(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_new_course_detail_handler(){
        let app_state = web::Data::new(AppState{
            health_check_response: "Actix WebService is running".to_string(),
            visit_count: Mutex::new(0),
            courses: Mutex::new(vec![])
        });
        let new_course = Course{
            teacher_id: 1,
            id: None,
            name: "Test Course".to_string(),
            time: None
        };
        let req = web::Json(new_course);
        let _ = new_course_handler(app_state.clone(),req).await;

        let params = web::Path::from((1,1));
        let resp = new_course_detail_handler(app_state, params).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
