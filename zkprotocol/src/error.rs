use actix_session;
use actix_web::error as awe;
use lettre;
use lettre::transport::smtp as lts;
use rusqlite;
use serde_json;
use std::fmt;

pub enum Error {
  Rusqlite(rusqlite::Error),
  SystemTimeError(std::time::SystemTimeError),
  String(String),
  ActixError(awe::Error),
  SerdeJson(serde_json::Error),
  LettreError(lettre::error::Error),
  LettreSmtpError(lts::Error),
  AddressError(lettre::address::AddressError),
  IoError(std::io::Error),
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self {
      Error::Rusqlite(rue) => write!(f, "{}", rue),
      Error::SystemTimeError(e) => write!(f, "{}", e),
      Error::String(e) => write!(f, "{}", e),
      Error::ActixError(e) => write!(f, "{}", e),
      Error::SerdeJson(e) => write!(f, "{}", e),
      Error::LettreError(e) => write!(f, "{}", e),
      Error::LettreSmtpError(e) => write!(f, "{}", e),
      Error::AddressError(e) => write!(f, "{}", e),
      Error::IoError(e) => write!(f, "{}", e),
    }
  }
}

impl fmt::Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self {
      Error::Rusqlite(rue) => write!(f, "{}", rue),
      Error::SystemTimeError(e) => write!(f, "{}", e),
      Error::String(e) => write!(f, "{}", e),
      Error::ActixError(e) => write!(f, "{}", e),
      Error::SerdeJson(e) => write!(f, "{}", e),
      Error::LettreError(e) => write!(f, "{}", e),
      Error::LettreSmtpError(e) => write!(f, "{}", e),
      Error::AddressError(e) => write!(f, "{}", e),
      Error::IoError(e) => write!(f, "{}", e),
    }
  }
}

impl From<rusqlite::Error> for Error {
  fn from(error: rusqlite::Error) -> Self {
    Error::Rusqlite(error)
  }
}

impl From<std::time::SystemTimeError> for Error {
  fn from(error: std::time::SystemTimeError) -> Self {
    Error::SystemTimeError(error)
  }
}

impl From<String> for Error {
  fn from(s: String) -> Self {
    Error::String(s)
  }
}

impl From<&str> for Error {
  fn from(s: &str) -> Self {
    Error::String(s.to_string())
  }
}

impl From<awe::Error> for Error {
  fn from(e: awe::Error) -> Self {
    Error::ActixError(e)
  }
}

impl From<serde_json::Error> for Error {
  fn from(e: serde_json::Error) -> Self {
    Error::SerdeJson(e)
  }
}

impl From<lettre::error::Error> for Error {
  fn from(e: lettre::error::Error) -> Self {
    Error::LettreError(e)
  }
}

impl From<lettre::address::AddressError> for Error {
  fn from(e: lettre::address::AddressError) -> Self {
    Error::AddressError(e)
  }
}

impl From<lts::Error> for Error {
  fn from(e: lts::Error) -> Self {
    Error::LettreSmtpError(e)
  }
}

impl From<std::io::Error> for Error {
  fn from(e: std::io::Error) -> Self {
    Error::IoError(e)
  }
}

impl From<actix_session::SessionGetError> for Error {
  fn from(e: actix_session::SessionGetError) -> Self {
    Error::String(e.to_string())
  }
}

impl From<actix_session::SessionInsertError> for Error {
  fn from(e: actix_session::SessionInsertError) -> Self {
    Error::String(e.to_string())
  }
}
