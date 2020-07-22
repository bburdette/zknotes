use std::path::Path;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
  pub ip: String,
  pub port: u16,
  pub createdirs: bool,
  pub db: String,
  pub mainsite: String,
  pub appname: String,
  pub domain: String,
}