#![allow(non_snake_case)]

use crate::search::TagSearch;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum SpecialNote {
  SnSearch(Vec<TagSearch>),
  SnSync(CompletedSync),
  SnList,
  // SnDateTime(DateTime),
  // SnAlert(Alert),
}

// to decode old lists.
pub enum SpecialNoteObsolete1 {
  SnSearch(Vec<TagSearch>),
  SnSync(CompletedSync),
  SnList(NotegraphObsolete1),
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct NotegraphObsolete1 {
  pub currentUuid: Option<Uuid>,
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
  pub local: Option<Uuid>,  // optional for backward compatibility
  pub remote: Option<Uuid>, // optional for backward compatibility
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
