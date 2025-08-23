#![allow(non_snake_case)]

use crate::search::TagSearch;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum SpecialNote {
  SnSearch(Vec<TagSearch>),
  SnSync(CompletedSync),
  SnPlaylist(Notelist),
  // SnDateTime(DateTime),
  // SnAlert(Alert),
}

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct DateTime {
//   pub datetime: i64,
// }

// #[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
// pub struct Search {
//   pub search: Vec<TagSearch>,
// }

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct CompletedSync {
  pub after: Option<i64>,
  pub now: i64,
  pub remote: Option<Uuid>,
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
