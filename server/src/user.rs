use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;
use zkprotocol::content::{ZkLink, ZkNote};

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
  pub zknote: i64,
  pub token: Option<Uuid>,
  pub tokendate: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginData {
  pub userid: i64,
  pub username: String,
  pub publicid: i64,
  pub shareid: i64,
  pub searchid: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZkDatabase {
  pub notes: Vec<ZkNote>,
  pub links: Vec<ZkLink>,
  pub users: Vec<User>,
}
