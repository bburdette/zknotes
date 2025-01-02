use crate::config::Config;
use crate::jobs::JobId;
use girlboss::Girlboss;
use girlboss::Monitor;
use std::sync::Arc;
use std::sync::RwLock;

pub struct State {
  pub config: Config,
  pub girlboss: Arc<RwLock<Girlboss<JobId, Monitor>>>,
  pub jobcounter: RwLock<i64>,
}

pub fn new_jobid(state: &State, uid: i64) -> JobId {
  let mut j = state.jobcounter.write().unwrap();
  // let mut j = state.jobcounter.lock().unwrap();

  *j = *j + 1;

  JobId {
    uid: uid,
    jobno: *j,
  }
}
