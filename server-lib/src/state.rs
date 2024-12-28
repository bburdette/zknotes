use crate::config::Config;
use crate::jobs::JobId;
use girlboss::tokio::Girlboss;
use std::sync::{Arc, RwLock};

pub struct State {
  pub config: Config,
  pub girlboss: Arc<RwLock<Girlboss<JobId>>>,
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
