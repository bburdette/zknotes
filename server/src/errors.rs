use std::io;

// #[derive(Debug, Fail)]
#[derive(Debug)]
pub enum Error {
  // #[fail(display = "i/o error: {}", inner)]
  Io { inner: io::Error },
  // #[fail(display = "could not parse address binding")]
  CouldNotParseBinding,
  // #[fail(display = "validation error: {}", inner)]
  IndraV { inner: indradb::ValidationError },
  // #[fail(display = "validation error: {}", inner)]
  Indra { inner: indradb::Error },
  // #[fail(display = "validation error: {}", inner)]
  SerdeJs { inner: serde_json::error::Error },
  // #[fail(display = "validation error: {}", inner)]
  Simple { inner: simple_error::SimpleError },
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
