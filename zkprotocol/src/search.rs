use crate::content::{ZkListNote, ZkNote};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub offset: i64,
  pub limit: Option<i64>,
  pub what: String,
  pub list: bool,
  pub archives: bool,
  pub created_after: Option<i64>,
  pub created_before: Option<i64>,
  pub changed_after: Option<i64>,
  pub changed_before: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TagSearch {
  SearchTerm {
    mods: Vec<SearchMod>,
    term: String,
  },
  Not {
    ts: Box<TagSearch>,
  },
  Boolex {
    ts1: Box<TagSearch>,
    ao: AndOr,
    ts2: Box<TagSearch>,
  },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SearchMod {
  ExactMatch,
  Tag,
  Note,
  User,
  File,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkListNoteSearchResult {
  pub notes: Vec<ZkListNote>,
  pub offset: i64,
  pub what: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearchResult {
  pub notes: Vec<ZkNote>,
  pub offset: i64,
  pub what: String,
}
