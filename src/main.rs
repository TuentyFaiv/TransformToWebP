extern crate tera;
extern crate reqwest;
extern crate actix_web;
extern crate serde_json;

use std::io::Write;
use dotenvy::dotenv;
use futures::TryStreamExt;
use std::{env, fs};

use reqwest::{Client};
use reqwest::header::{HeaderMap, CONTENT_TYPE, AUTHORIZATION};
use actix_web::{post, get, web, App, middleware};
use actix_web::{HttpServer, HttpResponse, Responder, Error};
use actix_multipart::Multipart;
use tera::{Tera, Context};
use serde_json::{Value};

use uuid::Uuid;
use sanitize_filename::sanitize;
use base64::{Engine as _, engine::{general_purpose}};

struct AppState {
	html: Tera,
	curl: Client
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
		Ok(data) => {
			HttpResponse::Ok()
				.json(data)
		},
		Err(err) => {
			HttpResponse::InternalServerError()
				.body(err.to_string())
		}
	}
}

#[post("/transform")]
async fn create_webp(
	state: web::Data<AppState>,
	mut payload: Multipart,
) -> Result<HttpResponse, Error> {
	// iterate over multipart stream

	let mut file_base64 = String::new();

	while let Some(mut field) = payload.try_next().await? {
		// A multipart/form-data stream has to contain `content_disposition`
		let content_disposition = field.content_disposition();

		let filename = content_disposition
			.get_filename()
			.map_or_else(|| Uuid::new_v4().to_string(), sanitize);

		let filepath = format!("./tmp/{filename}");


		// File::create is blocking operation, use threadpool
		let mut f = web::block(|| std::fs::File::create(filepath)).await??;

		// Field in turn is stream of *Bytes* object
		while let Some(chunk) = field.try_next().await? {
			// filesystem operations are blocking, we have to use threadpool
			f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
		}

		let file_vec = fs::read(format!("./tmp/{filename}"))
			.expect("Error to read file");

		file_base64 = general_purpose::STANDARD.encode(file_vec);
	}

	// println!("File base 64: {:?}", file_base64);

	let auth_key = general_purpose::STANDARD
		.encode(format!("api:8FJ7Jjrm06d7gSYdJGp0FbnkGhQfgPLY"));

	let mut heads = HeaderMap::new();

	heads.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());
	heads.insert(AUTHORIZATION, format!("Basic {auth_key}").parse().unwrap());

	let result = async {
		state.curl.post("https://api.tinify.com/shrink")
			.headers(heads)
			.body(file_base64)
			.send()
			.await?
			.error_for_status()?
			.json::<Value>()
			.await
	}.await;

	println!("Request: {:?}", result);

	match result {
		Ok(data) => {
			Ok(HttpResponse::Ok().json(data))
		},
		Err(err) => {
			Ok(HttpResponse::InternalServerError()
				.body(err.to_string()))
		}
	}
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv().ok();
	env::set_var("RUST_LOG", "info");
	fs::create_dir_all("./tmp")?;

	HttpServer::new(move || {
		let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
		let html = Tera::new(templates).unwrap();
		let curl = Client::new();

		let state = AppState {
			html,
			curl
		};

		App::new()
			.wrap(middleware::Logger::default())
			.service(home)
			.service(create_webp)
			.service(req)
			.app_data(web::Data::new(state))
	})
		.bind(("0.0.0.0", 5000))
		.unwrap()
		.run()
		.await
}