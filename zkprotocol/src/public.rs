use crate::content::{GetZkNoteAndLinks, GetZnlIfChanged, ZkNoteAndLinks, ZkNoteAndLinksWhat};
use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde_derive::{Deserialize, Serialize};

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicRequest {
  PbrGetZkNoteAndLinks(GetZkNoteAndLinks),
  PbrGetZnlIfChanged(GetZnlIfChanged),
  PbrGetZkNotePubId(String),
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicReply {
  PbyServerError(PublicError),
  PbyZkNoteAndLinks(ZkNoteAndLinks),
  PbyZkNoteAndLinksWhat(ZkNoteAndLinksWhat),
  PbyNoop,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicError {
  PbeString(String),
  PbeNoteNotFound(PublicRequest),
  PbeNoteIsPrivate(PublicRequest),
}
