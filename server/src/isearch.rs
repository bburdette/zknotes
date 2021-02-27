use crate::icontent::ZkListNote;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub offset: Option<Uuid>,
  pub limit: Option<i64>,
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

#[derive(PartialEq, Eq, Deserialize, Serialize, Debug, Clone)]
pub enum SearchMod {
  ExactMatch,
  Tag,
  Note,
  User,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkNoteSearchResult {
  pub notes: Vec<ZkListNote>,
  pub offset: Option<Uuid>,
}
