#![allow(non_snake_case)]

use std::fmt::Display;

use crate::search::ZkNoteSearch;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use orgauth::data::UserId;
use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum SpecialNote {
  SnSearch(ZkNoteSearch),
  SnSync(CompletedSync),
  SnPlaylist(Notelist),
  SnDateTime(DateTime),
  // SnAlert(Alert),
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct DateTime {
  pub datetime: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Search {
  pub search: ZkNoteSearch,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct CompletedSync {
  pub after: Option<i64>,
  pub now: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Notelist {
  pub sequence: Vec<Uuid>,
  pub current: Option<i64>,
}

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct Alert {
//   pub event: Uuid,
//   pub offset: i64,
// }

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct PdfAnnotation {
//   pub event: Uuid,
//   pub offset: i64,  // text selection?
//   pub comment: String,
// }

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct VideoAnnotation {
//   pub event: Uuid,
//   pub offset: i64,
//   pub comment: String
// }

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct YeetLinkl{
//   pub url: String,
//   pub audioonly: bool,
// }
