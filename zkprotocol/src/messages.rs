pub use crate::constants::{
  PrivateReplies, PrivateRequests, PrivateStreamingRequests, PublicReplies, PublicRequests,
};
use serde_json::Value;

#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateMessage {
  pub what: PrivateRequests,
  pub data: Option<Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateReplyMessage {
  pub what: PrivateReplies,
  pub content: Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateStreamingMessage {
  pub what: PrivateStreamingRequests,
  pub data: Option<Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicMessage {
  pub what: PublicRequests,
  pub data: Option<Value>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicReplyMessage {
  pub what: PublicReplies,
  pub content: Value,
}
