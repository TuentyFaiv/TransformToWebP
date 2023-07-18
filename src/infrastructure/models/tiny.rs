use std::collections::HashMap;
use base64::{Engine as _};
use base64::engine::general_purpose::STANDARD;
use actix_web::Result;
use reqwest::{Client, Response};
use reqwest::Error;
use reqwest::header::{HeaderMap, CONTENT_TYPE, AUTHORIZATION, ACCEPT};
use serde_json::json;

use super::inputs::TinyConvertionInput;
use super::outputs::TinyCompressionOutput;

#[derive(Clone)]
pub struct TinyApi {
  key: Option<String>,
  curl: Client,
  api: String,
}

impl TinyApi {
  pub fn new(curl: Client) -> Self {
    Self {
      curl,
      key: None,
      api: "https://api.tinify.com".to_string(),
    }
  }
  fn set_key(&mut self, key: String) {
    let auth_key = STANDARD.encode(format!("api:{key}"));
    self.key = Some(auth_key);
  }
  async fn compress(&self, image: Vec<u8>) -> Result<TinyCompressionOutput, Error> {
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, "*/*".parse().unwrap());
    headers.insert(AUTHORIZATION, format!("Basic {}", self.key.clone().unwrap()).parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());

    self.curl.post(format!("{}{}", self.api, "/shrink"))
      .headers(headers)
      .body(image)
      .send()
      .await?
      .error_for_status()?
      .json::<TinyCompressionOutput>()
      .await
  }
  pub async fn convert(&mut self, data: HashMap<String, Vec<u8>>) -> Result<Response, Error>{
    let file_bytes = data.get("image").unwrap().clone();
    let key = String::from_utf8(data.get("tiny_key").unwrap().clone()).unwrap();
    self.set_key(key);

    let default_format = "image/webp".as_bytes().to_vec();
    let format: String = String::from_utf8(data.get("type").unwrap_or_else(|| &default_format).clone()).unwrap();

    let compressed = self.compress(file_bytes).await;

    match compressed {
      Ok(data) => {
        let mut headers = HeaderMap::new();

        headers.insert(ACCEPT, "*/*".parse().unwrap());
        headers.insert(AUTHORIZATION, format!("Basic {}", self.key.clone().unwrap()).parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        println!("Default type image: {:?}", default_format);
        println!("User type image: {:?}", format);

        let body_json: TinyConvertionInput = TinyConvertionInput {
          convert: json!({ "type": format.as_str() })
        };

        self.curl.get(data.output.url)
          .headers(headers)
          .json(&body_json)
          .send()
          .await?
          .error_for_status()

      },
      Err(error) => Err(error)
    }
  }
}