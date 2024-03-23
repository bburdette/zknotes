use crate::error as zkerr;
use crate::sqldata;
use crate::sqldata::{delete_zknote, get_sysids, note_id};
use async_stream::try_stream;
use futures::Stream;
use orgauth::dbfun::user_id;
use rusqlite::Connection;
use std::convert::TryInto;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use zkprotocol::constants::SpecialUuids;
use zkprotocol::content::{SyncMessage, ZkListNote, ZkPhantomUser};
use zkprotocol::search::{
  AndOr, OrderDirection, OrderField, ResultType, SearchMod, TagSearch, ZkIdSearchResult,
  ZkListNoteSearchResult, ZkNoteAndLinksSearchResult, ZkNoteSearch, ZkNoteSearchResult,
  ZkSearchResultHeader,
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
    resulttype: ResultType::RtListNote,
    archives: false,
    deleted: false,
    ordering: None,
  };
  let znsr = search_zknotes(conn, user, &nolimsearch)?;
  match znsr {
    SearchResult::SrId(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n)?;
      }
      Ok(c)
    }
    SearchResult::SrListNote(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.id)?;
      }
      Ok(c)
    }
    SearchResult::SrNote(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.id)?;
      }
      Ok(c)
    }
    SearchResult::SrNoteAndLink(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.zknote.id)?;
      }
      Ok(c)
    }
  }
}

pub enum SearchResult {
  SrId(ZkIdSearchResult),
  SrListNote(ZkListNoteSearchResult),
  SrNote(ZkNoteSearchResult),
  SrNoteAndLink(ZkNoteAndLinksSearchResult),
}

pub fn search_zknotes(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<SearchResult, zkerr::Error> {
  let (sql, args) = build_sql(&conn, user, &search, None)?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let sysid = user_id(&conn, "system")?;

  let rec_iter = pstmt.query_and_then(rusqlite::params_from_iter(args.iter()), |row| {
    let id = row.get(0)?;
    let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
    let sysids = get_sysids(conn, sysid, id)?;
    Ok::<ZkListNote, zkerr::Error>(ZkListNote {
      id: uuid,
      title: row.get(2)?,
      is_file: {
        let wat: Option<i64> = row.get(3)?;
        match wat {
          Some(_) => true,
          None => false,
        }
      },
      user: row.get(4)?,
      createdate: row.get(5)?,
      changeddate: row.get(6)?,
      sysids,
    })
  })?;

  match search.resulttype {
    ResultType::RtId => {
      let mut pv = Vec::new();

      for rsrec in rec_iter {
        match rsrec {
          Ok(rec) => {
            pv.push(rec.id);
          }
          Err(_) => (),
        }
      }

      Ok(SearchResult::SrId(ZkIdSearchResult {
        notes: pv,
        offset: search.offset,
        what: search.what.clone(),
      }))
    }
    ResultType::RtListNote => {
      let mut pv = Vec::new();

      for rsrec in rec_iter {
        match rsrec {
          Ok(rec) => {
            pv.push(rec);
          }
          Err(_) => (),
        }
      }

      Ok(SearchResult::SrListNote(ZkListNoteSearchResult {
        notes: pv,
        offset: search.offset,
        what: search.what.clone(),
      }))
    }
    ResultType::RtNote => {
      let mut pv = Vec::new();

      for rsrec in rec_iter {
        match rsrec {
          Ok(rec) => {
            pv.push(sqldata::read_zknote(&conn, Some(user), &rec.id)?.1);
          }
          Err(_) => (),
        }
      }

      Ok(SearchResult::SrNote(ZkNoteSearchResult {
        notes: pv,
        offset: search.offset,
        what: search.what.clone(),
      }))
    }
    ResultType::RtNoteAndLinks => {
      let mut pv = Vec::new();

      for rsrec in rec_iter {
        match rsrec {
          Ok(rec) => {
            pv.push(sqldata::read_zknoteandlinks(&conn, Some(user), &rec.id)?);
          }
          Err(_) => (),
        }
      }

      Ok(SearchResult::SrNoteAndLink(ZkNoteAndLinksSearchResult {
        notes: pv,
        offset: search.offset,
        what: search.what.clone(),
      }))
    }
  }
}

pub fn search_zknotes_stream(
  conn: Arc<Connection>,
  user: i64,
  search: ZkNoteSearch,
  exclude_notes: Option<String>,
  what: String,
) -> impl Stream<Item = Result<SyncMessage, Box<dyn std::error::Error + 'static>>> {
  // uncomment for formatting, lsp
  // {
  try_stream! {

    println!("search_zknotes_stream - what: {} \nsearch {:?}", what, search);

    // let sysid = user_id(&conn, "system")?;
    let user = if search.archives {
      user_id(&conn, "system")?
    } else {
      user
    };

    let (sql, args) = build_sql(&conn, user, &search, exclude_notes)?;

    println!("zknote search what: {} sql {}", what, sql);
    println!("zknote search what: {} args {:?}", what, args);

    let mut stmt = conn.prepare(sql.as_str())?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    yield SyncMessage::from(ZkSearchResultHeader {
      what: search.what,
      resulttype: search.resulttype,
      offset: search.offset,
    });

    while let Some(row) = rows.next()? {
      let title = row.get::<usize, String>(2)?;
      println!("zknote title {}", title);
      match search.resulttype {
        ResultType::RtId => {
          yield SyncMessage::ZkNoteId(row.get::<usize, String>(1)?)
        }
        ResultType::RtListNote => {
          let zln = ZkListNote {
            id: Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?,
            title: row.get(2)?,
            is_file: {
              let wat: Option<i64> = row.get(3)?;
              wat.is_some()
            },
            user: row.get(4)?,
            createdate: row.get(5)?,
            changeddate: row.get(6)?,
            sysids: Vec::new(),
          };
          yield SyncMessage::from(zln)
        }
        ResultType::RtNote => {
          let zn = sqldata::read_zknote_i64(&conn, Some(user), row.get(0)?)?;
          yield SyncMessage::from(zn)
        }
        ResultType::RtNoteAndLinks => {
          // TODO: i64 version
          let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
          let zn = sqldata::read_zknoteandlinks(&conn, Some(user), &uuid)?;
          yield SyncMessage::from(zn)
        }
      }
    }
  }
}

