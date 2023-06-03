extern crate tera;
extern crate reqwest;
extern crate actix_web;
extern crate actix_cors;
extern crate serde_json;

use std::collections::HashMap;
use std::env;
use dotenvy::dotenv;
use futures::TryStreamExt;
use futures::stream::StreamExt;
use reqwest::Client;
use reqwest::header::{HeaderMap, CONTENT_TYPE, AUTHORIZATION, ACCEPT};
use actix_cors::Cors;
use actix_web::{post, get, web, App, middleware};
use actix_web::{HttpServer, HttpResponse, Responder};
use actix_multipart::Multipart;
use tera::{Tera, Context};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use base64::{Engine as _};
use base64::engine::general_purpose::STANDARD;

// struct EnvVars {
// }

struct AppState {
	html: Tera,
	curl: Client,
}

#[derive(Serialize, Deserialize, Debug)]
struct SuccessInput {
	size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct SuccessOutput {
	url: String,
	size: usize,
	width: usize,
	height: usize,
	ratio: f64
}

#[derive(Serialize, Deserialize, Debug)]
struct Success {
	input: SuccessInput,
	output: SuccessOutput
}

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
async fn create_webp(
	state: web::Data<AppState>,
	mut payload: Multipart,
) -> impl Responder {
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

	let file_bytes = data.get("image").unwrap().clone();
	let key = String::from_utf8(data.get("tiny_key").unwrap().clone()).unwrap();
	let auth_key = STANDARD.encode(format!("api:{key}"));

	let mut header_compress = HeaderMap::new();

	header_compress.insert(ACCEPT, "*/*".parse().unwrap());
	header_compress.insert(AUTHORIZATION, format!("Basic {auth_key}").parse().unwrap());
	header_compress.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

	let result = async {
		state.curl.post("https://api.tinify.com/shrink")
			.headers(header_compress)
			.body(file_bytes)
			.send()
			.await?
			.error_for_status()?
			.json::<Success>()
			.await
	}.await;

	println!("TO_COMPRESS: \n\n{:?}\n\n", result);

	match result {
		Ok(data) => {
			let mut headers_convert = HeaderMap::new();

			headers_convert.insert(ACCEPT, "*/*".parse().unwrap());
			headers_convert.insert(AUTHORIZATION, format!("Basic {auth_key}").parse().unwrap());
			headers_convert.insert(CONTENT_TYPE, "application/json".parse().unwrap());

			let body_json = json!({ "convert": { "type": "image/webp" } });

			let convert = async {
				state.curl.get(data.output.url)
					.headers(headers_convert)
					.json(&body_json)
					.send()
					.await?
					.error_for_status()
			}.await;

			match convert {
				Ok(response) => {
					
					let headers_res = response.headers().clone();

					let length = response.content_length().unwrap();
					let content = headers_res.get("content-type").unwrap().to_str().unwrap();
					let date = headers_res.get("date").unwrap().to_str().unwrap();
					let connection = headers_res.get("connection").unwrap().to_str().unwrap();
					let width = headers_res.get("image-width").unwrap().to_str().unwrap();
					let height = headers_res.get("image-height").unwrap().to_str().unwrap();
					let count = headers_res.get("compression-count").unwrap().to_str().unwrap();

					let image = response.bytes().await.unwrap().to_vec();

					let log_image = json!({
						"length": length,
						"type": content,
						"width": width,
						"height": height
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
				Err(err) => {
					HttpResponse::InternalServerError().body(err.to_string())
				}
			}
		},
		Err(err) => {
			HttpResponse::InternalServerError().body(err.to_string())
		}
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv().ok();
	env::set_var("RUST_LOG", "info");

	HttpServer::new(move || {
		let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
		let html = Tera::new(templates).unwrap();
		let curl = Client::new();

		let state = AppState {
			html,
			curl,
		};

		let cors = Cors::permissive();

		App::new()
			.wrap(cors)
			.wrap(middleware::Logger::default())
			.service(home)
			.service(create_webp)
			.service(req)
			.app_data(web::Data::new(state))
	})
		.bind(("0.0.0.0", 5000))?
		.run()
		.await
}