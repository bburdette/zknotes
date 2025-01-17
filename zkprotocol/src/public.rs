use crate::content::{GetZkNoteAndLinks, GetZnlIfChanged, ZkNoteAndLinks, ZkNoteAndLinksWhat};
use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde_derive::{Deserialize, Serialize};

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicRequest {
  PrGetZkNoteAndLinks(GetZkNoteAndLinks),
  PrGetZnlIfChanged(GetZnlIfChanged),
  PrGetZkNotePubId(String),
}

// TODO: rename to Pbr
#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicReply {
  PrServerError(PublicError),
  PrZkNoteAndLinks(ZkNoteAndLinks),
  PrZkNoteAndLinksWhat(ZkNoteAndLinksWhat),
  PrNoop,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PublicError {
  PbeString(String),
  PbeNoteNotFound(PublicRequest),
  PbeNoteIsPrivate(PublicRequest),
}