pub fn sync_users(
  conn: Arc<Connection>,
  uid: i64,
  _after: Option<i64>,
  zkns: &ZkNoteSearch,
) -> impl futures_util::Stream<Item = Result<SyncMessage, Box<dyn std::error::Error>>> {
  let lzkns = zkns.clone();
  // {
  try_stream! {

    let (sql, args) = build_sql(&conn, uid, &lzkns, None)?;

    let mut pstmt = conn.prepare(
      format!(
        "with search_notes ( id, uuid, title, file, user, createdate, changeddate) as ({})
        select U.id, U.uuid, U.name, U.active
        from orgauth_user U where
          U.id in (select distinct user from search_notes)",
        sql,
      )
      .as_str(),
    )?;

    println!("read_zklinks_since_stream 2");

    yield SyncMessage::PhantomUserHeader;

    let rec_iter =
      pstmt.query_map(
        rusqlite::params_from_iter(args.iter()),
        |row| match Uuid::parse_str(row.get::<usize, String>(1)?.as_str()) {
          Ok(uuid) => Ok(ZkPhantomUser {
            id: row.get(0)?,
            uuid: uuid,
            data: serde_json::Value::Null,
            name: row.get(2)?,
            active: row.get(3)?,
          }),
          Err(_e) => Err(rusqlite::Error::InvalidColumnType(
            0,
            "uuid".to_string(),
            rusqlite::types::Type::Text,
          )),
        },
      )?;

    for rec in rec_iter {
      println!("sync user {:?}", rec);
      if let Ok(mut r) = rec {
        let ed = serde_json::to_value(sqldata::read_extra_login_data(&conn, r.id)?)?;
        r.data = ed;
        yield SyncMessage::from(r);
      }
    }
  }
}

pub fn system_user(
  conn: Arc<Connection>,
) -> impl futures_util::Stream<Item = Result<SyncMessage, Box<dyn std::error::Error>>> {
  try_stream! {

    let sysid = user_id(&conn, "system")?;

    yield SyncMessage::from(ZkPhantomUser {
      id: sysid,
      uuid: Uuid::parse_str(&SpecialUuids::System.str())?,
      name: "system".to_string(),
      data: serde_json::to_value(sqldata::read_extra_login_data(&conn, sysid)?)?,
      active: true,
    });
  }
}

