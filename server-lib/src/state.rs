use crate::config::Config;
use crate::jobs::JobId;
use girlboss::Girlboss;
use girlboss::Monitor;
use lapin::Connection;
use orgauth::data::UserId;
use std::sync::Arc;
use std::sync::RwLock;
use zkprotocol::content::Server;

pub struct State {
  pub config: Config,
  pub girlboss: Arc<RwLock<Girlboss<JobId, Monitor>>>,
  pub jobcounter: RwLock<i64>,
  pub server: Server,
  pub lapin_conn: RwLock<Option<Connection>>,
}

pub fn new_jobid(state: &State, uid: UserId) -> JobId {
  let mut j = state.jobcounter.write().unwrap();

  *j = *j + 1;

  JobId {
    uid: *uid.to_i64(),
    jobno: *j,
  }
}
