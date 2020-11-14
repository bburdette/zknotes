#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ZkNoteSearch {
  pub tagsearch: TagSearch,
  pub offset: i64,
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SearchMod {
  ExactMatch,
  Tag,
  Note,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AndOr {
  And,
  Or,
}

pub fn build_sql(uid: i64, search: ZkNoteSearch) -> (String, Vec<String>) {
  let (cls, mut clsargs) = build_sql_clause(false, search.tagsearch);

  // let zklist = format!("{:?}", search.zks)
  //   .replace("[", "(")
  //   .replace("]", ")");

  let limclause = match search.limit {
    Some(lm) => format!(" limit {} offset {}", lm, search.offset),
    None => format!(" offset {}", search.offset),
  };

  let mut sqlbase = format!(
    "SELECT id, title, user, createdate, changeddate
      FROM zknote "
  );
  // let mut args = vec![uid.to_string()];
  let mut args = vec![];

  if clsargs.is_empty() {
    sqlbase.push_str(limclause.as_str());

    (sqlbase, args)
  } else {
    // sqlbase.push_str(" and ");
    // sqlbase.push_str(" where ");
    // sqlbase.push_str(cls.as_str());
    // sqlbase.push_str(limclause.as_str());

    // args.append(&mut clsargs);

    (sqlbase, args)
  }
}

fn build_sql_clause(not: bool, search: TagSearch) -> (String, Vec<String>) {
  match search {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut desc = false;

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
        }
      }
      let field = if desc { "content" } else { "title" };
      let notstr = match (not, exact) {
        (true, false) => "not",
        (false, false) => "",
        (true, true) => "!",
        (false, true) => "",
      };

      if tag {
        let clause = if exact {
          format!("{} {}= ?", field, notstr)
        } else {
          format!("{} {} like ?", field, notstr)
        };

        (
          format!(
            "(0 < (select count(zkn.id) from zknote as zkn, zklink
             where zkn.id = zklink.fromid
               and zklink.toid = zknote.id
               and {})
          or
          0 < (select count(zkn.id) from zknote as zkn, zklink
             where zkn.id = zklink.toid
               and zklink.fromid = zknote.id
               and {}))",
            clause, clause
          ),
          if exact {
            vec![term.clone(), term]
          } else {
            vec![
              format!("%{}%", term).to_string(),
              format!("%{}%", term).to_string(),
            ]
          },
        )
      } else {
        (
          if exact {
            format!("{} {}= ?", field, notstr)
          } else {
            format!("{} {} like ?", field, notstr)
          },
          if exact {
            vec![term]
          } else {
            vec![format!("%{}%", term).to_string()]
          },
        )
      }
    }
    TagSearch::Not { ts } => build_sql_clause(true, *ts),
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (cl1, mut arg1) = build_sql_clause(false, *ts1);
      let (cl2, mut arg2) = build_sql_clause(false, *ts2);
      let mut cls = String::new();
      let conj = match ao {
        AndOr::Or => " or ",
        AndOr::And => " and ",
      };
      cls.push_str("(");
      cls.push_str(cl1.as_str());
      cls.push_str(conj);
      cls.push_str(cl2.as_str());
      cls.push_str(")");
      arg1.append(&mut arg2);
      (cls, arg1)
    }
  }
}
