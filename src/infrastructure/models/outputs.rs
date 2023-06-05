use serde::{Serialize, Deserialize};

use super::inputs::TinyImage;

#[derive(Serialize, Deserialize, Debug)]
pub struct TinyCompression {
	pub url: String,
	pub size: usize,
	pub width: usize,
	pub height: usize,
	pub ratio: f64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TinyCompressionOutput {
  input: TinyImage,
  pub output: TinyCompression,
}