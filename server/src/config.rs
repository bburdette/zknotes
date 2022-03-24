use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub ip: String,
  pub port: u16,
  pub createdirs: bool,
  pub db: PathBuf,
  pub mainsite: String,
  pub altmainsite: Vec<String>,
  pub appname: String,
  pub domain: String,
  pub admin_email: String,
  pub error_index_note: Option<i64>,
  pub login_token_expiration_ms: i64,
  pub email_token_expiration_ms: i64,
  pub reset_token_expiration_ms: i64,
  pub static_path: Option<PathBuf>,
}
