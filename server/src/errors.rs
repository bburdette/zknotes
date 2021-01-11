use std::io;

// #[derive(Debug, Fail)]
#[derive(Debug)]
pub enum Error {
  // #[fail(display = "i/o error: {}", inner)]
  Io { inner: io::Error },
  CouldNotParseBinding,
  IndraV { inner: indradb::ValidationError },
  Indra { inner: indradb::Error },
  SerdeJs { inner: serde_json::error::Error },
  Simple { inner: simple_error::SimpleError },
  Std { inner: Box<dyn std::error::Error> },
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Error::Io { inner: err }
  }
}

impl From<indradb::ValidationError> for Error {
  fn from(err: indradb::ValidationError) -> Self {
    Error::IndraV { inner: err }
  }
}
impl From<indradb::Error> for Error {
  fn from(err: indradb::Error) -> Self {
    Error::Indra { inner: err }
  }
}
impl From<serde_json::error::Error> for Error {
  fn from(err: serde_json::error::Error) -> Self {
    Error::SerdeJs { inner: err }
  }
}
impl From<simple_error::SimpleError> for Error {
  fn from(err: simple_error::SimpleError) -> Self {
    Error::Simple { inner: err }
  }
}
impl From<Box<dyn std::error::Error>> for Error {
  fn from(err: Box<dyn std::error::Error>) -> Self {
    Error::Std { inner: err }
  }
}
