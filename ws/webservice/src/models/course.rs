use chrono::NaiveDateTime;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone,Deserialize,Serialize)]
pub struct Course{
    pub teacher_id : usize,
    pub id:Option<usize>,
    pub name:String,
    pub time: Option<NaiveDateTime>
}

impl From<web::Json<Course>> for Course{
    fn from(course: web::Json<Course>) -> Self{
        Course{
            teacher_id: course.teacher_id,
            id: course.id,
            name: course.name.clone(),
            time: course.time
        }
    }
}


// use chrono::NaiveDateTime;
// use actix_web::web;
// use serde::{Deserialize, Serialize};

// #[derive(Debug,Deserialize,Clone,Serialize)]
// pub struct Course{
//     pub teacher_id : i32,
//     pub id: i32,
//     pub name:String,
//     pub time: Option<NaiveDateTime>,
//     pub description: Option<String>,
//     pub format: Option<String>,
//     pub duration: Option<String>,
//     pub struction: Option<String>,
//     pub price: Option<f32>,
//     pub language: Option<String>,
//     pub level: Option<String>
// }


// #[derive(Debug, Clone,Serialize)]
// pub struct CreateCourse{
//     pub teacher_id : i32,
//     pub name:String,
//     pub description: Option<String>,
//     pub format: Option<String>,
//     pub duration: Option<String>,
//     pub struction: Option<String>,
//     pub price: Option<f32>,
//     pub language: Option<String>,
//     pub level: Option<String>
// }

// impl From<web::Json<CreateCourse>> for CreateCourse{
//     fn from(course: web::Json<CreateCourse>) -> Self{
//         CreateCourse{
//             teacher_id: course.teacher_id,
//             name: course.name.clone(),
//             description: course.description.clone(),
//             format: course.format.clone(),
//             struction: course.duration.clone(),
//             duration: course.duration.clone(),
//             price: course.price.clone(),
//             language: course.language.clone(),
//             level: course.level.clone()
//         }
//     }
// }
