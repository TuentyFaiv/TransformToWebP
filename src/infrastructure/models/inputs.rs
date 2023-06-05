use serde::{Serialize, Deserialize};
use serde_json::Value;


#[derive(Serialize, Deserialize, Debug)]
pub struct TinyImage {
	pub size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TinyConvertionInput {
  pub convert: Value,
}
