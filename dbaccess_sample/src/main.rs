use std::env;
use serde::{Deserialize, Serialize};
use anyhow::{Ok};
use sqlx::{postgres::PgPool, Executor, FromRow};
use uuid::Uuid;

#[derive(FromRow)]
pub struct T1 {
    pub id: i32,
    pub val: i32,
    pub col1: Uuid,
    pub col2: Uuid,
    pub col3: Uuid,
    pub col4: Uuid,
    pub col5: Uuid,
    pub col6: Uuid,
}

//postgres://postgres:123456@127.0.0.1:5432/dummy
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let postgresql_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&postgresql_url).await?;
    add_t1(&pool,100).await?;
    list_t1(&pool).await?;
    Ok(())
}

async fn add_t1(pool: &PgPool,val : i32) -> anyhow::Result<()> {
    
    let modified_row = sqlx::query(r#"INSERT INTO public.t1(
        val, col1, col2, col3, col4, col5, col6)
        VALUES ($1, $2, $3, $4, $5, $6, $7);"#)
        .bind(val)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .execute(pool).await?;
    
    println!("{:?} rows added", modified_row);
    Ok(())
}

async fn list_t1(pool: &PgPool) -> anyhow::Result<()> {
    let rows = sqlx::query_as::<_,T1>(
        r#"
SELECT id, val, col1, col2, col3, col4, col5, col6
FROM public.t1
LIMIT 100;
        "#
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        println!("{} - {} ({})", row.id, row.val, row.col1);
    }

    Ok(())
}