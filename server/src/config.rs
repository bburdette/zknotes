use orgauth::data as orgauth_data;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub ip: String,
  pub port: u16,
  pub createdirs: bool,
  pub altmainsite: Vec<String>,
  pub static_path: Option<PathBuf>,
  pub error_index_note: Option<i64>,
  pub orgauth_config: orgauth_data::Config,
}
