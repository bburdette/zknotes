#[macro_use]
extern crate serde_derive;
use serde_json::Value;

pub mod content;
pub mod search;

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

#[derive(Deserialize, Debug)]
pub struct UserMessage {
  pub uid: String,
  pub pwd: String,
  pub what: String,
  pub data: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct PublicMessage {
  pub what: String,
  pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
    assert_eq!(2 + 2, 4);
  }
}