pub fn build_sql(
  conn: &Connection,
  uid: i64,
  search: &ZkNoteSearch,
  exclude_notes: Option<String>,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let (sql, args) = build_base_sql(conn, uid, search)?;
  match exclude_notes {
    Some(exclude_note_table) => {
      let nusql = format!(
        "with SN ( id, uuid, title, file, user, createdate, changeddate) as ({})
        select SN.id, SN.uuid, SN.title, SN.file, SN.user, SN.createdate, SN.changeddate
        from SN
        left join {} as EN
        on SN.id = EN.id
        where EN.id is null",
        sql, exclude_note_table
      );
      Ok((nusql, args))
    }
    None => Ok((sql, args)),
  }
}

pub fn build_base_sql(
  conn: &Connection,
  uid: i64,
  search: &ZkNoteSearch,
) -> Result<(String, Vec<String>), zkerr::Error> {
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

  let ordclause = if let Some(o) = &search.ordering {
    let ord = match o.field {
      OrderField::Title => "order by N.title",
      OrderField::Created => "order by N.createdate",
      OrderField::Changed => "order by N.changeddate",
    };
    let dir = match o.direction {
      OrderDirection::Ascending => " asc",
      OrderDirection::Descending => " desc",
    };
    format!("{}{}", ord, dir)
  } else {
    " order by N.changeddate desc ".to_string()
  };

  let archives = search.archives;
  let deleted = if search.deleted {
    ""
  } else {
    "and N.deleted = 0"
  };

  let (mut sqlbase, mut baseargs) = if archives {
    (
      // archives of notes that are mine.
      format!(
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zknote O, zklink OL, zklink AL where
      (O.user = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
        {}",
        deleted
      ),
      vec![uid.to_string(), archiveid.to_string()],
    )
  } else {
    // notes that are mine.
    (
      format!(
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N where N.user = ?
        {}",
        deleted
      ),
      vec![uid.to_string()],
    )
  };

  // notes that are public, and not mine.
  let (mut sqlpub, mut pubargs) = if archives {
    (
      // archives of notes that are public, and not mine.
      format!(
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where (N.user != ?
        and L.fromid = N.id and L.toid = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
        {}",
        deleted
      ),
      vec![uid.to_string(), publicid.to_string(), archiveid.to_string()],
    )
  } else {
    (
      format!(
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)
      {}",
        deleted
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        {}",
        deleted
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ? 
        and M.toid = ? 
        and ((L.fromid = N.id and L.toid = M.fromid )
             or (L.toid = N.id and L.fromid = M.fromid ))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))
        {}",
        deleted,
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where N.user != ?
        and ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?))
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?
        {}",
        deleted
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (
        N.user != ? and
        ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?)))
        {}",
        deleted
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
      sql.push_str("\nand ");
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
  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqlpub.as_str());
  baseargs.append(&mut pubargs);

  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqlshare.as_str());
  baseargs.append(&mut shareargs);

  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqluser.as_str());
  baseargs.append(&mut userargs);

  // add order clause to the end.
  sqlbase.push_str(ordclause.as_str());

  // add limit clause to the end.
  sqlbase.push_str(limclause.as_str());

  // println!("sqlbase: {}", sqlbase);
  // println!("sqlargs: {:?}", baseargs);

  Ok((sqlbase, baseargs))
}

fn build_tagsearch_clause(
  conn: &Connection,
  uid: i64,
  not: bool,
  search: &TagSearch,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let (cls, args) = match search {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut zknoteid = false;
      let mut tag = false;
      let mut desc = false;
      let mut user = false;
      let mut file = false;
      let mut before = false;
      let mut after = false;
      let mut create = false;
      let mut modd = false;

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
          SearchMod::User => user = true,
          SearchMod::File => file = true,
          SearchMod::Before => before = true,
          SearchMod::After => after = true,
          SearchMod::Create => create = true,
          SearchMod::Mod => modd = true,
          SearchMod::ZkNoteId => {
            zknoteid = true;
            exact = true; // zknoteid implies exact.
          }
        }
      }
      let field = if zknoteid {
        "uuid"
      } else if desc {
        "content"
      } else {
        "title"
      };

      if create || modd {
        let op = if before {
          " < "
        } else if after {
          " > "
        } else {
          " = "
        };
        if create {
          (format!("N.createdate {} ?", op), vec![term.clone()])
        } else {
          (format!("N.changeddate {} ?", op), vec![term.clone()])
        }
      } else if user {
        let userid = user_id(conn, &term)?;
        let notstr = match not {
          true => "!",
          false => "",
        };
        (format!("N.user {}= ?", notstr), vec![format!("{}", userid)])
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
      if not {
        cls.push_str(" not ");
      }
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
