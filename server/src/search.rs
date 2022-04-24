use crate::sqldata;
use crate::sqldata::{get_sysids, note_id, power_delete_zknote, zknote_access_id};
use either::Either;
use either::Either::{Left, Right};
use orgauth::dbfun::user_id;
use rusqlite::Connection;
use std::convert::TryInto;
use std::error::Error;
use zkprotocol::content::ZkListNote;
use zkprotocol::search::{
  AndOr, SearchMod, TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult,
};

pub fn power_delete_zknotes(
  conn: &Connection,
  user: i64,
  search: &TagSearch,
) -> Result<i64, Box<dyn Error>> {
  // get all, and delete all.  Maybe not a good idea for a big database, but ours is small
  // and soon to be replaced with indradb, perhaps.

  let nolimsearch = ZkNoteSearch {
    tagsearch: search.clone(),
    offset: 0,
    limit: None,
    what: "".to_string(),
    list: true,
  };

  let znsr = search_zknotes(conn, user, &nolimsearch)?;
  match znsr {
    Left(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        power_delete_zknote(conn, user, n.id)?;
      }
      Ok(c)
    }
    Right(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        power_delete_zknote(conn, user, n.id)?;
      }
      Ok(c)
    }
  }
}

pub fn search_zknotes(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<Either<ZkListNoteSearchResult, ZkNoteSearchResult>, Box<dyn Error>> {
  let (sql, args) = build_sql(&conn, user, search.clone())?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let sysid = user_id(&conn, "system")?;

  let rec_iter = pstmt.query_map(args.as_slice(), |row| {
    let id = row.get(0)?;
    let sysids = get_sysids(conn, sysid, id)?;
    Ok(ZkListNote {
      id: id,
      title: row.get(1)?,
      user: row.get(2)?,
      createdate: row.get(3)?,
      changeddate: row.get(4)?,
      sysids: sysids,
    })
  })?;

  if search.list {
    let mut pv = Vec::new();

    for rsrec in rec_iter {
      match rsrec {
        Ok(rec) => {
          pv.push(rec);
        }
        Err(_) => (),
      }
    }

    Ok(Left(ZkListNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  } else {
    let mut pv = Vec::new();

    for rsrec in rec_iter {
      match rsrec {
        Ok(rec) => {
          pv.push(sqldata::read_zknote(&conn, Some(user), rec.id)?);
        }
        Err(_) => (),
      }
    }

    Ok(Right(ZkNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  }
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
    None => "".to_string(), // no limit, no offset either
                            // None => format!(" offset {}", search.offset),
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
      where (
        N.user != ? and
        ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?)))"
  );
  let mut userargs = vec![
    uid.to_string(),
    usernoteid.to_string(),
    usernoteid.to_string(),
  ];

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

pub fn search_zknotes_simple(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<Either<ZkListNoteSearchResult, ZkNoteSearchResult>, Box<dyn Error>> {
  let (sql, args) = build_simple_sql(&conn, user, search.clone())?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let sysid = user_id(&conn, "system")?;

  let rec_iter = pstmt.query_map(args.as_slice(), |row| {
    let id = row.get(0)?;
    let sysids = get_sysids(conn, sysid, id)?;
    let access = match zknote_access_id(conn, Some(user), id) {
      Ok(sqldata::Access::Read) => true,
      Ok(sqldata::Access::ReadWrite) => true,
      Ok(sqldata::Access::Private) => false,
      Err(_) => false,
    };
    if access {
      Ok(Some(ZkListNote {
        id: id,
        title: row.get(1)?,
        user: row.get(2)?,
        createdate: row.get(3)?,
        changeddate: row.get(4)?,
        sysids: sysids,
      }))
    } else {
      Ok(None)
    }
  })?;

  if search.list {
    let mut pv = Vec::new();

    for rsrec in rec_iter {
      match rsrec {
        Ok(Some(rec)) => {
          pv.push(rec);
        }
        _ => (),
      }
    }

    Ok(Left(ZkListNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  } else {
    let mut pv = Vec::new();

    for rsrec in rec_iter {
      match rsrec {
        Ok(Some(rec)) => {
          pv.push(sqldata::read_zknote(&conn, Some(user), rec.id)?);
        }
        _ => (),
      }
    }

    Ok(Right(ZkNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  }
}

pub fn build_simple_sql(
  conn: &Connection,
  uid: i64,
  search: ZkNoteSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (cls, clsargs) = build_sql_clause(&conn, uid, false, search.tagsearch)?;

  let limclause = match search.limit {
    Some(lm) => format!(" limit {} offset {}", lm, search.offset),
    None => "".to_string(), // no limit, no offset either
                            // None => format!(" offset {}", search.offset),
  };

  let ordclause = " order by N.id desc ";

  // all notes.
  let mut sqlbase = format!(
    "select N.id, N.title, N.user, N.createdate, N.changeddate
      from zknote N "
  );

  if cls != "" {
    sqlbase.push_str(" where ");
    sqlbase.push_str(cls.as_str());
  }

  // add order clause to the end.
  sqlbase.push_str(ordclause);

  // add limit clause to the end.
  sqlbase.push_str(limclause.as_str());

  Ok((sqlbase, clsargs))
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
              "{} (N.id in (select zklink.toid from zknote as zkn, zklink
             where zkn.id = zklink.fromid
               and {})
            or
                N.id in (select zklink.fromid from zknote as zkn, zklink
             where zkn.id = zklink.toid
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
