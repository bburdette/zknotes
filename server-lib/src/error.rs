use actix_session;
use actix_web::error as awe;
use regex;
use reqwest;
use rusqlite;
use serde_json;
use std::fmt;
use url;

pub enum Error {
  Rusqlite(rusqlite::Error),
  SystemTimeError(std::time::SystemTimeError),
  String(String),
  ActixError(awe::Error),
  SerdeJson(serde_json::Error),
  IoError(std::io::Error),
  Reqwest(reqwest::Error),
  Uuid(uuid::Error),
  Orgauth(orgauth::error::Error),
  Regex(regex::Error),
  UrlParser(url::ParseError),
}

pub fn to_orgauth_error(e: Error) -> orgauth::error::Error {
  match e {
    Error::Rusqlite(ze) => orgauth::error::Error::Rusqlite(ze),
    Error::SystemTimeError(ze) => orgauth::error::Error::SystemTimeError(ze),
    Error::String(ze) => orgauth::error::Error::String(ze),
    Error::ActixError(ze) => orgauth::error::Error::ActixError(ze),
    Error::SerdeJson(ze) => orgauth::error::Error::SerdeJson(ze),
    Error::IoError(ze) => orgauth::error::Error::IoError(ze),
    Error::Reqwest(ze) => orgauth::error::Error::Reqwest(ze),
    Error::Uuid(ze) => orgauth::error::Error::Uuid(ze),
    Error::Orgauth(ze) => ze,
    Error::Regex(ze) => orgauth::error::Error::String(ze.to_string()),
    Error::UrlParser(ze) => orgauth::error::Error::String(ze.to_string()),
  }
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
      Error::IoError(e) => write!(f, "{}", e),
      Error::Reqwest(e) => write!(f, "{}", e),
      Error::Uuid(e) => write!(f, "{}", e),
      Error::Orgauth(e) => write!(f, "{}", e),
      Error::Regex(e) => write!(f, "{}", e),
      Error::UrlParser(e) => write!(f, "{}", e),
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
      Error::IoError(e) => write!(f, "{}", e),
      Error::Reqwest(e) => write!(f, "{}", e),
      Error::Uuid(e) => write!(f, "{}", e),
      Error::Orgauth(e) => write!(f, "{}", e),
      Error::Regex(e) => write!(f, "{}", e),
      Error::UrlParser(e) => write!(f, "{}", e),
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

impl From<std::io::Error> for Error {
  fn from(e: std::io::Error) -> Self {
    Error::IoError(e)
  }
}

impl From<reqwest::Error> for Error {
  fn from(e: reqwest::Error) -> Self {
    Error::Reqwest(e)
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
impl From<uuid::Error> for Error {
  fn from(e: uuid::Error) -> Self {
    Error::String(e.to_string())
  }
}
impl From<orgauth::error::Error> for Error {
  fn from(e: orgauth::error::Error) -> Self {
    Error::String(e.to_string())
  }
}
impl From<regex::Error> for Error {
  fn from(e: regex::Error) -> Self {
    Error::String(e.to_string())
  }
}
impl From<url::ParseError> for Error {
  fn from(e: url::ParseError) -> Self {
    Error::String(e.to_string())
  }
}
