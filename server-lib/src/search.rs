use crate::error as zkerr;
use crate::sqldata;
use crate::sqldata::{delete_zknote, get_sysids, note_id};
use async_stream::try_stream;
use bytes::Bytes;
use futures::Stream;
use orgauth::data::PhantomUser;
use orgauth::dbfun::user_id;
use rusqlite::Connection;
use std::convert::TryInto;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use zkprotocol::constants::PrivateReplies;
use zkprotocol::content::{SyncMessage, ZkListNote, ZkPhantomUser};
use zkprotocol::messages::PrivateReplyMessage;
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
    created_after: None,
    created_before: None,
    changed_after: None,
    changed_before: None,
    synced_after: None,
    synced_before: None,
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
  let (sql, args) = build_sql(&conn, user, &search)?;

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
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + 'static>>> {
  // uncomment for formatting, lsp
  // {
  try_stream! {
    // let sysid = user_id(&conn, "system")?;
    let s_user = if search.archives {
      user_id(&conn, "system")?
    } else {
      user
    };
    let (sql, args) = build_sql(&conn, user, &search)?;

    let mut stmt = conn.prepare(sql.as_str())?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    let mut header = serde_json::to_value(SyncMessage::from(ZkSearchResultHeader {
      what: search.what,
      resultType: search.resulttype,
      offset: search.offset,
    }))?
    .to_string();

    header.push_str("\n");

    yield Bytes::from(header);

    while let Some(row) = rows.next()? {
      let title = row.get::<usize, String>(2)?;
      println!("zknote title {}", title);
      match search.resulttype {
        ResultType::RtId => {
          let mut s = serde_json::to_value(SyncMessage::ZkNoteId(row.get::<usize, String>(1)?))?
            .to_string()
            .to_string();
          s.push_str("\n");
          yield Bytes::from(s);
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

          let mut s = serde_json::to_value(SyncMessage::from(zln))?
            .to_string()
            .to_string();
          s.push_str("\n");
          yield Bytes::from(s);
        }
        ResultType::RtNote => {
          let zn = sqldata::read_zknote_i64(&conn, Some(s_user), row.get(0)?)?;
          let mut s = serde_json::to_value(SyncMessage::from(zn))?
            .to_string()
            .to_string();
          s.push_str("\n");
          yield Bytes::from(s);
        }
        ResultType::RtNoteAndLinks => {
          // TODO: i64 version
          let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
          let zn = sqldata::read_zknoteandlinks(&conn, Some(s_user), &uuid)?;
          let mut s = serde_json::to_value(SyncMessage::from(zn))?
            .to_string()
            .to_string();
          s.push_str("\n");
          yield Bytes::from(s);
        }
      }
    }
  }
}

pub fn sync_users(
  conn: Arc<Connection>,
  uid: i64,
  after: Option<i64>,
  zkns: &ZkNoteSearch,
) -> impl futures_util::Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> {
  let lzkns = zkns.clone();
  // {
  try_stream! {

    // println!("read_zklinks_since_stream");
    let (sql, args) = build_sql(&conn, uid, &lzkns)?;

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

    {
      let mut s = serde_json::to_value(SyncMessage::PhantomUserHeader)?.to_string();
      s.push_str("\n");
      yield Bytes::from(s);
    }

    let rec_iter =
      pstmt.query_map(
        rusqlite::params_from_iter(args.iter()),
        |row| match Uuid::parse_str(row.get::<usize, String>(1)?.as_str()) {
          Ok(uuid) => Ok(SyncMessage::from(ZkPhantomUser {
            id: row.get(0)?,
            uuid: uuid,
            name: row.get(2)?,
            active: row.get(3)?,
          })),
          Err(e) => Err(rusqlite::Error::InvalidColumnType(
            0,
            "uuid".to_string(),
            rusqlite::types::Type::Text,
          )),
        },
      )?;

    for rec in rec_iter {
      println!("sync user {:?}", rec);
      if let Ok(r) = rec {
        let mut s = serde_json::to_value(r)?.to_string();
        s.push_str("\n");
        yield Bytes::from(s);
      }
    }
  }
}

pub fn build_sql(
  conn: &Connection,
  uid: i64,
  search: &ZkNoteSearch,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let (mut cls, mut clsargs) = build_tagsearch_clause(&conn, uid, false, &search.tagsearch)?;

  let (dtcls, mut dtclsargs) = build_daterange_clause(&search)?;
  if !dtclsargs.is_empty() {
    cls.push_str("\nand (");
    cls.push_str(dtcls.as_str());
    cls.push_str(")");
    clsargs.append(&mut dtclsargs);
  }

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
      OrderField::Synced => "order by N.syncdate",
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

  let (mut sqlbase, mut baseargs) = if archives {
    (
      // archives of notes that are mine.
      format!(
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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
        "select N.id, N.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
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

fn build_daterange_clause(search: &ZkNoteSearch) -> Result<(String, Vec<String>), zkerr::Error> {
  let create_clawses = [
    search
      .created_after
      .map(|dt| ("N.createdate > ?", dt.to_string())),
    search
      .created_before
      .map(|dt| ("N.createdate < ?", dt.to_string())),
  ];
  let changed_clawses = [
    search
      .changed_after
      .map(|dt| ("N.changeddate > ?", dt.to_string())),
    search
      .changed_before
      .map(|dt| ("N.changeddate < ?", dt.to_string())),
  ];
  let sync_clawses = [
    search
      .synced_after
      .map(|dt| ("N.syncdate > ?", dt.to_string())),
    search
      .synced_before
      .map(|dt| ("N.syncdate < ?", dt.to_string())),
  ];
  let join = |clawses: Vec<Option<(&str, String)>>, conj| {
    let clause = clawses
      .iter()
      .filter_map(|pair| pair.as_ref().map(|(s, _)| s.to_string()))
      .collect::<Vec<String>>()
      .join(conj);
    let mut args: Vec<String> = clawses
      .iter()
      .filter_map(|pair| pair.as_ref().map(|(_, dt)| dt.clone()))
      .collect();
    (clause, args)
  };

  let (crcls, mut crargs) = join(create_clawses.to_vec(), " and ");
  let (changedcls, mut changedargs) = join(changed_clawses.to_vec(), " and ");
  let (synccls, mut syncargs) = join(sync_clawses.to_vec(), " and ");

  let clause: String = {
    let mut v: Vec<String> = Vec::new();
    if crcls != "" {
      v.push(crcls);
    }
    if changedcls != "" {
      v.push(changedcls);
    }
    if synccls != "" {
      v.push(synccls);
    }
    v.join(" or ")
  };
  let mut args: Vec<String> = Vec::new();
  args.append(&mut crargs);
  args.append(&mut changedargs);
  args.append(&mut syncargs);
  Ok((clause, args))
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

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
          SearchMod::User => user = true,
          SearchMod::File => file = true,
          SearchMod::ZkNoteId => zknoteid = true,
        }
      }
      let field = if zknoteid {
        "uuid"
      } else if desc {
        "content"
      } else {
        "title"
      };

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

          let clause = if exact || zknoteid {
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
            if exact || zknoteid {
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
            if exact || zknoteid {
              format!("N.{} {}= ? {}", field, notstr, fileclause)
            } else {
              format!("N.{} {} like ? {}", field, notstr, fileclause)
            },
            // args
            if exact || zknoteid {
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
