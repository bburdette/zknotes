use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde_derive::{Deserialize, Serialize};

use crate::content::ZkListNote;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum UploadReply {
  UrFilesUploaded(Vec<ZkListNote>),
}
