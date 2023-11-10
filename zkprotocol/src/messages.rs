use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

// TODO rename to WhatMessage?
#[derive(Deserialize, Serialize, Debug)]
pub struct UserMessage {
  pub what: String,
  pub data: Option<Value>,
}

// TODO get rid of in favor of WhatMessage?
#[derive(Deserialize, Serialize, Debug)]
pub struct PublicMessage {
  pub what: String,
  pub data: Option<Value>,
}
