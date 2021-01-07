use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UserMessage {
  pub uid: String,
  pub pwd: String,
  pub what: String,
  pub data: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicMessage {
  pub what: String,
  pub data: Option<serde_json::Value>,
}
