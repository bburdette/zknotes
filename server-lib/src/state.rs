use girlboss::Girlboss;

use crate::config::Config;

pub struct State {
  pub config: Config,
  pub girlboss: Girlboss<String>,
}
