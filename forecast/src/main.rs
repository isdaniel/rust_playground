//https://www.shuttle.rs/blog/2023/09/27/rust-vs-go-comparison
use askama_axum::Template;
use forecast::*;
use axum::{
     http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
    extract::Query,
};
use axum_macros::debug_handler;
use anyhow::Context;

async fn weather(Query(params): Query<WeatherQuery>) -> Result<WeatherDisplay, AppError> {
	let lat_long = fetch_lat_long(&params.city).await?;
	let weather = fetch_weather(lat_long).await?;
	Ok(WeatherDisplay::new(params.city, weather))
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate<'a> {
    name: &'a str,
}

#[debug_handler]
async fn index() -> HelloTemplate<'static> {
    HelloTemplate { name: "world" }
}

async fn stats(user: User) -> Result<&'static str, AppError> {
	Ok("We're authorized!")
}


// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    // let conn_str = "postgres://postgres:pwd123456@localhost:5432/forecast?sslmode=disable";
	// let pool = sqlx::PgPool::connect(&conn_str)
    //     .await
    //     .context("can't connect to database")?;

    let app = Router::new()
    .route("/", get(index))
    .route("/weather", get(weather))
    .route("/stats", get(stats));
    //.with_state(pool);


    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
