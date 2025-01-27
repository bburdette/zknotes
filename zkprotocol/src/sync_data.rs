use orgauth::data::UserId;
use uuid::Uuid;

use crate::{
  content::{ArchiveZkLink, FileInfo, UuidZkLink, ZkNote},
  search::ZkSearchResultHeader,
};

// Represents a remote user that is not registered on this server.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ZkPhantomUser {
  pub id: UserId,
  pub uuid: Uuid,   // uuid in orgauth_user record.
  pub data: String, // uuid in user note.
  pub name: String,
  pub active: bool,
}

// TODO: add time on first msg.
#[derive(Deserialize, Serialize, Debug)]
pub enum SyncMessage {
  SyncStart(Option<i64>, i64),
  PhantomUserHeader,
  PhantomUser(ZkPhantomUser),
  ZkSearchResultHeader(ZkSearchResultHeader),
  ZkNoteId(String),
  ZkNote(ZkNote, Option<FileInfo>),
  ArchiveZkLinkHeader,
  ArchiveZkLink(ArchiveZkLink),
  UuidZkLinkHeader,
  UuidZkLink(UuidZkLink),
  SyncError(String),
  SyncEnd,
}

impl From<ZkPhantomUser> for SyncMessage {
  fn from(a: ZkPhantomUser) -> Self {
    SyncMessage::PhantomUser(a)
  }
}

impl From<ZkSearchResultHeader> for SyncMessage {
  fn from(a: ZkSearchResultHeader) -> Self {
    SyncMessage::ZkSearchResultHeader(a)
  }
}

impl From<(ZkNote, Option<FileInfo>)> for SyncMessage {
  fn from(a: (ZkNote, Option<FileInfo>)) -> Self {
    SyncMessage::ZkNote(a.0, a.1)
  }
}

impl From<ArchiveZkLink> for SyncMessage {
  fn from(a: ArchiveZkLink) -> Self {
    SyncMessage::ArchiveZkLink(a)
  }
}

impl From<UuidZkLink> for SyncMessage {
  fn from(a: UuidZkLink) -> Self {
    SyncMessage::UuidZkLink(a)
  }
}
