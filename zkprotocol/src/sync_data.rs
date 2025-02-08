use orgauth::{data::UserId, util::show_time};
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

pub fn show_syncmsg_logformat(sm: &SyncMessage) -> String {
  // TODO: add time on first msg.
  match sm {
    SyncMessage::SyncStart(from, to) => {
      format!(
        "SyncStart from {}, to {}",
        from
          .and_then(|x| show_time(x / 1000))
          .unwrap_or("<no date>".to_string()),
        show_time(*to / 1000).unwrap_or("".to_string())
      )
    }
    SyncMessage::PhantomUserHeader => {
      format!("PhantomUserHeader")
    }
    SyncMessage::PhantomUser(zpu) => {
      format!("PhantomUser {} {}", zpu.uuid, zpu.name)
    }
    SyncMessage::ZkSearchResultHeader(_zksearchresultheader) => {
      format!("ZkSearchResultHeader")
    }
    SyncMessage::ZkNoteId(str) => {
      format!("ZkNoteId {}", str)
    }
    SyncMessage::ZkNote(zknote, _mbfileinfo) => {
      format!("ZkNote {}, {}", zknote.title, zknote.id)
    }
    SyncMessage::ArchiveZkLinkHeader => {
      format!("ArchiveZkLinkHeader")
    }
    SyncMessage::ArchiveZkLink(archivezklink) => {
      format!(
        "ArchiveZkLink from {}, to {}",
        archivezklink.fromUuid, archivezklink.toUuid
      )
    }
    SyncMessage::UuidZkLinkHeader => {
      format!("UuidZkLinkHeader")
    }
    SyncMessage::UuidZkLink(uuidzklink) => {
      format!(
        "UuidZkLink from {}, to {}",
        uuidzklink.fromUuid, uuidzklink.toUuid
      )
    }
    SyncMessage::SyncError(string) => {
      format!("SyncError: {}", string)
    }
    SyncMessage::SyncEnd => {
      format!("SyncEnd")
    }
  }
}
