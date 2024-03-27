#![allow(non_snake_case)]

use crate::search::ZkListNoteSearchResult;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraLoginData {
  pub userid: i64,
  pub zknote: i64,
  pub homenote: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sysids {
  pub publicid: i64,
  pub shareid: i64,
  pub searchid: i64,
  pub commentid: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: i64,
  pub title: String,
  pub content: String,
  pub user: i64,
  pub username: String,
  pub usernote: i64,
  pub editable: bool,
  pub editableValue: bool,
  pub showtitle: bool,
  pub pubid: Option<String>,
  pub createdate: i64,
  pub changeddate: i64,
  pub deleted: bool,
  pub is_file: bool,
  pub sysids: Vec<i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkListNote {
  pub id: i64,
  pub title: String,
  pub is_file: bool,
  pub user: i64,
  pub createdate: i64,
  pub changeddate: i64,
  pub sysids: Vec<i64>,
}

#[derive(Serialize, Debug, Clone)]
pub struct SavedZkNote {
  pub id: i64,
  pub changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  pub id: Option<i64>,
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
  pub otherid: i64,
  pub direction: Direction,
  pub user: i64,
  pub zknote: Option<i64>,
  pub delete: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNotePlusLinks {
  pub note: SaveZkNote,
  pub links: Vec<SaveZkLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  pub from: i64,
  pub to: i64,
  pub user: i64,
  pub linkzknote: Option<i64>,
  pub delete: Option<bool>,
  pub fromname: Option<String>,
  pub toname: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditLink {
  pub otherid: i64,
  pub direction: Direction,
  pub user: i64,
  pub zknote: Option<i64>,
  pub othername: Option<String>,
  pub sysids: Vec<i64>,
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
  pub zknote: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteAndLinks {
  pub zknote: i64,
  pub what: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GetZnlIfChanged {
  pub zknote: i64,
  pub changeddate: i64,
  pub what: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteArchives {
  pub zknote: i64,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteArchives {
  pub zknote: i64,
  pub results: ZkListNoteSearchResult,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetArchiveZkNote {
  pub parentnote: i64,
  pub noteid: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteComments {
  pub zknote: i64,
  pub offset: i64,
  pub limit: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteAndLinks {
  pub zknote: ZkNote,
  pub links: Vec<EditLink>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteAndLinksWhat {
  pub what: String,
  pub znl: ZkNoteAndLinks,
}

#[derive(Serialize, Debug)]
pub struct ErrPrivateNote {
  pub what: String,
  pub zknote: i64,
}

#[derive(Serialize, Debug)]
pub enum ServerError {
  PrivateNote(ErrPrivateNote),
  String(String),
}
