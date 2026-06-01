use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    CreateProductRequest, Product, SearchQuery, SearchResponse, UpdateProductRequest,
};
use crate::{db, search};

pub struct AppState {
    pub pool: PgPool,
    pub es_client: elasticsearch::Elasticsearch,
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn create_product(
    state: web::Data<AppState>,
    body: web::Json<CreateProductRequest>,
) -> HttpResponse {
    let product = match db::insert_product(&state.pool, &body).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to insert product: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create product"
            }));
        }
    };

    if let Err(e) = search::index_product(&state.es_client, &product).await {
        log::warn!("Failed to index product in ES (will be eventually consistent): {}", e);
    }

    HttpResponse::Created().json(&product)
}

pub async fn get_product(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> HttpResponse {
    let id = path.into_inner();

    match db::get_product(&state.pool, id).await {
        Ok(Some(product)) => HttpResponse::Ok().json(&product),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Product not found"
        })),
        Err(e) => {
            log::error!("Failed to get product: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to get product"
            }))
        }
    }
}

pub async fn search_products(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> HttpResponse {
    let es_results = match search::search_products(&state.es_client, &query.q).await {
        Ok(results) => results,
        Err(e) => {
            log::error!("Elasticsearch query failed: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Search failed"
            }));
        }
    };

    if es_results.is_empty() {
        return HttpResponse::Ok().json(SearchResponse {
            products: vec![],
            total: 0,
            query: query.q.clone(),
        });
    }

    let ids: Vec<Uuid> = es_results.iter().map(|(id, _)| *id).collect();

    let products = match db::get_products_by_ids(&state.pool, &ids).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to fetch products from PG: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch search results"
            }));
        }
    };

    let ordered = order_by_es_score(&es_results, &products);

    HttpResponse::Ok().json(SearchResponse {
        total: ordered.len(),
        products: ordered,
        query: query.q.clone(),
    })
}

fn order_by_es_score(es_results: &[(Uuid, f64)], products: &[Product]) -> Vec<Product> {
    let mut ordered: Vec<Product> = Vec::with_capacity(es_results.len());
    for (id, _score) in es_results {
        if let Some(product) = products.iter().find(|p| p.id == *id) {
            ordered.push(product.clone());
        }
    }
    ordered
}

pub async fn update_product(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateProductRequest>,
) -> HttpResponse {
    let id = path.into_inner();

    match db::update_product(&state.pool, id, &body).await {
        Ok(Some(product)) => {
            if let Err(e) = search::index_product(&state.es_client, &product).await {
                log::warn!("Failed to reindex product in ES: {}", e);
            }
            HttpResponse::Ok().json(&product)
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Product not found"
        })),
        Err(e) => {
            log::error!("Failed to update product: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to update product"
            }))
        }
    }
}

pub async fn delete_product(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> HttpResponse {
    let id = path.into_inner();

    match db::delete_product(&state.pool, id).await {
        Ok(true) => {
            if let Err(e) = search::delete_product(&state.es_client, id).await {
                log::warn!("Failed to remove product from ES: {}", e);
            }
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Product deleted"
            }))
        }
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Product not found"
        })),
        Err(e) => {
            log::error!("Failed to delete product: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to delete product"
            }))
        }
    }
}

pub async fn seed_products(state: web::Data<AppState>) -> HttpResponse {
    let seed_data = vec![
        CreateProductRequest {
            name: "iPhone 15 Pro Max".to_string(),
            description: "Apple's flagship smartphone with A17 Pro chip, titanium design, and advanced camera system".to_string(),
            category: "Electronics".to_string(),
            price: 1199.0,
        },
        CreateProductRequest {
            name: "Samsung Galaxy S24 Ultra".to_string(),
            description: "Samsung premium smartphone with Galaxy AI, S Pen, and 200MP camera".to_string(),
            category: "Electronics".to_string(),
            price: 1299.0,
        },
        CreateProductRequest {
            name: "Sony WH-1000XM5 Headphones".to_string(),
            description: "Industry-leading noise canceling wireless headphones with exceptional sound quality".to_string(),
            category: "Electronics".to_string(),
            price: 349.0,
        },
        CreateProductRequest {
            name: "The Rust Programming Language".to_string(),
            description: "Official Rust book covering ownership, borrowing, lifetimes, and systems programming".to_string(),
            category: "Books".to_string(),
            price: 39.99,
        },
        CreateProductRequest {
            name: "Designing Data-Intensive Applications".to_string(),
            description: "A deep dive into distributed systems, databases, and data processing architectures by Martin Kleppmann".to_string(),
            category: "Books".to_string(),
            price: 45.99,
        },
        CreateProductRequest {
            name: "Nike Air Max 270".to_string(),
            description: "Comfortable running shoes with Max Air unit for all-day cushioning and breathable mesh upper".to_string(),
            category: "Clothing".to_string(),
            price: 150.0,
        },
        CreateProductRequest {
            name: "Patagonia Better Sweater Jacket".to_string(),
            description: "Warm fleece jacket made from recycled polyester with Fair Trade Certified sewn".to_string(),
            category: "Clothing".to_string(),
            price: 139.0,
        },
        CreateProductRequest {
            name: "Organic Matcha Green Tea Powder".to_string(),
            description: "Premium Japanese ceremonial grade matcha, stone-ground for smooth taste".to_string(),
            category: "Food".to_string(),
            price: 29.99,
        },
        CreateProductRequest {
            name: "Wilson Evolution Basketball".to_string(),
            description: "Premium indoor game basketball with moisture-absorbing composite leather cover".to_string(),
            category: "Sports".to_string(),
            price: 65.0,
        },
        CreateProductRequest {
            name: "PostgreSQL 高效能資料庫管理".to_string(),
            description: "深入探討 PostgreSQL 效能調校、索引優化與查詢計畫分析的進階指南".to_string(),
            category: "Books".to_string(),
            price: 52.0,
        },
    ];

    let mut products: Vec<Product> = Vec::with_capacity(seed_data.len());

    for req in &seed_data {
        match db::insert_product(&state.pool, req).await {
            Ok(product) => products.push(product),
            Err(e) => {
                log::error!("Failed to seed product '{}': {}", req.name, e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to seed product: {}", e)
                }));
            }
        }
    }

    if let Err(e) = search::bulk_index_products(&state.es_client, &products).await {
        log::warn!("Failed to bulk index seed data into ES: {}", e);
    }

    HttpResponse::Created().json(serde_json::json!({
        "message": format!("Seeded {} products", products.len()),
        "products": products
    }))
}
