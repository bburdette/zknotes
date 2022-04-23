use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub mainsite: String,
  pub appname: String,
  pub domain: String,
  pub db: PathBuf,
  pub admin_email: String,
  pub error_index_note: Option<i64>,
  pub login_token_expiration_ms: i64,
  pub email_token_expiration_ms: i64,
  pub reset_token_expiration_ms: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginData {
  pub userid: i64,
  pub name: String,
  pub data: Option<serde_json::Value>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RegistrationData {
  pub uid: String,
  pub pwd: String,
  pub email: String,
}

#[derive(Deserialize, Debug)]
pub struct Login {
  pub uid: String,
  pub pwd: String,
}

#[derive(Deserialize, Debug)]
pub struct ResetPassword {
  pub uid: String,
}

#[derive(Deserialize, Debug)]
pub struct SetPassword {
  pub uid: String,
  pub newpwd: String,
  pub reset_key: Uuid,
}

#[derive(Deserialize, Debug)]
pub struct ChangePassword {
  pub oldpwd: String,
  pub newpwd: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChangeEmail {
  pub pwd: String,
  pub email: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct WhatMessage {
  pub what: String,
  pub data: Option<serde_json::Value>,
}
