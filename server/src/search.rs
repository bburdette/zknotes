use rusqlite::Connection;
use sqldata;
use sqldata::{note_id, user_id};
use std::error::Error;
use zkprotocol::content::ZkListNote;
use zkprotocol::search::{AndOr, SearchMod, TagSearch, ZkNoteSearch, ZkNoteSearchResult};

pub fn search_zknotes(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<ZkNoteSearchResult, Box<dyn Error>> {
  let (sql, args) = build_sql(&conn, user, search.clone())?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let rec_iter = pstmt.query_map(args.as_slice(), |row| {
    Ok(ZkListNote {
      id: row.get(0)?,
      title: row.get(1)?,
      user: row.get(2)?,
      createdate: row.get(3)?,
      changeddate: row.get(4)?,
    })
  })?;

  let mut pv = Vec::new();

  for rsrec in rec_iter {
    match rsrec {
      Ok(rec) => {
        pv.push(rec);
      }
      Err(_) => (),
    }
  }

  Ok(ZkNoteSearchResult {
    notes: pv,
    offset: search.offset,
  })
}

pub fn build_sql(
  conn: &Connection,
  uid: i64,
  search: ZkNoteSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (cls, clsargs) = build_sql_clause(&conn, uid, false, search.tagsearch)?;

  let publicid = note_id(&conn, "system", "public")?;

  let shareid = note_id(&conn, "system", "share")?;

  let limclause = match search.limit {
    Some(lm) => format!(" limit {} offset {}", lm, search.offset),
    None => format!(" offset {}", search.offset),
  };

  let ordclause = " order by N.id desc ";

  // notes that are mine.
  let mut sqlbase = format!(
    "select N.id, N.title, N.user, N.createdate, N.changeddate
      from zknote N where N.user = ?"
  );
  let mut baseargs = vec![uid.to_string()];

  // notes that are public, and not mine.
  let mut sqlpub = format!(
    "select N.id, N.title, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)"
  );
  let mut pubargs = vec![uid.to_string(), publicid.to_string()];

  // notes shared with a share tag.
  let usernoteid = sqldata::user_note_id(&conn, uid)?;
  // clause 1: user is not-me
  //
  // clause 2: is N linked to a share note?
  // link M is to shareid, and L links either to or from M's from.
  //
  // clause 3 is M.from (the share)
  // is that share linked to usernoteid?
  let mut sqlshare = format!(
    "select N.id, N.title, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ? and
        (M.toid = ? and (
          (L.fromid = N.id and L.toid = M.fromid ) or
          (L.toid = N.id and L.fromid = M.fromid )))
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))",
  );
  let mut shareargs = vec![
    uid.to_string(),
    shareid.to_string(),
    usernoteid.to_string(),
    usernoteid.to_string(),
  ];

  // notes that are tagged with my usernoteid, and not mine.
  let mut sqluser = format!(
    "select N.id, N.title, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)"
  );
  let mut userargs = vec![uid.to_string(), usernoteid.to_string()];

  // local ftn to add clause and args.
  let addcls = |sql: &mut String, args: &mut Vec<String>| {
    if !args.is_empty() {
      sql.push_str(" and ");
      sql.push_str(cls.as_str());

      // clone, otherwise no clause vals next time!
      let mut pendargs = clsargs.clone();
      args.append(&mut pendargs);
    }
  };

  addcls(&mut sqlbase, &mut baseargs);
  addcls(&mut sqlpub, &mut pubargs);
  addcls(&mut sqluser, &mut userargs);
  addcls(&mut sqlshare, &mut shareargs);

  // combine the queries.
  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlpub.as_str());
  baseargs.append(&mut pubargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlshare.as_str());
  baseargs.append(&mut shareargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqluser.as_str());
  baseargs.append(&mut userargs);

  // add order clause to the end.
  sqlbase.push_str(ordclause);

  // add limit clause to the end.
  sqlbase.push_str(limclause.as_str());

  Ok((sqlbase, baseargs))
}

fn build_sql_clause(
  conn: &Connection,
  uid: i64,
  not: bool,
  search: TagSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (cls, args) = match search {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut desc = false;
      let mut user = false;

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
          SearchMod::User => user = true,
        }
      }
      let field = if desc { "content" } else { "title" };

      if user {
        let user = user_id(conn, &term)?;
        let notstr = match not {
          true => "!",
          false => "",
        };
        (format!("N.user {}= ?", notstr), vec![format!("{}", user)])
      } else {
        if tag {
          let clause = if exact {
            format!("zkn.{} = ?", field)
          } else {
            format!("zkn.{}  like ?", field)
          };

          let notstr = if not { "not" } else { "" };

          (
            // clause
            format!(
              "{} (0 < (select count(zkn.id) from zknote as zkn, zklink
             where zkn.id = zklink.fromid
               and zklink.toid = N.id
               and {})
            or
                0 < (select count(zkn.id) from zknote as zkn, zklink
             where zkn.id = zklink.toid
               and zklink.fromid = N.id
               and {}))",
              notstr, clause, clause
            ),
            // args
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
          let notstr = match (not, exact) {
            (true, false) => "not",
            (false, false) => "",
            (true, true) => "!",
            (false, true) => "",
          };

          (
            // clause
            if exact {
              format!("N.{} {}= ?", field, notstr)
            } else {
              format!("N.{} {} like ?", field, notstr)
            },
            // args
            if exact {
              vec![term]
            } else {
              vec![format!("%{}%", term).to_string()]
            },
          )
        }
      }
    }
    TagSearch::Not { ts } => build_sql_clause(&conn, uid, true, *ts)?,
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (cl1, mut arg1) = build_sql_clause(&conn, uid, false, *ts1)?;
      let (cl2, mut arg2) = build_sql_clause(&conn, uid, false, *ts2)?;
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
  };
  Ok((cls, args))
}
