mod config;
mod db;
mod handlers;
mod models;
mod search;

use actix_web::{web, App, HttpServer};
use handlers::AppState;
use sqlx::postgres::PgPoolOptions;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = config::Config::from_env();

    log::info!("Connecting to PostgreSQL...");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    db::init_db(&pool).await.expect("Failed to initialize database");

    log::info!("Connecting to Elasticsearch at {}...", config.elasticsearch_url);
    let es_client = search::create_client(&config.elasticsearch_url)
        .expect("Failed to create Elasticsearch client");

    search::init_index(&es_client)
        .await
        .expect("Failed to initialize Elasticsearch index");

    let state = web::Data::new(AppState {
        pool,
        es_client,
    });

    log::info!(
        "Starting server at {}:{}",
        config.server_host,
        config.server_port
    );

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/health", web::get().to(handlers::health))
            .service(
                web::scope("/api/products")
                    .route("/search", web::get().to(handlers::search_products))
                    .route("/seed", web::post().to(handlers::seed_products))
                    .route("", web::post().to(handlers::create_product))
                    .route("/{id}", web::get().to(handlers::get_product))
                    .route("/{id}", web::put().to(handlers::update_product))
                    .route("/{id}", web::delete().to(handlers::delete_product)),
            )
    })
    .bind(format!("{}:{}", config.server_host, config.server_port))?
    .run()
    .await
}
