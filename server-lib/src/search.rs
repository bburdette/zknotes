use crate::sqldata;
use crate::sqldata::{delete_zknote, get_sysids, note_id};
use async_stream::try_stream;
use bytes::Bytes;
use either::Either;
use either::Either::{Left, Right};
use futures::Stream;
use orgauth::dbfun::user_id;
use ouroboros::self_referencing;
use rusqlite::{Connection, MappedRows, Row};
use std::cell::Cell;
use std::convert::TryInto;
use std::error::Error;
use std::iter::IntoIterator;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use zkprotocol::content::ZkListNote;
use zkprotocol::search::{
  AndOr, SearchMod, TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult,
};

pub fn power_delete_zknotes(
  conn: &Connection,
  file_path: PathBuf,
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
    archives: false,
    created_after: None,
    created_before: None,
    changed_after: None,
    changed_before: None,
  };

  let znsr = search_zknotes(conn, user, &nolimsearch)?;
  match znsr {
    Left(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, n.id)?;
      }
      Ok(c)
    }
    Right(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, n.id)?;
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

  let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), |row| {
    let id = row.get(0)?;
    let sysids = get_sysids(conn, sysid, id)?;
    Ok(ZkListNote {
      id: id,
      title: row.get(1)?,
      is_file: {
        let wat: Option<i64> = row.get(2)?;
        match wat {
          Some(_) => true,
          None => false,
        }
      },
      user: row.get(3)?,
      createdate: row.get(4)?,
      changeddate: row.get(5)?,
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

    let s_user = if search.archives { sysid } else { user };

    for rsrec in rec_iter {
      match rsrec {
        Ok(rec) => {
          pv.push(sqldata::read_zknote(&conn, Some(s_user), rec.id)?);
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

pub fn zkn_stream(
  conn: Arc<Connection>,
  user: i64,
  search: ZkNoteSearch,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> + 'static {
  try_stream! {
    let (sql, args) = build_sql(&conn, user, search.clone())?;
    let mut stmt = conn.prepare(sql.as_str())?;

    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;

    while let Some(row) = rows.next()? {
      let zln = ZkListNote {
        id: row.get(0)?,
        title: row.get(1)?,
        is_file: {
          let wat: Option<i64> = row.get(2)?;
          wat.is_some()
        },
        user: row.get(3)?,
        createdate: row.get(4)?,
        changeddate: row.get(5)?,
        sysids: Vec::new(),
      };

      yield Bytes::from(serde_json::to_value(zln)?.to_string())
    }
  }
}

pub fn search_zknotes_stream(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<Either<ZkListNoteSearchResult, ZkNoteSearchResult>, Box<dyn Error>> {
  Err("wat".into())
}

pub fn build_sql(
  conn: &Connection,
  uid: i64,
  search: ZkNoteSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (mut dtcls, mut dtclsargs) = build_daterange_clause(&search)?;

  println!("dtcls: {:?}", dtcls);

  let (cls, clsargs) = build_tagsearch_clause(&conn, uid, false, &search.tagsearch)?;

  let publicid = note_id(&conn, "system", "public")?;
  let archiveid = note_id(&conn, "system", "archive")?;
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = sqldata::user_note_id(&conn, uid)?;

  let limclause = match search.limit {
    Some(lm) => format!(" limit {} offset {}", lm, search.offset),
    None => "".to_string(), // no limit, no offset either
                            // None => format!(" offset {}", search.offset),
  };

  let ordclause = " order by N.changeddate desc ";

  let archives = search.archives;

  let (mut sqlbase, mut baseargs) = if archives {
    (
      // archives of notes that are mine.
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zknote O, zklink OL, zklink AL where
      (O.user = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
      and O.deleted = 0"
      ),
      vec![uid.to_string(), archiveid.to_string()],
    )
  } else {
    // notes that are mine.
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N where N.user = ?
      and N.deleted = 0"
      ),
      vec![uid.to_string()],
    )
  };

  // notes that are public, and not mine.
  let (mut sqlpub, mut pubargs) = if archives {
    (
      // archives of notes that are public, and not mine.
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where (N.user != ?
        and L.fromid = N.id and L.toid = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
      and N.deleted = 0"
      ),
      vec![uid.to_string(), publicid.to_string(), archiveid.to_string()],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)
      and N.deleted = 0"
      ),
      vec![uid.to_string(), publicid.to_string()],
    )
  };

  // notes shared with a share tag, and not mine.
  // clause 1: user is not-me
  //
  // clause 2: is N linked to a share note?
  // link M is to shareid, and L links either to or from M's from.
  //
  // clause 3 is M.from (the share)
  // is that share linked to usernoteid?
  let (mut sqlshare, mut shareargs) = if archives {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U, zknote O, zklink OL, zklink AL
      where (N.user != ? and
        (M.toid = ? and (
          (L.fromid = N.id and L.toid = M.fromid ) or
          (L.toid = N.id and L.fromid = M.fromid ))
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?))
      and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        archiveid.to_string(),
        archiveid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ? 
        and M.toid = ? 
        and ((L.fromid = N.id and L.toid = M.fromid )
             or (L.toid = N.id and L.fromid = M.fromid ))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))
      and N.deleted = 0",
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        archiveid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  };

  // notes that are tagged with my usernoteid, and not mine.
  let (mut sqluser, mut userargs) = if archives {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where N.user != ?
        and ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?))
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?
        and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
        archiveid.to_string(),
      ],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (
        N.user != ? and
        ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?)))
        and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  };
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
  addcls(&mut dtcls, &mut dtclsargs);

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

