extern crate tera;
extern crate reqwest;
extern crate actix_web;
extern crate serde_json;

use std::io::Write;
use dotenvy::dotenv;
use futures::stream::TryStreamExt;
use tokio::io::AsyncWriteExt;
use std::{env, fs};

use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use reqwest::{Client, Body};
use reqwest::header::{HeaderMap, CONTENT_TYPE, AUTHORIZATION, ACCEPT};
use actix_web::{post, get, web, App, middleware};
use actix_web::{HttpServer, HttpResponse, Responder, Error};
use actix_multipart::Multipart;
use tera::{Tera, Context};

use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use uuid::Uuid;
use sanitize_filename::sanitize;
use base64::{Engine as _, engine::{general_purpose}};

struct EnvVars {
	tiny_key: String
}

struct AppState {
	html: Tera,
	curl: Client,
	envs: EnvVars
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

fn file_to_body(file: File) -> Body {
	let stream = FramedRead::new(file, BytesCodec::new());
	let body = Body::wrap_stream(stream);
	body
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

	let mut filename = String::new();

	while let Some(mut field) = payload.try_next().await? {
		// A multipart/form-data stream has to contain `content_disposition`
		let content_disposition = field.content_disposition();

		filename = content_disposition
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
	}
	
	let file = File::open(format!("./tmp/{filename}")).await?;

	let key = state.envs.tiny_key.clone();
	let auth_key = general_purpose::STANDARD
	.encode(format!("api:{key}"));

	let mut header_compress = HeaderMap::new();

	header_compress.insert(ACCEPT, "*/*".parse().unwrap());
	header_compress.insert(AUTHORIZATION, format!("Basic {auth_key}").parse().unwrap());
	header_compress.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

	let result = async {
		state.curl.post("https://api.tinify.com/shrink")
			.headers(header_compress)
			.body(file_to_body(file))
			.send()
			.await?
			.error_for_status()?
			.json::<Success>()
			.await
	}.await;

	println!("REQUEST: \n\n{:?}\n\n", result);

	fs::remove_file(format!("./tmp/{filename}"))
		.expect(format!("Error to delete file: ./tmp/{filename}").as_str());

	match result {
		Ok(data) => {
			let mut headers_convert = HeaderMap::new();

			headers_convert.insert(ACCEPT, "*/*".parse().unwrap());
			headers_convert.insert(AUTHORIZATION, format!("Basic {auth_key}").parse().unwrap());
			headers_convert.insert(CONTENT_TYPE, "application/json".parse().unwrap());

			let body_json = json!({
				"convert": { "type": "image/webp" }
			});

			let convert = async {
				state.curl.get(data.output.url)
					.headers(headers_convert)
					.json(&body_json)
					.send()
					.await?
					.error_for_status()
			}.await;

			let convertion = match convert {
				Ok(res_convert) => {
					
					let headers_res = res_convert.headers().clone();

					let length = res_convert.content_length().unwrap();
					let content = headers_res.get("content-type").unwrap().to_str().unwrap();
					let date = headers_res.get("date").unwrap().to_str().unwrap();
					let connection = headers_res.get("connection").unwrap().to_str().unwrap();
					let width = headers_res.get("image-width").unwrap().to_str().unwrap();
					let height = headers_res.get("image-height").unwrap().to_str().unwrap();
					let count = headers_res.get("compression-count").unwrap().to_str().unwrap();

					let bytes = res_convert.bytes().await.unwrap();

					let path_saved = "./tmp/compressed_image.webp";

					let mut file_res = File::create(path_saved).await?;

					file_res.write(&bytes).await?;

					let image_content = web::block(move || {
						fs::read(path_saved).unwrap()
					}).await?;

					let log_image = json!({
						"length": length,
						"type": content,
						"width": width,
						"height": height
					});

					println!("IMAGE_DATA: \n\n{:?}\n\n", log_image);

					Ok(
						HttpResponse::Ok()
							.content_type(content)
							.insert_header(("date", date))
							.insert_header(("content-length", length))
							.insert_header(("connection", connection))
							.insert_header(("image-width", width))
							.insert_header(("image-height", height))
							.insert_header(("compression-count", count))
							.body(image_content)
					)
				},
				Err(err_convert) => {
					Ok(HttpResponse::InternalServerError()
						.body(err_convert.to_string()))
				}
			};
			
			return convertion;
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

		let tiny_key = env::var("TINY_KEY").expect("TINY Key variable not found");

		let state = AppState {
			html,
			curl,
			envs: EnvVars { tiny_key }
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