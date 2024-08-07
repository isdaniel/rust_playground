
use anyhow::Context;
use askama_axum::Template;
use serde::Deserialize;
use axum::{
    async_trait, extract::FromRequestParts, http::{request::Parts, StatusCode}, response::{IntoResponse, Response}
};
use std::str::from_utf8;
use base64;

#[derive(Deserialize, Debug, Clone)]
pub struct GeoResponse {
    pub results: Vec<LatLong>,
}

#[derive(Deserialize)]
pub struct WeatherQuery {
	pub city: String,
}

#[derive(sqlx::FromRow, Deserialize, Debug, Clone)]
pub struct LatLong {
	pub latitude: f64,
	pub longitude: f64,
}

pub async fn fetch_lat_long(city: &str) -> Result<LatLong, anyhow::Error> {
	let endpoint = format!(
		"https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
		city
	);
	let response = reqwest::get(&endpoint).await?.json::<GeoResponse>().await?;
	response.results.get(0).cloned().context("No results found")
}

pub async fn fetch_weather(lat_long: LatLong) -> Result<WeatherResponse, anyhow::Error> {
	let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m",
        lat_long.latitude, lat_long.longitude
	);
	let response = reqwest::get(&endpoint).await?.json::<WeatherResponse>().await?;
	
    Ok(response)
}

impl WeatherDisplay{
	pub fn new (city: String, response: WeatherResponse) -> Self {
		Self {
			city,
			forecasts: response
				.hourly
				.time
				.iter()
				.zip(response.hourly.temperature_2m.iter())
				.map(|(date, temperature)| Forecast {
					date: date.to_string(),
					temperature: temperature.to_string(),
				})
				.collect(),
		}
	}
}


#[derive(Deserialize, Debug)]
pub struct WeatherResponse {
	pub latitude: f64,
	pub longitude: f64,
	pub timezone: String,
	pub hourly: Hourly,
}

#[derive(Deserialize, Debug)]
pub struct Hourly {
	pub time: Vec<String>,
	pub temperature_2m: Vec<f64>,
}


#[derive(Template, Deserialize, Debug)]
#[template(path = "weather.html")]
pub struct WeatherDisplay {
	pub city: String,
	pub forecasts: Vec<Forecast>,
}

#[derive(Deserialize, Debug)]
pub struct Forecast {
	pub date: String,
	pub temperature: String,
}

pub struct AppError(anyhow::Error);

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


pub struct User;

#[async_trait]
impl<S> FromRequestParts<S> for User
where
	S: Send + Sync,
{
	type Rejection = axum::http::Response<axum::body::Body>;

	async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
		let auth_header = parts
			.headers
			.get("Authorization")
			.and_then(|header| header.to_str().ok());

		if let Some(auth_header) = auth_header {
			if auth_header.starts_with("Basic ") {
				let credentials = auth_header.trim_start_matches("Basic ");
				let decoded = base64::decode(credentials).unwrap_or_default();
				let credential_str = from_utf8(&decoded).unwrap_or("");

				// Our username and password are hardcoded here.
				// In a real app, you'd want to read them from the environment.
				if credential_str == "forecast:forecast" {
					return Ok(User);
				}
			}
		}

		let reject_response = axum::http::Response::builder()
			.status(StatusCode::UNAUTHORIZED)
			.header(
				"WWW-Authenticate",
				"Basic realm=\"Please enter your credentials\"",
			)
			.body(axum::body::Body::from("Unauthorized"))
			.unwrap();

		Err(reject_response)
	}
}