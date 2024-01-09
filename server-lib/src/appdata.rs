use orgauth::data as orgauth_data;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppData {
  pub config: Config,
  pub wstokens: Arc<Mutex<HashMap<Uuid, TokenInfo>>>,
}

#[derive(Clone, Debug)]
pub struct TokenInfo {
  pub create_time: i64,
  pub uid: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub ip: String,
  pub port: u16,
  pub createdirs: bool,
  pub altmainsite: Vec<String>,
  pub static_path: Option<PathBuf>,
  pub file_tmp_path: PathBuf,
  pub file_path: PathBuf,
  pub error_index_note: Option<i64>,
  pub orgauth_config: orgauth_data::Config,
}
