use crate::content::{ZkListNote, ZkNote, ZkNoteAndLinks, ZkNoteId};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub offset: i64,
  pub limit: Option<i64>,
  pub what: String,
  pub resulttype: ResultType,
  pub archives: bool,
  pub deleted: bool, // include deleted notes
  pub ordering: Option<Ordering>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Ordering {
  pub field: OrderField,
  pub direction: OrderDirection,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OrderDirection {
  Ascending,
  Descending,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OrderField {
  Title,
  Created,
  Changed,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum ResultType {
  RtId,
  RtListNote,
  RtNote,
  RtNoteAndLinks,
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
  ZkNoteId,
  Tag,
  Note,
  User,
  File,
  Before,
  After,
  Create,
  Mod,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkIdSearchResult {
  pub notes: Vec<ZkNoteId>,
  pub offset: i64,
  pub what: String,
}

#[derive(Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkSearchResultHeader {
  pub what: String,
  pub resulttype: ResultType,
  pub offset: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkNoteAndLinksSearchResult {
  pub notes: Vec<ZkNoteAndLinks>,
  pub offset: i64,
  pub what: String,
}
