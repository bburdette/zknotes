use barrel::backend::Sqlite;
use barrel::{types, Migration};
use rusqlite::{params, Connection};
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub zks: Vec<i64>,
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
  CaseSensitive,
  ExactMatch,
  Tag,
  Description,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

pub fn buildSql(uid: i64, search: TagSearch) -> (String, Vec<String>) {
  let (cls, mut clsargs) = buildSqlClause(false, search);
  let mut sqlbase = "SELECT id, title, zk, createdate, changeddate
      FROM zknote where zk IN (select zk from zkmember where user = ?)"
    .to_string();
  let mut args = vec![uid.to_string()];

  if args.is_empty() {
    (sqlbase, args)
  } else {
    sqlbase.push_str(" and ");
    sqlbase.push_str(cls.as_str());
    args.append(&mut clsargs);
    (sqlbase, args)
  }
}

fn buildSqlClause(not: bool, search: TagSearch) -> (String, Vec<String>) {
  match search {
    TagSearch::SearchTerm { mods, term } => {
      if not {
        (
          "title not like ?".to_string(),
          vec![format!("%{}%", term).to_string()],
        )
      } else {
        (
          "title like ?".to_string(),
          vec![format!("%{}%", term).to_string()],
        )
      }
    }
    TagSearch::Not { ts } => buildSqlClause(true, *ts),
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (mut cl1, mut arg1) = buildSqlClause(false, *ts1);
      let (mut cl2, mut arg2) = buildSqlClause(false, *ts2);
      let mut cls = String::new();
      println!("ao: {:?}", ao);
      let conj = match ao {
        AndOr::Or => " or ",
        AndOr::And => " and ",
      };
      println!("conj: {}", conj);
      cls.push_str("(");
      cls.push_str(cl1.as_str());
      cls.push_str(conj);
      cls.push_str(cl2.as_str());
      cls.push_str(")");
      println!("cls: {}", cls);
      arg1.append(&mut arg2);
      (cls, arg1)
    }
  }
}
