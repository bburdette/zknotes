use crate::icontent::SystemVs;
use indradb::SledDatastore;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
  pub ip: String,
  pub port: u16,
  pub createdirs: bool,
  pub db: PathBuf,
  pub indradb: PathBuf,
  pub mainsite: String,
  pub appname: String,
  pub domain: String,
  pub admin_email: String,
  pub token_expiration_ms: i64,
}

pub struct State {
  pub config: Config,
  pub db: SledDatastore,
  pub svs: SystemVs,
}
