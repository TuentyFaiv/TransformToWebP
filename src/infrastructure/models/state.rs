use reqwest::Client;
use tera::Tera;

use super::tiny::TinyApi;

pub struct AppState {
	pub html: Tera,
	pub curl: Client,
  pub tiny: TinyApi,
}

impl AppState {
  pub fn new(html: Tera, curl: Client) -> Self {
    let tiny = TinyApi::new(curl.clone());
    Self {
      html,
      curl,
      tiny,
    }
  }
}
