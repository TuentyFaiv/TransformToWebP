extern crate tera;
extern crate reqwest;

use actix_web::body::MessageBody;
use dotenvy::dotenv;
use reqwest::Client;
use serde_json::{Value};
use std::collections::HashMap;
use std::env;
// use reqwest::blocking;

use actix_web::{post, get, web, App, middleware};
use actix_web::{HttpServer, HttpResponse, Responder};
use tera::{Tera, Context};

struct AppState {
	tera: Tera,
	curl: Client
}

#[get("/")]
async fn home(state: web::Data<AppState>) -> impl Responder {
	let mut ctx = Context::new();

	let page = state.tera.render("index.html", &mut ctx).unwrap();

	HttpResponse::Ok()
		.content_type("text/html")
		.body(page)
}

#[get("/reqwest")]
async fn req(state: web::Data<AppState>) -> impl Responder {

	// let resp = state.curl
	// 	.get("https://rickandmortyapi.com/api")
	// 	.send()
	// 	.await
	// 	.json::<HashMap<String, String>>()
	// 	.await;

	let result = async {
		state.curl
			.get("https://rickandmortyapi.com/api")
			.send()
			.await?
			// .error_for_status()?
			.json::<Value>()
			.await
	}.await;

	println!("DATA: {:?}", result);

	HttpResponse::Ok().body(result)
	// match result {
	// 	Ok(data) => {
	// 		HttpResponse::Ok()
	// 			.content_type("application/json")
	// 			.body(data)
	// 	},
	// 	Err(err) => {
	// 		HttpResponse::InternalServerError()
	// 			.body(err.to_string())
	// 	}
	// }
}

// #[post("/transform")]
// async fn create_webp(mut payload: Multipart) -> Result<HttpResponse, Error> {
// 	// iterate over multipart stream
// 	while let Some(mut field) = payload.try_next().await? {
// 		// A multipart/form-data stream has to contain `content_disposition`
// 		let content_disposition = field.content_disposition();

// 		let filename = content_disposition
// 			.get_filename()
// 			.map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);
// 		let filepath = format!("./tmp/{filename}");

// 		// File::create is blocking operation, use threadpool
// 		let mut f = web::block(|| std::fs::File::create(filepath)).await??;

// 		// Field in turn is stream of *Bytes* object
// 		while let Some(chunk) = field.try_next().await? {
// 			// filesystem operations are blocking, we have to use threadpool
// 			f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
// 		}
// 	}

// 	Ok(HttpResponse::Ok().into())
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	dotenv().ok();
	env::set_var("RUST_LOG", "info");

	HttpServer::new(move || {
		let templates = concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*");
		let tera = Tera::new(templates).unwrap();
		let curl = Client::new();

		let state = AppState {
			tera,
			curl
		};

		App::new()
			.wrap(middleware::Logger::default())
			.service(home)
			.service(req)
			.app_data(web::Data::new(state))
	})
		.bind(("0.0.0.0", 5000))
		.unwrap()
		.run()
		.await
}