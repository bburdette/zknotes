pub use crate::constants::PrivateStreamingRequests;
use serde_json::Value;

#[derive(Deserialize, Serialize, Debug)]
pub struct PrivateStreamingMessage {
  pub what: PrivateStreamingRequests,
  pub data: Option<Value>,
}
