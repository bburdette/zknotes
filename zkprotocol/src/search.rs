use crate::content::ZkListNote;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub offset: i64,
  pub limit: Option<i64>,
  pub order: Option<Order>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OrderBy {
  CreateDate,
  Changeddate,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum OrderDirection {
  Ascending,
  Descending,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Order {
  pub by: OrderBy,
  pub direction: OrderDirection,
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
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkNoteSearchResult {
  pub notes: Vec<ZkListNote>,
  pub offset: i64,
}
