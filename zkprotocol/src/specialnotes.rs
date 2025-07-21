#![allow(non_snake_case)]

use std::fmt::Display;

use crate::search::ZkNoteSearch;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use orgauth::data::UserId;
use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum SpecialNote {
  SnDateTime(DateTime),
  SnSearch(ZkNoteSearch),
  SnSync(CompletedSync),
  SnPlaylist(,
  SnAlert
}


#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct DateTime {
    pub datetime i64;
  }
#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Search {
}
#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct CompletedSync {
  after: Option<i64>,
  now: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Playlist {
  sequence: Vec[Uuid],
  current: Option<i64>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Alert {
    pub event Uuid;
    
    
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct JobStatus {
  pub jobno: i64,
  pub state: JobState,
  pub message: String,
}


