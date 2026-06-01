use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateProductRequest, Product, UpdateProductRequest};

pub async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products (
            id UUID PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT NOT NULL,
            category VARCHAR(100) NOT NULL,
            price DOUBLE PRECISION NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    log::info!("Database initialized successfully");
    Ok(())
}

pub async fn insert_product(
    pool: &PgPool,
    req: &CreateProductRequest,
) -> Result<Product, sqlx::Error> {
    let id = Uuid::new_v4();
    let product = sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (id, name, description, category, price)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, name, description, category, price, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.category)
    .bind(req.price)
    .fetch_one(pool)
    .await?;

    Ok(product)
}

pub async fn get_product(pool: &PgPool, id: Uuid) -> Result<Option<Product>, sqlx::Error> {
    let product = sqlx::query_as::<_, Product>(
        "SELECT id, name, description, category, price, created_at, updated_at FROM products WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(product)
}

pub async fn get_products_by_ids(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<Vec<Product>, sqlx::Error> {
    let products = sqlx::query_as::<_, Product>(
        "SELECT id, name, description, category, price, created_at, updated_at FROM products WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(products)
}

pub async fn update_product(
    pool: &PgPool,
    id: Uuid,
    req: &UpdateProductRequest,
) -> Result<Option<Product>, sqlx::Error> {
    let existing = get_product(pool, id).await?;
    if existing.is_none() {
        return Ok(None);
    }
    let existing = existing.unwrap();

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let description = req.description.as_deref().unwrap_or(&existing.description);
    let category = req.category.as_deref().unwrap_or(&existing.category);
    let price = req.price.unwrap_or(existing.price);

    let product = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products
        SET name = $2, description = $3, category = $4, price = $5, updated_at = NOW()
        WHERE id = $1
        RETURNING id, name, description, category, price, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(category)
    .bind(price)
    .fetch_optional(pool)
    .await?;

    Ok(product)
}

pub async fn delete_product(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
