use crate::error as zkerr;
use crate::sqldata;
use crate::sqldata::local_server_id;
use crate::sqldata::server_id;
use crate::sqldata::uuid_for_note_id;
use crate::sqldata::{delete_zknote, get_sysids, note_id};
use async_stream::try_stream;
use futures::Stream;
use orgauth::data::UserId;
use orgauth::dbfun::user_id;
use rusqlite::Connection;
use serde_derive::Serialize;
use std::convert::TryInto;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use zkprotocol::constants::SpecialUuids;
use zkprotocol::content::FileStatus;
use zkprotocol::content::ZkListNote;
use zkprotocol::content::ZkNoteId;
use zkprotocol::search::{
  AndOr, ArchivesOrCurrent, OrderDirection, OrderField, ResultType, SearchMod, TagSearch,
  ZkIdSearchResult, ZkListNoteSearchResult, ZkNoteAndLinksSearchResult, ZkNoteSearch,
  ZkNoteSearchResult, ZkSearchResultHeader,
};
use zkprotocol::sync_data::{SyncMessage, ZkPhantomUser};

pub fn power_delete_zknotes(
  conn: &Connection,
  file_path: PathBuf,
  user: UserId,
  search: &Vec<TagSearch>,
) -> Result<i64, zkerr::Error> {
  // get all, and delete all.  Maybe not a good idea for a big database, but ours is small
  // and soon to be replaced with indradb, perhaps.

  let nolimsearch = ZkNoteSearch {
    tagsearch: search.clone(),
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtListNote,
    archives: ArchivesOrCurrent::Current,
    deleted: false,
    ordering: None,
  };
  let znsr = search_zknotes(conn, &file_path, user, &nolimsearch)?;
  match znsr {
    SearchResult::SrId(znsr) => {
      let c = znsr
        .notes
        .len()
        .try_into()
        .map_err(|_| zkerr::Error::String("int conversion error".to_string()))?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n)?;
      }
      Ok(c)
    }
    SearchResult::SrListNote(znsr) => {
      let c = znsr
        .notes
        .len()
        .try_into()
        .map_err(|_| zkerr::Error::String("int conversion error".to_string()))?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.id)?;
      }
      Ok(c)
    }
    SearchResult::SrNote(znsr) => {
      let c = znsr
        .notes
        .len()
        .try_into()
        .map_err(|_| zkerr::Error::String("int conversion error".to_string()))?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.id)?;
      }
      Ok(c)
    }
    SearchResult::SrNoteAndLink(znsr) => {
      let c = znsr
        .notes
        .len()
        .try_into()
        .map_err(|_| zkerr::Error::String("int conversion error".to_string()))?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, &n.zknote.id)?;
      }
      Ok(c)
    }
  }
}

#[derive(Serialize, Debug, Clone)]
pub enum SearchResult {
  SrId(ZkIdSearchResult),
  SrListNote(ZkListNoteSearchResult),
  SrNote(ZkNoteSearchResult),
  SrNoteAndLink(ZkNoteAndLinksSearchResult),
}

