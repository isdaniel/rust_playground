use sqlx::postgres::PgPool;


// pub async fn get_courses_for_teacher_db(pool : &PgPool, teacher_id: i32) -> Result<Vec<Course>, sqlx::Error>{
//     let result: Vec<Course> = sqlx::query_as!(
//         Course,
//         r#"SELECT id,teacher_id,name,time FROM courses WHERE teacher_id = $1"#,
//         teacher_id
//     )
//     .fetch_all(pool)
//     .await?;
//     Ok(result)
// }