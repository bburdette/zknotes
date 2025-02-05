#![allow(non_snake_case)]

use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

use crate::content::ZkListNote;

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TauriRequest {
  TrqUploadFiles(UploadFiles),
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct UploadFiles {
  paths: Vec<String>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TauriReply {
  TyUploadedFiles(UploadedFiles),
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct UploadedFiles {
  paths: Vec<ZkListNote>,
}
