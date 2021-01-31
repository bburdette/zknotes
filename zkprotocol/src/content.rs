#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: i64,
  pub title: String,
  pub content: String,
  pub user: i64,
  pub username: String,
  pub pubid: Option<String>,
  pub createdate: i64,
  pub changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkListNote {
  pub id: i64,
  pub title: String,
  pub user: i64,
  pub createdate: i64,
  pub changeddate: i64,
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
}

#[derive(Deserialize, Debug, Clone)]
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
pub struct GetZkNoteEdit {
  pub zknote: i64,
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
