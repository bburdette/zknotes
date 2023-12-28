#![allow(non_snake_case)]

use crate::search::ZkListNoteSearchResult;
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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
  pub createddate_after: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkLinksSince {
  pub createddate_after: i64,
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

#[derive(Serialize, Debug, Clone)]
pub struct ZkNoteAndLinks {
  pub zknote: ZkNote,
  pub links: Vec<EditLink>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteAndLinksWhat {
  pub what: String,
  pub znl: ZkNoteAndLinks,
}
