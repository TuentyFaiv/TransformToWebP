mod infrastructure;

use std::collections::HashMap;
use std::env;
use dotenvy::dotenv;
use env_logger::Env;
use futures::TryStreamExt;
use futures::stream::StreamExt;
use reqwest::Client;
use actix_cors::Cors;
use actix_web::{post, get, web, middleware};
use actix_web::{HttpResponse, Responder};
use actix_multipart::Multipart;
use tera::{Tera, Context};
use serde_json::{Value, json};

use crate::infrastructure::models::state::AppState;

#[get("/")]
async fn home(state: web::Data<AppState>) -> impl Responder {
	let mut ctx = Context::new();

	let page = state.html.render("index.html", &mut ctx).unwrap();

	HttpResponse::Ok()
		.content_type("text/html")
		.body(page)
}

#[get("/reqwest")]
async fn req(state: web::Data<AppState>) -> impl Responder {

	let result = async {
		state.curl.get("https://rickandmortyapi.com/api")
			.send()
			.await?
			.error_for_status()?
			.json::<Value>()
			.await
	}.await;

	match result {
		Ok(data) => HttpResponse::Ok().json(data),
		Err(err) => HttpResponse::InternalServerError().body(err.to_string())
	}
}

#[post("/transform")]
async fn create_webp(state: web::Data<AppState>, mut payload: Multipart) -> impl Responder {
	let mut data: HashMap<String, Vec<u8>> = HashMap::new();

	while let Ok(Some(mut field)) = payload.try_next().await {
		let content_type = field.content_disposition();
		let name = content_type.get_name().unwrap().to_string();

		let mut data_bytes: Vec<u8> = Vec::new();
		while let Some(chunk) = field.next().await {
			data_bytes.extend_from_slice(&chunk.unwrap());
		}

		data.insert(name, data_bytes);
	}
	let mut tiny = state.tiny.clone();

	// let file_bytes = data.get("image").unwrap().clone();
	// let key = String::from_utf8(data.get("tiny_key").unwrap().clone()).unwrap();

	// tiny.set_key(key);

	// let result = tiny.compress(file_bytes).await;

	// println!("TO_COMPRESS: \n\n{:?}\n\n", result);

	match tiny.convert(data).await {
		Ok(response) => {
			let headers = response.headers().clone();

			let length = response.content_length().unwrap();
			let content = headers.get("content-type").unwrap();
			let date = headers.get("date").unwrap();
			let connection = headers.get("connection").unwrap();
			let width = headers.get("image-width").unwrap();
			let height = headers.get("image-height").unwrap();
			let count = headers.get("compression-count").unwrap();

			let image = response.bytes().await.unwrap().to_vec();

			let log_image = json!({
				"length": length,
				"type": content.to_str().unwrap(),
				"width": width.to_str().unwrap(),
				"height": height.to_str().unwrap(),
			});

			println!("COMPRESSED: \n\n{:?}\n\n", log_image);

			HttpResponse::Ok()
				.content_type(content)
				.insert_header(("date", date))
				.insert_header(("content-length", length))
				.insert_header(("connection", connection))
				.insert_header(("image-width", width))
				.insert_header(("image-height", height))
				.insert_header(("compression-count", count))
				.body(image)
		},
		Err(err) => HttpResponse::InternalServerError().body(err.to_string())
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	use actix_web::{App, HttpServer};
	dotenv().ok();
	env::set_var("RUST_LOG", "info");

	env_logger::init_from_env(Env::default().default_filter_or("info"));

	let port = env::var("PORT")
		.unwrap_or_else(|_| "5000".to_string())
		.parse::<u16>()
		.expect("PORT must be a number");

	HttpServer::new(move || {
		let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
		let html = Tera::new(templates).unwrap();
		let curl = Client::new();

		let state = AppState::new(html, curl);

		let cors = Cors::permissive();

		App::new()
			.wrap(cors)
			.wrap(middleware::Logger::default())
			.wrap(middleware::Logger::new("%a %{User-Agent}i"))
			.service(home)
			.service(create_webp)
			.service(req)
			.app_data(web::Data::new(state))
	})
		.bind(("0.0.0.0", port))?
		.run()
		.await
}