pub use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SystemVs {
  pub public: Uuid,
  pub search: Uuid,
  pub share: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct UserId(pub Uuid);

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
  pub id: UserId,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginData {
  pub userid: UserId,
  pub username: String,
  pub publicid: Uuid,
  pub shareid: Uuid,
  pub searchid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: Uuid,
  pub title: String,
  pub content: String,
  pub user: UserId,
  pub username: String,
  pub pubid: Option<String>,
  pub createdate: i64,
  pub changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkListNote {
  pub id: Uuid,
  pub title: String,
  pub user: UserId,
  pub createdate: i64,
  pub changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct SavedZkNote {
  pub id: Uuid,
  pub changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  pub id: Option<Uuid>,
  pub title: String,
  pub pubid: Option<String>,
  pub content: String,
}
#[derive(Deserialize, Debug, Clone)]
pub enum Direction {
  From,
  To,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkLink {
  pub otherid: Uuid,
  pub direction: Direction,
  pub user: UserId,
  pub delete: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNotePlusLinks {
  pub note: SaveZkNote,
  pub links: Vec<SaveZkLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  pub from: Uuid,
  pub to: Uuid,
  pub mine: bool,   // did I create this link?
  pub others: bool, // am I sole owner or do others own too
  pub delete: Option<bool>,
  pub fromname: Option<String>,
  pub toname: Option<String>,
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
  pub zknote: Uuid,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteEdit {
  pub zknote: Uuid,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteEdit {
  pub zknote: ZkNote,
  pub links: Vec<ZkLink>,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteAndAccomplices {
  pub zknote: ZkNote,
  pub links: Vec<ZkLink>,
}