fn build_daterange_clause(search: &ZkNoteSearch) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let clawses = [
    search
      .created_after
      .map(|dt| ("N.createddate < ?", dt.to_string())),
    search
      .created_before
      .map(|dt| ("N.createddate < ?", dt.to_string())),
    search
      .changed_after
      .map(|dt| ("N.changeddate < ?", dt.to_string())),
    search
      .changed_before
      .map(|dt| ("N.changeddate < ?", dt.to_string())),
  ];
  let clause = clawses
    .iter()
    .filter_map(|pair| pair.as_ref().map(|(s, _)| s.to_string()))
    .collect::<Vec<String>>()
    .join(" and ");
  let args = clawses
    .iter()
    .filter_map(|pair| pair.as_ref().map(|(_, dt)| dt.clone()))
    .collect();
  Ok((clause, args))
}

fn build_tagsearch_clause(
  conn: &Connection,
  uid: i64,
  not: bool,
  search: &TagSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (cls, args) = match search {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut desc = false;
      let mut user = false;
      let mut file = false;

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
          SearchMod::User => user = true,
          SearchMod::File => file = true,
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
          let fileclause = if file { "and zkn.file is not null" } else { "" };

          let clause = if exact {
            format!("zkn.{} = ? {}", field, fileclause)
          } else {
            format!("zkn.{}  like ? {}", field, fileclause)
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
              vec![term.clone(), term.clone()]
            } else {
              vec![
                format!("%{}%", term).to_string(),
                format!("%{}%", term).to_string(),
              ]
            },
          )
        } else {
          let fileclause = if file { "and N.file is not null" } else { "" };

          let notstr = match (not, exact) {
            (true, false) => "not",
            (false, false) => "",
            (true, true) => "!",
            (false, true) => "",
          };

          (
            // clause
            if exact {
              format!("N.{} {}= ? {}", field, notstr, fileclause)
            } else {
              format!("N.{} {} like ? {}", field, notstr, fileclause)
            },
            // args
            if exact {
              vec![term.clone()]
            } else {
              vec![format!("%{}%", term).to_string()]
            },
          )
        }
      }
    }
    TagSearch::Not { ts } => build_tagsearch_clause(&conn, uid, true, &*ts)?,
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (cl1, mut arg1) = build_tagsearch_clause(&conn, uid, false, &*ts1)?;
      let (cl2, mut arg2) = build_tagsearch_clause(&conn, uid, false, &*ts2)?;
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
