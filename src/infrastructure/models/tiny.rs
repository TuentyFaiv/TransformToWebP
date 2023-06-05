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
  pub fn set_key(&mut self, key: String) {
    let auth_key = STANDARD.encode(format!("api:{key}"));
    self.key = Some(auth_key);
  }
  pub async fn compress(&self, image: Vec<u8>) -> Result<TinyCompressionOutput, Error> {
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
  pub async fn convert(&self, url: String) -> Result<Response, Error>{
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, "*/*".parse().unwrap());
    headers.insert(AUTHORIZATION, format!("Basic {}", self.key.clone().unwrap()).parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let body_json: TinyConvertionInput = TinyConvertionInput {
      convert: json!({ "type": "image/webp" })
    };

    self.curl.get(url)
      .headers(headers)
      .json(&body_json)
      .send()
      .await?
      .error_for_status()
  }
}