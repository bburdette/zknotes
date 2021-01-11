use uuid::Uuid;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkNote {
  pub id: Uuid,
  pub title: String,
  pub content: String,
  pub user: Uuid,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  pub from: Uuid,
  pub to: Uuid,
  pub user: Uuid,
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
