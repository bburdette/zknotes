#![allow(non_snake_case)]

use crate::search::{ZkListNoteSearchResult, ZkSearchResultHeader};
use uuid::Uuid;

pub type ZkNoteId = Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraLoginData {
  pub userid: i64,
  pub zknote: ZkNoteId,
  pub homenote: Option<ZkNoteId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sysids {
  pub publicid: ZkNoteId,
  pub commentid: ZkNoteId,
  pub shareid: ZkNoteId,
  pub searchid: ZkNoteId,
  pub userid: ZkNoteId,
  pub archiveid: ZkNoteId,
  pub systemid: ZkNoteId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: ZkNoteId,
  pub title: String,
  pub content: String,
  pub user: i64,
  pub username: String,
  pub usernote: ZkNoteId,
  pub editable: bool,
  pub editableValue: bool,
  pub showtitle: bool,
  pub pubid: Option<String>,
  pub createdate: i64,
  pub changeddate: i64,
  pub deleted: bool,
  pub is_file: bool,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct ZkListNote {
  pub id: ZkNoteId,
  pub title: String,
  pub is_file: bool,
  pub user: i64,
  pub createdate: i64,
  pub changeddate: i64,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SavedZkNote {
  pub id: ZkNoteId,
  pub changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  pub id: Option<ZkNoteId>,
  pub title: String,
  pub pubid: Option<String>,
  pub content: String,
  pub editable: bool,
  pub showtitle: bool,
  pub deleted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Direction {
  From,
  To,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkLink {
  pub otherid: ZkNoteId,
  pub direction: Direction,
  pub user: i64,
  pub zknote: Option<ZkNoteId>,
  pub delete: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNoteAndLinks {
  pub note: SaveZkNote,
  pub links: Vec<SaveZkLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  pub from: ZkNoteId,
  pub to: ZkNoteId,
  pub user: i64,
  pub linkzknote: Option<ZkNoteId>,
  pub delete: Option<bool>,
  pub fromname: Option<String>,
  pub toname: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditLink {
  pub otherid: ZkNoteId,
  pub direction: Direction,
  pub user: i64,
  pub zknote: Option<ZkNoteId>,
  pub othername: Option<String>,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLinks {
  pub links: Vec<ZkLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportZkNote {
  pub title: String,
  pub content: String,
  pub fromLinks: Vec<String>,
  pub toLinks: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkLinks {
  pub zknote: ZkNoteId,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteAndLinks {
  pub zknote: ZkNoteId,
  pub what: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetZnlIfChanged {
  pub zknote: ZkNoteId,
  pub changeddate: i64,
  pub what: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteArchives {
  pub zknote: ZkNoteId,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteArchives {
  pub zknote: ZkNoteId,
  pub results: ZkListNoteSearchResult,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetArchiveZkNote {
  pub parentnote: ZkNoteId,
  pub noteid: ZkNoteId,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetArchiveZkLinks {
  pub createddate_after: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkLinksSince {
  pub createddate_after: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SyncSince {
  pub after: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FileInfo {
  pub hash: String,
  pub size: u64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ArchiveZkLink {
  pub userUuid: String, // uuid too!
  pub fromUuid: String,
  pub toUuid: String,
  pub linkUuid: Option<String>,
  pub createdate: i64,
  pub deletedate: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UuidZkLink {
  pub userUuid: String, // uuid too!
  pub fromUuid: String,
  pub toUuid: String,
  pub linkUuid: Option<String>,
  pub createdate: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteComments {
  pub zknote: ZkNoteId,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteAndLinks {
  pub zknote: ZkNote,
  pub links: Vec<EditLink>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteAndLinksWhat {
  pub what: String,
  pub znl: ZkNoteAndLinks,
}

// Represents a remote user that is not registered on this server.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ZkPhantomUser {
  pub id: i64,
  pub uuid: Uuid,              // uuid in orgauth_user record.
  pub data: serde_json::Value, // uuid in user note.
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
  // ZkListNote(ZkListNote),
  ZkNote(ZkNote, Option<FileInfo>),
  // ZkNoteAndLinks(ZkNoteAndLinks),
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

// impl From<ZkListNote> for SyncMessage {
//   fn from(a: ZkListNote) -> Self {
//     SyncMessage::ZkListNote(a)
//   }
// }

impl From<(ZkNote, Option<FileInfo>)> for SyncMessage {
  fn from(a: (ZkNote, Option<FileInfo>)) -> Self {
    SyncMessage::ZkNote(a.0, a.1)
  }
}

// impl From<ZkNoteAndLinks> for SyncMessage {
//   fn from(a: ZkNoteAndLinks) -> Self {
//     SyncMessage::ZkNoteAndLinks(a)
//   }
// }

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
