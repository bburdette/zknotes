#![allow(non_snake_case)]

use std::fmt::Display;

use crate::search::ZkListNoteSearchResult;
use elm_rs::{Elm, ElmDecode, ElmEncode};
use orgauth::data::UserId;
use uuid::Uuid;

// pub type ZkNoteId = Uuid;
#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum ZkNoteId {
  Zni(Uuid),
}

impl Into<Uuid> for ZkNoteId {
  fn into(self) -> Uuid {
    match self {
      ZkNoteId::Zni(uuid) => uuid,
    }
  }
}

impl From<Uuid> for ZkNoteId {
  fn from(a: Uuid) -> Self {
    ZkNoteId::Zni(a)
  }
}

impl Display for ZkNoteId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ZkNoteId::Zni(uuid) => write!(f, "{}", uuid),
    }
  }
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct ExtraLoginData {
  pub userid: UserId,
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

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: ZkNoteId,
  pub title: String,
  pub content: String,
  pub user: UserId,
  pub username: String,
  pub usernote: ZkNoteId,
  pub editable: bool,
  pub editableValue: bool,
  pub showtitle: bool,
  pub pubid: Option<String>,
  pub createdate: i64,
  pub changeddate: i64,
  pub deleted: bool,
  pub filestatus: FileStatus,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub enum FileStatus {
  NotAFile,
  FileMissing,
  FilePresent,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct ZkListNote {
  pub id: ZkNoteId,
  pub title: String,
  pub filestatus: FileStatus,
  pub user: UserId,
  pub createdate: i64,
  pub changeddate: i64,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct SavedZkNote {
  pub id: ZkNoteId,
  pub changeddate: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  pub id: Option<ZkNoteId>,
  pub title: String,
  pub pubid: Option<String>,
  pub content: String,
  pub editable: bool,
  pub showtitle: bool,
  pub deleted: bool,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub enum Direction {
  From,
  To,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct SaveZkLink {
  pub otherid: ZkNoteId,
  pub direction: Direction,
  pub user: UserId,
  pub zknote: Option<ZkNoteId>,
  pub delete: Option<bool>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct SaveZkNoteAndLinks {
  pub note: SaveZkNote,
  pub links: Vec<SaveZkLink>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  pub from: ZkNoteId,
  pub to: ZkNoteId,
  pub user: UserId,
  pub linkzknote: Option<ZkNoteId>,
  pub delete: Option<bool>,
  pub fromname: Option<String>,
  pub toname: Option<String>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct EditLink {
  pub otherid: ZkNoteId,
  pub direction: Direction,
  pub user: UserId,
  pub zknote: Option<ZkNoteId>,
  pub othername: Option<String>,
  pub delete: Option<bool>,
  pub sysids: Vec<ZkNoteId>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct ZkLinks {
  pub links: Vec<ZkLink>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct ImportZkNote {
  pub title: String,
  pub content: String,
  pub fromLinks: Vec<String>,
  pub toLinks: Vec<String>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetZkLinks {
  pub zknote: ZkNoteId,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetZkNoteAndLinks {
  pub zknote: ZkNoteId,
  pub what: String,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug, Clone)]
pub struct GetZnlIfChanged {
  pub zknote: ZkNoteId,
  pub changeddate: i64,
  pub what: String,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetZkNoteArchives {
  pub zknote: ZkNoteId,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub struct ZkNoteArchives {
  pub zknote: ZkNoteId,
  pub results: ZkListNoteSearchResult,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetArchiveZkNote {
  pub parentnote: ZkNoteId,
  pub noteid: ZkNoteId,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetArchiveZkLinks {
  pub createddate_after: Option<i64>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetZkLinksSince {
  pub createddate_after: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SyncSince {
  pub after: Option<i64>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct FileInfo {
  pub hash: String,
  pub size: u64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct ArchiveZkLink {
  pub userUuid: String, // uuid too!
  pub fromUuid: String,
  pub toUuid: String,
  pub linkUuid: Option<String>,
  pub createdate: i64,
  pub deletedate: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct UuidZkLink {
  pub userUuid: String, // uuid too!
  pub fromUuid: String,
  pub toUuid: String,
  pub linkUuid: Option<String>,
  pub createdate: i64,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct GetZkNoteComments {
  pub zknote: ZkNoteId,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteAndLinks {
  pub zknote: ZkNote,
  pub links: Vec<EditLink>,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub struct ZkNoteAndLinksWhat {
  pub what: String,
  pub znl: ZkNoteAndLinks,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub enum JobState {
  Started,
  Running,
  Completed,
  Failed,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug, Clone)]
pub struct JobStatus {
  pub jobno: i64,
  pub state: JobState,
  pub message: String,
}
