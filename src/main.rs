extern crate tera;

use dotenvy::dotenv;
use std::env;

use actix_web::{post, get, web, App, middleware};
use actix_web::{HttpServer, HttpResponse, Responder};
use tera::{Tera, Context};

#[get("/")]
async fn home(html: web::Data<Tera>) -> impl Responder {
	let mut ctx = Context::new();

	let page = html.render("index.html", &mut ctx).unwrap();

	HttpResponse::Ok()
		.content_type("text/html")
		.body(page)
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

		App::new()
			.wrap(middleware::Logger::default())
			.service(home)
			.app_data(web::Data::new(tera))
	})
		.bind(("0.0.0.0", 5000))
		.unwrap()
		.run()
		.await
}