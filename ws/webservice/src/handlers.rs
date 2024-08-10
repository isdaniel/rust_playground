use crate::models::Course;
use super::state::AppState;
use actix_web::{web,HttpResponse};
use chrono::Utc;

pub async fn health_check_handler(app_state : web::Data<AppState>) -> HttpResponse{
    let health_check_response = &app_state.health_check_response;
    let mut visitor_count = app_state.visit_count.lock().unwrap();
    let res = format!("{} - Visitor Count: {}", health_check_response, *visitor_count);
    *visitor_count +=1;
    HttpResponse::Ok().json(&res)
}

pub async fn new_course_handler(
    app_state : web::Data<AppState>, 
    new_course: web::Json<Course>
) -> HttpResponse{
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
    HttpResponse::Ok().json("Course created")
}

pub async fn get_courses_for_teacher_handler(
    app_state : web::Data<AppState>,
    teacher_id: web::Path<usize>
) -> HttpResponse{
    let courses = app_state.courses
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .filter(|course| course.teacher_id == *teacher_id)
        .collect::<Vec<Course>>();
    if courses.len() == 0{
        return HttpResponse::Ok().json("No courses found for teacher");    
    }

    HttpResponse::Ok().json(courses)
}

pub async fn new_course_detail_handler(
    app_state : web::Data<AppState>,
    params: web::Path<(usize,usize)>
) -> HttpResponse{
    let (teacher_id,course_id) = *params;
    let selected_course = app_state.courses
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .find(|c| c.teacher_id == teacher_id && c.id == Some(course_id))
        .ok_or("Course not found");
    
    if let Ok(course) = selected_course{
        HttpResponse::Ok().json(course)
    } else {
        HttpResponse::NotFound().json("Course not found")
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{body::to_bytes,web};
    use crate::models::Course;
    use crate::state::AppState;
    use std::sync::Mutex;

    #[actix_rt::test]
    async fn test_health_check_handler(){
        let app_state = web::Data::new(AppState{
            health_check_response: "Actix WebService is running".to_string(),
            visit_count: Mutex::new(0),
            courses: Mutex::new(vec![])
        });

        let resp = health_check_handler(app_state).await;
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
        let resp = new_course_handler(app_state.clone(),req).await;
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
        assert_eq!(resp.status(), StatusCode::OK);
    }

}
