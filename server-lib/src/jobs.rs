use std::fmt;

use girlboss::Monitor;
use log::{logger, Record};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct JobId {
  pub uid: i64,
  pub jobno: i64,
}

pub trait JobMonitor {
  // pub fn report(&self, status: impl Into<JobStatus>) {
  //     self.0.set_status(status.into());
  // }

  /// Implementation to allow use with [`write!`].
  fn write_fmt(&self, args: fmt::Arguments<'_>);
}

pub struct GirlbossMonitor {
  pub monitor: Monitor,
}

impl JobMonitor for GirlbossMonitor {
  fn write_fmt(&self, args: fmt::Arguments<'_>) {
    self.monitor.write_fmt(args);
  }
}

pub struct LogMonitor {}

impl JobMonitor for LogMonitor {
  fn write_fmt(&self, args: fmt::Arguments<'_>) {
    let r = Record::builder().args(args).level(log::Level::Info).build();
    logger().log(&r);
  }
}
