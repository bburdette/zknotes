use icontent::SystemVs;
use indradb::SledDatastore;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
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
}

pub struct State {
  pub config: Config,
  pub db: SledDatastore,
  pub svs: SystemVs,
}
