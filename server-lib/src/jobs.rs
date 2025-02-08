use std::{fmt, fs::File, io::Write, sync::Mutex};

use girlboss::Monitor;
use log::{error, logger, Record};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct JobId {
  pub uid: i64,
  pub jobno: i64,
}

pub trait JobMonitor {
  // Implementation to allow use with [`write!`].
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
    let r = Record::builder()
      .args(args)
      .level(log::Level::Debug)
      .build();
    logger().log(&r);
  }
}

pub struct ReportFileMonitor {
  pub monitor: Monitor,
  pub outf: Mutex<File>,
}

impl JobMonitor for ReportFileMonitor {
  fn write_fmt(&self, args: fmt::Arguments<'_>) {
    if let Ok(mut f) = self.outf.lock() {
      // block returning Result.  Works better in async!
      let mut r = || {
        f.write_fmt(args)?;
        write!(f, "\n")?;
        Ok::<(), Box<dyn std::error::Error>>(())
      };
      match r() {
        Ok(_) => (),
        Err(e) => error!("{}", e.to_string()),
      }
    }
    self.monitor.write_fmt(args);
  }
}
