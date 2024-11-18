use std::sync::RwLock;

use girlboss::Girlboss;

use crate::config::Config;
use crate::jobs::JobId;

pub struct State {
  pub config: Config,
  pub girlboss: Girlboss<JobId>,
  pub jobcounter: RwLock<i64>,
}

pub fn new_jobid(state: &State, uid: i64) -> JobId {
  let mut j = state.jobcounter.write().unwrap();

  *j = *j + 1;
  // state.jobcounter = state.jobcounter + 1;

  JobId {
    uid: uid,
    jobno: *j,
  }
}
