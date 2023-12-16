use crate::constants::{PrivateRequests, PrivateStreamingRequests, PublicRequests};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

// TODO rename to WhatMessage?
#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateMessage {
  pub what: PrivateRequests,
  pub data: Option<Value>,
}

// TODO rename to WhatMessage?
#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateStreamingMessage {
  pub what: PrivateStreamingRequests,
  pub data: Option<Value>,
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
  pub what: PublicRequests,
  pub data: Option<Value>,
}
