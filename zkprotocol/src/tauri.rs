#![allow(non_snake_case)]

use elm_rs::{Elm, ElmDecode, ElmEncode};

use crate::content::ZkListNote;

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TauriRequest {
  TrqUploadFiles,
}

// #[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
// pub struct UploadFiles {
//   pub paths: Vec<String>,
// }

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TauriReply {
  TyUploadedFiles(UploadedFiles),
  TyServerError(String),
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct UploadedFiles {
  pub notes: Vec<ZkListNote>,
}