pub fn search_zknotes(
  conn: &Connection,
  filedir: &Path,
  user: UserId,
  search: &ZkNoteSearch,
) -> Result<SearchResult, zkerr::Error> {
  let (sql, args) = build_sql(&conn, user, &search, None)?;

  let mut pstmt = conn.prepare(sql.as_str())?;
  let sysid = user_id(&conn, "system")?;
  let rec_iter = pstmt.query_and_then(rusqlite::params_from_iter(args.iter()), |row| {
    let id = row.get(0)?;
    let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
    let pid = row
      .get::<usize, String>(2)
      .ok()
      .and_then(|x| Uuid::parse_str(x.as_str()).ok());
    let sysids = get_sysids(conn, sysid, id)?;
    Ok::<ZkListNote, zkerr::Error>(ZkListNote {
      id: match pid {
        Some(pid) => ZkNoteId::ArchiveZni(uuid, pid),
        None => ZkNoteId::Zni(uuid),
      },
      title: row.get(3)?,
      filestatus: {
        let wat: Option<i64> = row.get(4)?;

        match wat {
          Some(file_id) => {
            if sqldata::file_exists(&conn, filedir, file_id)? {
              FileStatus::FilePresent
            } else {
              FileStatus::FileMissing
            }
          }
          None => FileStatus::NotAFile,
        }
      },
      user: UserId::Uid(row.get(5)?),
      createdate: row.get(6)?,
      changeddate: row.get(7)?,
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
            pv.push(sqldata::read_zknote(&conn, filedir, Some(user), &rec.id)?.1);
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
            pv.push(sqldata::read_zknoteandlinks(
              &conn,
              filedir,
              Some(user),
              &rec.id,
            )?);
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
  files_dir: PathBuf,
  user: UserId,
  search: ZkNoteSearch,
  exclude_notes: Option<String>,
) -> impl Stream<Item = Result<SyncMessage, Box<dyn std::error::Error + 'static>>> {
  // uncomment for formatting, lsp
  // {
  try_stream! {
    let (sql, args) = build_sql(&conn, user, &search, exclude_notes)?;


    let mut stmt = conn.prepare(sql.as_str())?;
    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
    yield SyncMessage::from(ZkSearchResultHeader {
      what: search.what,
      resulttype: search.resulttype,
      offset: search.offset,
    });

    while let Some(row) = rows.next()? {
      match search.resulttype {
        ResultType::RtId => yield SyncMessage::ZkNoteId(row.get::<usize, String>(1)?),
        ResultType::RtListNote => {
          yield SyncMessage::SyncError("unimplemented".to_string())
        }
        ResultType::RtNote => {
          let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
          let parent : Option<Uuid>
           = row.get::<usize, String>(2).ok().and_then(
               |x| Uuid::parse_str(x.as_str()).ok());
          let (_id, zn) = match parent {
            None => sqldata::read_zknote(&conn, &files_dir,Some(user), &ZkNoteId::Zni(uuid))?,
            Some(pid) => sqldata::read_zknote(&conn, &files_dir,Some(user), &ZkNoteId::ArchiveZni(uuid, pid))?,
            };
          let mbf = if zn.filestatus != FileStatus::NotAFile {
            Some( sqldata::read_file_info(&conn, row.get(0)?)? )} else { None };
          yield SyncMessage::from((zn, mbf))
        }
        ResultType::RtNoteAndLinks => {
          yield SyncMessage::SyncError("unimplemented".to_string())
        }
      }
    }
  }
}

pub fn sync_users(
  conn: Arc<Connection>,
  uid: UserId,
  _after: Option<i64>,
  zkns: &ZkNoteSearch,
) -> impl futures_util::Stream<Item = Result<SyncMessage, Box<dyn std::error::Error>>> {
  let lzkns = zkns.clone();
  // {
  try_stream! {

    let (sql, args) = build_sql(&conn, uid, &lzkns, None)?;

    let mut pstmt = conn.prepare(
      format!(
        "with search_notes ( id, uuid, zknote, title, file, user, createdate, changeddate) as ({})
        select U.id, U.uuid, U.name, U.active
        from orgauth_user U where
          U.id in (select distinct user from search_notes)",
        sql,
      )
      .as_str(),
    )?;

    yield SyncMessage::PhantomUserHeader;

    let rec_iter =
      pstmt.query_map(
        rusqlite::params_from_iter(args.iter()),
        |row| match Uuid::parse_str(row.get::<usize, String>(1)?.as_str()) {
          Ok(uuid) => Ok(ZkPhantomUser {
            id: UserId::Uid(row.get(0)?),
            uuid: uuid,
            data: serde_json::Value::Null.to_string(),
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
      if let Ok(mut r) = rec {
        let ed = serde_json::to_value(sqldata::read_extra_login_data(&conn, r.id)?)?;
        r.data = ed.to_string();
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
      data: serde_json::to_value(sqldata::read_extra_login_data(&conn, sysid)?)?.to_string(),
      active: true,
    });
  }
}

pub fn build_sql(
  conn: &Connection,
  uid: UserId,
  search: &ZkNoteSearch,
  exclude_notes: Option<String>,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let (sql, args) = build_base_sql(conn, uid, search)?;
  match exclude_notes {
    Some(exclude_note_table) => {
      let nusql = format!(
        "with SN ( id, uuid, zknote, title, file, user, createdate, changeddate) as ({})
        select SN.id, SN.uuid, SN.zknote, SN.title, SN.file, SN.user, SN.createdate, SN.changeddate
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

pub fn andify_search(search: &Vec<TagSearch>) -> TagSearch {
  let mut it = search.iter();
  match it.next() {
    Some(head) => it.fold(head.clone(), |acc, elt| TagSearch::Boolex {
      ts1: Box::new(acc.clone()),
      ao: AndOr::And,
      ts2: Box::new(elt.clone()),
    }),
    None => TagSearch::SearchTerm {
      mods: Vec::new(),
      term: "".to_string(),
    },
  }
}

pub fn build_base_sql(
  conn: &Connection,
  uid: UserId,
  search: &ZkNoteSearch,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let ts = andify_search(&search.tagsearch);

  let publicid = note_id(&conn, "system", "public")?;
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

  let archives = (search.archives == ArchivesOrCurrent::Archives)
    || (search.archives == ArchivesOrCurrent::CurrentAndArchives);

  let current = (search.archives == ArchivesOrCurrent::Current)
    || (search.archives == ArchivesOrCurrent::CurrentAndArchives);

  let deleted = if search.deleted {
    ""
  } else {
    "and N.deleted = 0"
  };

  // sql, sqlargs, current (or archives = false)
  let mut sqlargs: Vec<(String, Vec<String>, bool)> = Vec::new();

  if archives {
    let (sqlbase, baseargs) = (
      // archives of notes that are mine.
      format!(
        "select N.id, N.uuid, PN.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zkarch N, zknote PN where N.user = ? and PN.id = N.zknote
        {}",
        deleted
      ),
      vec![uid.to_string()],
    );
    sqlargs.push((sqlbase, baseargs, false));
  }
  if current {
    let ( sqlbase,  baseargs) =

    // notes that are mine.
    (
      format!(
        "select N.id, N.uuid, null, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N where N.user = ?
        {}",
        deleted
      ),
      vec![uid.to_string()],
    );
    sqlargs.push((sqlbase, baseargs, true));
  };

  // notes that are public, and not mine.
  if archives {
    let (sqlpub, pubargs) = (
      // archives of notes that are public, and not mine.
      format!(
        "select N.id, N.uuid, PN.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zkarch N, zklink L, zknote PN
      where (N.user != ?
        and L.fromid = N.zknote and L.toid = ? )
        and PN.id = N.zknote
        {}",
        deleted
      ),
      vec![uid.to_string(), publicid.to_string()],
    );
    sqlargs.push((sqlpub, pubargs, false));
  }
  if current {
    let (sqlpub, pubargs) = (
      format!(
        "select N.id, N.uuid, null, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)
      {}",
        deleted
      ),
      vec![uid.to_string(), publicid.to_string()],
    );
    sqlargs.push((sqlpub, pubargs, true));
  };

  // notes shared with a share tag, and not mine.
  // clause 1: user is not-me
  //
  // clause 2: is O (original (non-archived) note) linked to a share note?
  // link M is to shareid, and L links either to or from M's from.
  //
  // clause 3 is M.from (the share)
  // is that share linked to usernoteid?
  if archives {
    let (sqlshare, shareargs) = (
      format!(
        "select N.id, N.uuid, PN.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zkarch N, zklink L, zklink M, zklink U, zknote PN
      where N.user != ?
      and
        (M.toid = ? and
          ((L.fromid = N.zknote and L.toid = M.fromid ) or
           (L.toid = N.zknote and L.fromid = M.fromid )))
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?))
        and PN.id = N.zknote
        {}",
        deleted
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    );
    sqlargs.push((sqlshare, shareargs, false));
  }
  if current {
    let (sqlshare, shareargs) = (
      format!(
        "select N.id, N.uuid, null, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ?
        and M.toid = ?
        and ((L.fromid = N.id and L.toid = M.fromid )
             or (L.toid = N.id and L.fromid = M.fromid ))
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))
        {}",
        deleted,
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    );
    sqlargs.push((sqlshare, shareargs, true));
  };

  // notes that are tagged with my usernoteid, and not mine.
  if archives {
    let (sqluser, userargs) = (
      format!(
        "select N.id, N.uuid, PN.uuid, N.title, N.file, N.user, N.createdate, N.changeddate
      from zkarch N, zklink L, zknote PN
      where N.user != ?
        and ((L.fromid = N.zknote and L.toid = ?) or (L.toid = N.zknote and L.fromid = ?))
        and PN.id = N.zknote
        {}",
        deleted
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    );
    sqlargs.push((sqluser, userargs, false));
  }
  if current {
    let (sqluser, userargs) = (
      format!(
        "select N.id, N.uuid, null, N.title, N.file, N.user, N.createdate, N.changeddate
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
    );
    sqlargs.push((sqluser, userargs, true));
  };

  struct Tsc {
    cls: String,
    clsargs: Vec<String>,
  }

  let cur_tsc: Option<Tsc> = if current {
    let (cls, clsargs) =
      build_tagsearch_clause(&conn, uid, &ArchivesOrCurrent::Current, false, &ts)?;
    Some(Tsc {
      cls: cls,
      clsargs: clsargs,
    })
  } else {
    None
  };
  let arch_tsc: Option<Tsc> = if archives {
    let (cls, clsargs) =
      build_tagsearch_clause(&conn, uid, &ArchivesOrCurrent::Archives, false, &ts)?;
    Some(Tsc {
      cls: cls,
      clsargs: clsargs,
    })
  } else {
    None
  };

  // let (cls, clsargs) = build_tagsearch_clause(&conn, uid, &search.archives, false, &ts)?;

  // local ftn to add clause and args.
  let addcls = |sql: &mut String, args: &mut Vec<String>, current: bool| {
    if !args.is_empty() {
      let tsc: Option<&Tsc> = if current {
        match &cur_tsc {
          Some(tsc) => Some(&tsc),
          None => None,
        }
      } else {
        match &arch_tsc {
          Some(tsc) => Some(&tsc),
          None => None,
        }
      };

      if let Some(&ref tsc) = tsc {
        sql.push_str("\nand ");
        sql.push_str(tsc.cls.as_str());

        // clone, otherwise no clause vals next time!
        let mut pendargs = tsc.clsargs.clone();
        args.append(&mut pendargs);
      } else {
        tracing::error!("tsc error");
      };
    }
  };

  let mut rsql: String = String::new();
  let mut rargs: Vec<String> = Vec::new();

  while let Some((mut sql, mut args, current)) = sqlargs.pop() {
    addcls(&mut sql, &mut args, current);

    rsql.push_str(sql.as_str());
    if sqlargs.len() > 0 {
      rsql.push_str("\nunion ");
    }

    rargs.append(&mut args);
  }

  /*

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

  */

  // add order clause to the end.
  rsql.push_str(ordclause.as_str());

  // add limit clause to the end.
  rsql.push_str(limclause.as_str());

  Ok((rsql, rargs))
}

fn build_tagsearch_clause(
  conn: &Connection,
  uid: UserId,
  aoc: &ArchivesOrCurrent,
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
      let mut server = false;

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
          SearchMod::Server => server = true,
        }
      }
      let field = if zknoteid {
        "uuid"
      } else if desc {
        "content"
      } else if server {
        "server"
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
      } else if server {
        let serverid = if term == "local" {
          local_server_id(conn)?.id
        } else {
          server_id(conn, &term)?
        };
        let notstr = match not {
          true => "!",
          false => "",
        };
        (
          format!("N.server {}= ?", notstr),
          vec![format!("{}", serverid)],
        )
      } else {
        if tag {
          let fileclause = if file { "and zkn.file is not null" } else { "" };

          let clause = if exact {
            format!("zkn.{} = ? {}", field, fileclause)
          } else {
            format!("zkn.{}  like ? {}", field, fileclause)
          };

          let notstr = if not { "not" } else { "" };

          enum Aocid {
            AocS(&'static str),
            Both,
          }

          let ai = match aoc {
            ArchivesOrCurrent::Current => Aocid::AocS("id"),
            ArchivesOrCurrent::Archives => Aocid::AocS("zknote"),
            ArchivesOrCurrent::CurrentAndArchives => Aocid::Both,
          };
          match ai {
            Aocid::AocS(nid) => (
              // clause
              format!(
                "{} (N.{} in (select zklink.toid from zknote as zkn, zklink
                 where zkn.id = zklink.fromid
                   and {})
                or
                    N.{} in (select zklink.fromid from zknote as zkn, zklink
                 where zkn.id = zklink.toid
                   and {}))",
                notstr, nid, clause, nid, clause
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
            ),
            Aocid::Both => (
              // clause
              format!(
                "{} (N.id in (select zklink.toid from zknote as zkn, zklink
                 where zkn.id = zklink.fromid
                   and {})
                or
                    N.id in (select zklink.fromid from zknote as zkn, zklink
                 where zkn.id = zklink.toid
                   and {})
                or
                    N.zknote in (select zklink.fromid from zknote as zkn, zklink
                 where zkn.id = zklink.toid
                   and {})
                or
                    N.zknote in (select zklink.fromid from zknote as zkn, zklink
                 where zkn.id = zklink.toid
                   and {})
                   )",
                notstr, clause, clause, clause, clause
              ),
              // args
              if exact {
                vec![term.clone(), term.clone(), term.clone(), term.clone()]
              } else {
                vec![
                  format!("%{}%", term).to_string(),
                  format!("%{}%", term).to_string(),
                  format!("%{}%", term).to_string(),
                  format!("%{}%", term).to_string(),
                ]
              },
            ),
          }
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
    TagSearch::Not { ts } => build_tagsearch_clause(&conn, uid, aoc, true, &*ts)?,
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (cl1, mut arg1) = build_tagsearch_clause(&conn, uid, aoc, false, &*ts1)?;
      let (cl2, mut arg2) = build_tagsearch_clause(&conn, uid, aoc, false, &*ts2)?;
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
