#![allow(non_snake_case)]

use crate::search::TagSearch;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum SpecialNote {
  SnSearch(Vec<TagSearch>),
  SnSync(CompletedSync),
  SnList(Notegraph),
  SnStylePalette(StylePalette),
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
  pub local: Option<Uuid>,  // optional for backward compatibility
  pub remote: Option<Uuid>, // optional for backward compatibility
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct StyleColor {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct StylePalette {
  pub buttons: StyleColor,
  pub buttonFontColor: StyleColor,
  pub tabs: StyleColor,
  pub Background: StyleColor,
  pub tabBackground: StyleColor,
  pub fontColor: StyleColor,
  pub savecolor: StyleColor,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct Notegraph {
  pub currentUuid: Option<Uuid>,
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
