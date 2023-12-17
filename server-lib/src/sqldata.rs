use crate::migrations as zkm;
use async_stream::try_stream;
use barrel::backend::Sqlite;
use bytes::Bytes;
use futures::Stream;
use log::info;
use orgauth::data::RegistrationData;
use orgauth::dbfun::user_id;
use orgauth::util::now;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use zkprotocol::constants::PrivateReplies;
use zkprotocol::content::{
  ArchiveZkLink, Direction, EditLink, ExtraLoginData, GetArchiveZkNote, GetZkLinks,
  GetZkNoteArchives, GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkLink, SaveZkNote,
  SavedZkNote, Sysids, UuidZkLink, ZkLink, ZkListNote, ZkNote, ZkNoteAndLinks,
};
use zkprotocol::messages::PrivateReplyMessage;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub noteid: i64,
  pub homenoteid: Option<i64>,
}

pub fn on_new_user(
  conn: &Connection,
  rd: &RegistrationData,
  data: Option<String>,
  creator: Option<i64>,
  uid: i64,
) -> Result<(), orgauth::error::Error> {
  let usernoteid = note_id(&conn, "system", "user")?;
  let publicnoteid = note_id(&conn, "system", "public")?;
  let systemid = user_id(&conn, "system")?;

  let now = now()?;

  // make a corresponding note,
  conn.execute(
    "insert into zknote (title, content, user, editable, showtitle, deleted, uuid, createdate, changeddate)
     values (?1, ?2, ?3, 0, 1, 0, ?4, ?5, ?6)",
    params![rd.uid, "", systemid, uuid::Uuid::new_v4().to_string(), now, now],
  )?;

  let zknid = conn.last_insert_rowid();

  // make a user record.
  conn.execute(
    "insert into user (id, zknote)
      values (?1, ?2)",
    params![uid, zknid],
  )?;

  conn.execute(
    "update zknote set sysdata = ?1
        where id = ?2",
    params![systemid, uid.to_string().as_str()],
  )?;

  // indicate a 'user' record, and 'public'
  save_zklink(&conn, zknid, usernoteid, systemid, None)?;
  save_zklink(&conn, zknid, publicnoteid, systemid, None)?;

  // add extra links from 'data'
  match (&data, creator) {
    (Some(data), Some(creator)) => {
      let extra_links: Vec<SaveZkLink> = serde_json::from_str(data.as_str())?;
      save_savezklinks(&conn, creator, zknid, extra_links)?;
    }
    _ => (),
  }

  Ok(())
}

// callback to pass to orgauth
pub fn extra_login_data_callback(
  conn: &Connection,
  uid: i64,
) -> Result<Option<serde_json::Value>, orgauth::error::Error> {
  Ok(Some(serde_json::to_value(extra_login_data(&conn, uid)?)?))
}

// for-real delete of user - no archives?
pub fn on_delete_user(conn: &Connection, uid: i64) -> Result<bool, orgauth::error::Error> {
  // try deleting all their links and notes.
  // TODO: delete archive notes that have system ownership.
  conn.execute("delete from zklinkarchive where user = ?1", params!(uid))?;
  conn.execute("delete from zklink where user = ?1", params!(uid))?;
  conn.execute("delete from zknote where user = ?1", params!(uid))?;
  conn.execute("delete from user where id = ?1", params!(uid))?;
  Ok(true)
}

pub fn extra_login_data(
  conn: &Connection,
  uid: i64,
) -> Result<ExtraLoginData, orgauth::error::Error> {
  let user = read_user_by_id(&conn, uid)?;

  let eld = ExtraLoginData {
    userid: uid,
    zknote: user.noteid,
    homenote: user.homenoteid,
  };

  Ok(eld)
}

pub fn read_sysids(conn: &Connection) -> Result<Sysids, orgauth::error::Error> {
  Ok(Sysids {
    publicid: note_id(conn, "system", "public")?,
    shareid: note_id(conn, "system", "share")?,
    searchid: note_id(conn, "system", "search")?,
    commentid: note_id(conn, "system", "comment")?,
  })
}

pub fn update_user(conn: &Connection, user: &User) -> Result<(), orgauth::error::Error> {
  conn.execute(
    "update user set zknote = ?1, homenote = ?2
           where id = ?3",
    params![user.noteid, user.homenoteid, user.id,],
  )?;

  Ok(())
}

pub fn connection_open(dbfile: &Path) -> Result<Connection, orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;

  // conn.busy_timeout(Duration::from_millis(500))?;
  conn.busy_handler(Some(|count| {
    info!("busy_handler: {}", count);
    let d = Duration::from_millis(500);
    std::thread::sleep(d);
    true
  }))?;

  conn.execute("PRAGMA foreign_keys = true;", params![])?;

  Ok(conn)
}

pub fn get_single_value(
  conn: &Connection,
  name: &str,
) -> Result<Option<String>, orgauth::error::Error> {
  match conn.query_row(
    "select value from singlevalue where name = ?1",
    params![name],
    |row| Ok(row.get(0)?),
  ) {
    Ok(v) => Ok(Some(v)),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
    Err(x) => Err(x.into()),
  }
}

pub fn set_single_value(
  conn: &Connection,
  name: &str,
  value: &str,
) -> Result<(), orgauth::error::Error> {
  conn.execute(
    "insert into singlevalue (name, value) values (?1, ?2)
        on conflict (name) do update set value = ?2 where name = ?1",
    params![name, value],
  )?;
  Ok(())
}

pub fn dbinit(
  dbfile: &Path,
  token_expiration_ms: Option<i64>,
) -> Result<(), orgauth::error::Error> {
  let exists = dbfile.exists();

  let conn = connection_open(dbfile)?;

  if !exists {
    info!("initialdb");
    conn.execute_batch(zkm::initialdb().make::<Sqlite>().as_str())?;
  }

  let nlevel = match get_single_value(&conn, "migration_level") {
    Err(_) => 0,
    Ok(None) => 0,
    Ok(Some(level)) => {
      let l = match level.parse::<i32>() {
        Ok(l) => l,
        Err(e) => return Err(format!("{}", e).into()),
      };
      l
    }
  };

  if nlevel < 1 {
    info!("udpate1");
    conn.execute_batch(zkm::udpate1().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "1")?;
  }

  if nlevel < 2 {
    info!("udpate2");
    conn.execute_batch(zkm::udpate2().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "2")?;
  }
  if nlevel < 3 {
    info!("udpate3");
    zkm::udpate3(&dbfile)?;
    set_single_value(&conn, "migration_level", "3")?;
  }
  if nlevel < 4 {
    info!("udpate4");
    zkm::udpate4(&dbfile)?;
    set_single_value(&conn, "migration_level", "4")?;
  }
  if nlevel < 5 {
    info!("udpate5");
    zkm::udpate5(&dbfile)?;
    set_single_value(&conn, "migration_level", "5")?;
  }
  if nlevel < 6 {
    info!("udpate6");
    zkm::udpate6(&dbfile)?;
    set_single_value(&conn, "migration_level", "6")?;
  }
  if nlevel < 7 {
    info!("udpate7");
    zkm::udpate7(&dbfile)?;
    set_single_value(&conn, "migration_level", "7")?;
  }
  if nlevel < 8 {
    info!("udpate8");
    zkm::udpate8(&dbfile)?;
    set_single_value(&conn, "migration_level", "8")?;
  }
  if nlevel < 9 {
    info!("udpate9");
    zkm::udpate9(&dbfile)?;
    set_single_value(&conn, "migration_level", "9")?;
  }
  if nlevel < 10 {
    info!("udpate10");
    zkm::udpate10(&dbfile)?;
    set_single_value(&conn, "migration_level", "10")?;
  }
  if nlevel < 11 {
    info!("udpate11");
    zkm::udpate11(&dbfile)?;
    set_single_value(&conn, "migration_level", "11")?;
  }
  if nlevel < 12 {
    info!("udpate12");
    zkm::udpate12(&dbfile)?;
    set_single_value(&conn, "migration_level", "12")?;
  }
  if nlevel < 13 {
    info!("udpate13");
    zkm::udpate13(&dbfile)?;
    set_single_value(&conn, "migration_level", "13")?;
  }
  if nlevel < 14 {
    info!("udpate14");
    zkm::udpate14(&dbfile)?;
    set_single_value(&conn, "migration_level", "14")?;
  }
  if nlevel < 15 {
    info!("udpate15");
    zkm::udpate15(&dbfile)?;
    set_single_value(&conn, "migration_level", "15")?;
  }
  if nlevel < 16 {
    info!("udpate16");
    zkm::udpate16(&dbfile)?;
    set_single_value(&conn, "migration_level", "16")?;
  }
  if nlevel < 17 {
    info!("udpate17");
    zkm::udpate17(&dbfile)?;
    set_single_value(&conn, "migration_level", "17")?;
  }
  if nlevel < 18 {
    info!("udpate18");
    zkm::udpate18(&dbfile)?;
    set_single_value(&conn, "migration_level", "18")?;
  }
  if nlevel < 19 {
    info!("udpate19");
    zkm::udpate19(&dbfile)?;
    set_single_value(&conn, "migration_level", "19")?;
  }
  if nlevel < 20 {
    info!("udpate20");
    zkm::udpate20(&dbfile)?;
    set_single_value(&conn, "migration_level", "20")?;
  }
  if nlevel < 21 {
    info!("udpate21");
    zkm::udpate21(&dbfile)?;
    set_single_value(&conn, "migration_level", "21")?;
  }
  if nlevel < 22 {
    info!("udpate22");
    zkm::udpate22(&dbfile)?;
    set_single_value(&conn, "migration_level", "22")?;
  }
  if nlevel < 23 {
    info!("udpate23");
    zkm::udpate23(&dbfile)?;
    set_single_value(&conn, "migration_level", "23")?;
  }
  if nlevel < 24 {
    info!("udpate24");
    zkm::udpate24(&dbfile)?;
    set_single_value(&conn, "migration_level", "24")?;
  }
  if nlevel < 25 {
    info!("udpate25");
    zkm::udpate25(&dbfile)?;
    set_single_value(&conn, "migration_level", "25")?;
  }
  if nlevel < 26 {
    info!("udpate26");
    zkm::udpate26(&dbfile)?;
    set_single_value(&conn, "migration_level", "26")?;
  }
  if nlevel < 27 {
    info!("udpate27");
    zkm::udpate27(&dbfile)?;
    set_single_value(&conn, "migration_level", "27")?;
  }
  if nlevel < 28 {
    info!("udpate28");
    zkm::udpate28(&dbfile)?;
    set_single_value(&conn, "migration_level", "28")?;
  }
  if nlevel < 29 {
    info!("udpate29");
    zkm::udpate29(&dbfile)?;
    set_single_value(&conn, "migration_level", "29")?;
  }
  if nlevel < 30 {
    info!("udpate30");
    zkm::udpate30(&dbfile)?;
    set_single_value(&conn, "migration_level", "30")?;
  }

  info!("db up to date.");

  if let Some(expms) = token_expiration_ms {
    orgauth::dbfun::purge_login_tokens(&conn, expms)?;
  }

  Ok(())
}

// user CRUD

pub fn save_zklink(
  conn: &Connection,
  fromid: i64,
  toid: i64,
  user: i64,
  linkzknote: Option<i64>,
) -> Result<i64, orgauth::error::Error> {
  // ok to link to notes you don't own.
  // can link between notes you don't own, even.
  // but linking from a note you don't own to 'share' or 'public' is not allowed.
  // if one note is a share, then you must be a member of that share to link, or own that share.
  // or, if the link is owned by 'system' its ok.

  let shareid = note_id(&conn, "system", "share")?;
  let publicid = note_id(&conn, "system", "public")?;
  let systemid = user_id(&conn, "system")?;
  let usernote = user_note_id(&conn, user)?;

  let authed = if fromid == shareid || fromid == publicid || fromid == usernote {
    // can't link non-me notes to shareid or public or usernote.
    let izm = is_zknote_mine(&conn, toid, user)?;
    izm
  } else if toid == shareid || toid == publicid || toid == usernote {
    // can't link non-me notes to shareid or public or usernote.
    let izm = is_zknote_mine(&conn, fromid, user)?;
    izm
  } else if are_notes_linked(&conn, fromid, shareid)? && !are_notes_linked(&conn, usernote, fromid)?
  {
    // fromid is a share.
    // user does not link to it.
    // not allowed!
    is_zknote_mine(&conn, fromid, user)? // unless user owns fromid.
  } else if are_notes_linked(&conn, toid, shareid)? && !are_notes_linked(&conn, usernote, toid)? {
    // toid is a share.
    // user does not link to it.
    // not allowed!
    let izm = is_zknote_mine(&conn, toid, user)?; // unless user owns toid.
    izm || (user == systemid) // OR user is system.  for links from archive notes to share notes.
  } else {
    true
  };

  // yeesh.  doing this to exit with ? instead of having a big if-then to the end.
  let orwat: Result<(), orgauth::error::Error> = if authed {
    Ok(())
  } else {
    Err("link not allowed".into())
  };
  let _wat = orwat?;

  let now = now()?;

  // if there's already a record that is NOT like this record, make an archive record.
  // if the record is just like what we have now, nothing happens.
  conn.execute(
    "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
      select fromid, toid, user, linkzknote, createdate, ?1 from zklink
      where fromid = ?2 and toid = ?3 and user = ?4 and linkzknote <> ?5",
    params![now, fromid, toid, user, linkzknote],
  )?;

  // now create the new record or modify the existing.
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote, createdate) values (?1, ?2, ?3, ?4, ?5)
      on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
    params![fromid, toid, user, linkzknote, now],
  )?;

  Ok(conn.last_insert_rowid())
}

pub fn note_id(conn: &Connection, name: &str, title: &str) -> Result<i64, orgauth::error::Error> {
  let id: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params![name, title],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn note_id2(
  conn: &Connection,
  uid: i64,
  title: &str,
) -> Result<Option<i64>, orgauth::error::Error> {
  match conn.query_row(
    "select zknote.id from
      zknote
      where zknote.title = ?2
      and zknote.user = ?1",
    params![uid, title],
    |row| Ok(row.get(0)?),
  ) {
    Err(e) => match e {
      rusqlite::Error::QueryReturnedNoRows => Ok(None),
      _ => Err(e.into()),
    },
    Ok(i) => Ok(i),
  }
}

pub fn user_note_id(conn: &Connection, uid: i64) -> Result<i64, orgauth::error::Error> {
  let id: i64 = conn.query_row(
    "select zknote from user
      where user.id = ?1",
    params![uid],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn user_shares(conn: &Connection, uid: i64) -> Result<Vec<i64>, orgauth::error::Error> {
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = user_note_id(&conn, uid)?;

  // user shares!
  //   looking for notes that link to 'shareid' and link to 'usernoteid'
  let mut pstmt = conn.prepare(
    "select A.fromid from zklink A, zklink B
      where A.toid = ?1 and
        ((A.fromid = B.fromid and B.toid = ?2) or
         (A.fromid = B.toid and B.fromid = ?2))
     union
    select A.toid from zklink A, zklink B
      where A.fromid = ?1 and
        ((A.toid = B.fromid and B.toid = ?2) or
         (A.toid = B.toid and B.fromid = ?2))
      ",
  )?;
  let r = pstmt.query_map(params![shareid, usernoteid], |row| Ok(row.get(0)?))?;
  Ok(r.collect::<Result<Vec<i64>, rusqlite::Error>>()?)
}

// is there a connection between this note and uid's user note?
pub fn is_zknote_usershared(
  conn: &Connection,
  zknoteid: i64,
  uid: i64,
) -> Result<bool, orgauth::error::Error> {
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  let ret = are_notes_linked(&conn, zknoteid, usernoteid)?;

  Ok(ret)
}

pub fn is_zknote_shared(
  conn: &Connection,
  zknoteid: i64,
  uid: i64,
) -> Result<bool, orgauth::error::Error> {
  let shareid: i64 = note_id(conn, "system", "share")?;
  let publicid: i64 = note_id(conn, "system", "public")?;
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  // does note link to a note that links to share?
  // and does that note link to usernoteid?
  let ret: Result<bool, orgauth::error::Error> = match conn.query_row(
    "select count(*)
      from zklink L,
        (select U.toid id
            from zklink U, zklink V
            where
              ((U.fromid = ?3 and U.toid = V.fromid and V.toid = ?2) or
               (U.fromid = ?3 and U.toid = V.toid and V.fromid = ?2)) and
              U.toid != ?4
         union
         select U.fromid id
            from zklink U, zklink V
            where
              ((U.toid = ?3 and U.fromid = V.fromid and V.toid = ?2) or
               (U.toid = ?3 and U.fromid = V.toid and V.fromid = ?2)) and
              U.toid != ?4 ) shares
      where
        (L.fromid = shares.id and L.toid = ?1) or
        (L.toid = shares.id and L.fromid = ?1)",
    params![zknoteid, shareid, usernoteid, publicid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(x.into()),
  };

  Ok(ret?)
}

pub fn is_zknote_public(conn: &Connection, zknoteid: i64) -> Result<bool, orgauth::error::Error> {
  let pubid: i64 = note_id(conn, "system", "public")?;
  match conn.query_row(
    "select count(*) from
      zklink, zknote R
      where (zklink.fromid = ?1 and zklink.toid = ?2)
      or (zklink.fromid = ?2  and zklink.toid = ?1)",
    params![zknoteid, pubid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(x.into()),
  }
}

pub fn is_zknote_mine(
  conn: &Connection,
  zknoteid: i64,
  userid: i64,
) -> Result<bool, orgauth::error::Error> {
  match conn.query_row(
    "select count(*) from
      zknote
      where id = ?1 and user = ?2",
    params![zknoteid, userid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(x.into()),
  }
}

pub fn are_notes_linked(
  conn: &Connection,
  nid1: i64,
  nid2: i64,
) -> Result<bool, orgauth::error::Error> {
  match conn.query_row(
    "select count(*) from
      zklink
      where (fromid = ?1 and toid = ?2)
      or (toid = ?1 and fromid = ?2)",
    params![nid1, nid2],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(x.into()),
  }
}

// zknote CRUD

pub fn archive_zknote(
  conn: &Connection,
  noteid: i64,
) -> Result<SavedZkNote, orgauth::error::Error> {
  let now = now()?;
  let sysid = user_id(&conn, "system")?;
  let aid = note_id(&conn, "system", "archive")?;

  // copy the note, with user 'system'.
  // exclude pubid, to avoid unique constraint problems.
  conn.execute(
    "insert into zknote (title, content, user, editable, showtitle, deleted, uuid, createdate, changeddate)
     select title, content, ?1, editable, showtitle, deleted, ?2, createdate, changeddate from
         zknote where id = ?3",
    params![sysid, uuid::Uuid::new_v4().to_string(), noteid],
  )?;
  let archive_note_id = conn.last_insert_rowid();

  // mark the note as an archive note.
  save_zklink(&conn, archive_note_id, aid, sysid, None)?;

  // link the note to the original note, AND indicate this is an archive link.
  save_zklink(&conn, archive_note_id, noteid, sysid, Some(aid))?;

  Ok(SavedZkNote {
    id: archive_note_id,
    changeddate: now,
  })
}

pub fn set_zknote_file(
  conn: &Connection,
  noteid: i64,
  fileid: i64,
) -> Result<(), orgauth::error::Error> {
  conn.execute(
    "update zknote set file = ?1
         where id = ?2",
    params![fileid, noteid],
  )?;
  Ok(())
}

pub fn save_zknote(
  conn: &Connection,
  uid: i64,
  note: &SaveZkNote,
) -> Result<SavedZkNote, orgauth::error::Error> {
  let now = now()?;

  match note.id {
    Some(id) => {
      archive_zknote(&conn, id)?;
      // existing note.  update IF mine.
      match conn.execute(
        "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4, editable = ?5, showtitle = ?6, deleted = ?7
         where id = ?8 and user = ?9 and deleted = 0",
        params![
          note.title,
          note.content,
          now,
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          note.id,
          uid
        ],
      ) {
        Ok(1) => {
          Ok(SavedZkNote {
          id: id,
          changeddate: now,
        })}
        Ok(0) => {
          match zknote_access_id(conn, Some(uid), id)? {
            Access::ReadWrite => {
              // update other user's record!  editable flag must be true.  can't modify delete flag.
              match conn.execute(
                "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4, showtitle = ?5
                 where id = ?6 and editable = 1 and deleted = 0",
                params![note.title, note.content, now, note.pubid, note.showtitle, id],
              )? {
                0 => bail!("can't update; note is not writable"),
                1 => Ok(SavedZkNote {
                    id: id,
                    changeddate: now,
                  }),
                _ => bail!("unexpected update success!"),
              }
            }
            _ => bail!("can't update; note is not writable"),
          }
        }
        Ok(_) => bail!("unexpected update success!"),
        Err(e) => Err(e)?,
      }
    }
    None => {
      // new note!
      conn.execute(
        "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate)
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
          note.title,
          note.content,
          uid,
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          uuid::Uuid::new_v4().to_string(),
          now,
          now
        ],
      )?;
      Ok(SavedZkNote {
        id: conn.last_insert_rowid(),
        changeddate: now,
      })
    }
  }
}

pub fn get_sysids(conn: &Connection, sysid: i64, noteid: i64) -> Result<Vec<i64>, rusqlite::Error> {
  let mut pstmt = conn.prepare(
    // return system notes that are linked TO by noteid.
    "select A.toid
       from zklink A, zknote N
      where
       (A.fromid = ?1 and A.toid = N.id and N.user = ?2)",
  )?;

  let r = Ok(
    pstmt
      .query_map(params![noteid, sysid], |row| Ok(row.get(0)?))?
      .filter_map(|x| x.ok())
      .collect(),
  );

  r
}

pub fn read_user_by_id(conn: &Connection, id: i64) -> Result<User, orgauth::error::Error> {
  let user = conn.query_row(
    "select id, zknote, homenote
      from user where id = ?1",
    params![id],
    |row| {
      Ok(User {
        id: row.get(0)?,
        noteid: row.get(1)?,
        homenoteid: row.get(2)?,
      })
    },
  )?;

  Ok(user)
}

pub fn read_zknote(
  conn: &Connection,
  uid: Option<i64>,
  id: i64,
) -> Result<ZkNote, orgauth::error::Error> {
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  let mut note = conn.query_row(
    "select ZN.uuid, ZN.title, ZN.content, ZN.user, OU.name, U.zknote, ZN.pubid, ZN.editable, ZN.showtitle, ZN.deleted, ZN.file, ZN.createdate, ZN.changeddate
      from zknote ZN, orgauth_user OU, user U where ZN.id = ?1 and U.id = ZN.user and OU.id = ZN.user",
    params![id],
    |row| {
      Ok(ZkNote {
        id: id,
        uuid: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        user: row.get(3)?,
        username: row.get(4)?,
        usernote: row.get(5)?,
        pubid: row.get(6)?,
        editable: row.get(7)?,                // editable same as editableValue!
        editableValue: row.get(7)?,           // <--- same index.
        showtitle: row.get(8)?,
        deleted: row.get(9)?,
        is_file: { let wat : Option<i64> = row.get(10)?;
          match wat {
          Some(_) => true,
          None => false,
        }},
        createdate: row.get(11)?,
        changeddate: row.get(12)?,
        sysids: sysids,
      })
    },
  )?;

  match zknote_access(conn, uid, &note) {
    Ok(zna) => match zna {
      Access::ReadWrite => {
        note.editable = true;
        Ok(note)
      }
      Access::Read => {
        note.editable = false;
        Ok(note)
      }
      Access::Private => Err("can't read zknote; note is private".into()),
      // Access::Private => Err(Box::new(std::io::Error::new(
      //   std::io::ErrorKind::PermissionDenied,
      //   "can't read zknote; note is private",
      // ))),
    },
    Err(e) => Err(e),
  }
}

pub fn read_zklistnote(
  conn: &Connection,
  uid: Option<i64>,
  id: i64,
) -> Result<ZkListNote, orgauth::error::Error> {
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  // access check
  let zna = zknote_access_id(conn, uid, id)?;
  match zna {
    // Access::Private => Err(Box::new(std::io::Error::new(
    //   std::io::ErrorKind::PermissionDenied,
    //   "can't read zknote; note is private",
    // ))),
    Access::Private => Err::<_, orgauth::error::Error>("can't read zknote; note is private".into()),
    _ => Ok(()),
  }?;

  let note = conn.query_row(
    "select ZN.title, ZN.file, ZN.user, ZN.createdate, ZN.changeddate
      from zknote ZN, orgauth_user OU, user U where ZN.id = ?1 and U.id = ZN.user and OU.id = ZN.user",
    params![id],
    |row| {
      let wat : Option<i64> = row.get(1)?;
      let zln = ZkListNote {
        id: id,
        title: row.get(0)?,
        is_file: wat.is_some(),
        user: row.get(2)?,
        createdate: row.get(3)?,
        changeddate: row.get(4)?,
        sysids: sysids,
      };
      Ok(zln)
    },
  )?;

  Ok(note)
}

#[derive(Debug, PartialEq)]
pub enum Access {
  Private,
  Read,
  ReadWrite,
}

pub fn zknote_access(
  conn: &Connection,
  uid: Option<i64>,
  note: &ZkNote,
) -> Result<Access, orgauth::error::Error> {
  match uid {
    Some(uid) => {
      if uid == note.user {
        Ok(Access::ReadWrite)
      } else if is_zknote_usershared(conn, note.id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_shared(conn, note.id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_public(conn, note.id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
    None => {
      if is_zknote_public(conn, note.id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
  }
}

pub fn zknote_access_id(
  conn: &Connection,
  uid: Option<i64>,
  noteid: i64,
) -> Result<Access, orgauth::error::Error> {
  match uid {
    Some(uid) => {
      if is_zknote_mine(&conn, noteid, uid)? {
        Ok(Access::ReadWrite)
      } else if is_zknote_usershared(conn, noteid, uid)? {
        // editable and accessible.
        Ok(Access::ReadWrite)
      } else if is_zknote_shared(conn, noteid, uid)? {
        // editable and accessible.
        Ok(Access::ReadWrite)
      } else if is_zknote_public(conn, noteid)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
    None => {
      if is_zknote_public(conn, noteid)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
  }
}

pub fn read_zknote_filehash(
  conn: &Connection,
  uid: Option<i64>,
  noteid: &i64,
) -> Result<Option<String>, orgauth::error::Error> {
  if zknote_access_id(&conn, uid, *noteid)? != Access::Private {
    let hash = conn.query_row(
      "select F.hash from zknote N, file F
      where N.id = ?1
      and N.file = F.id",
      params![noteid],
      |row| Ok(row.get(0)?),
    )?;

    Ok(Some(hash))
  } else {
    Ok(None)
  }
}

pub fn read_zknotepubid(
  conn: &Connection,
  uid: Option<i64>,
  pubid: &str,
) -> Result<ZkNote, orgauth::error::Error> {
  let publicid = note_id(&conn, "system", "public")?;
  let mut note = conn.query_row(
    "select A.id, A.uuid, A.title, A.content, A.user, OU.name, U.zknote, A.pubid, A.editable, A.showtitle, A.deleted, A.file, A.createdate, A.changeddate
      from zknote A, user U, orgauth_user OU, zklink L where A.pubid = ?1
      and ((A.id = L.fromid
      and L.toid = ?2) or (A.id = L.toid
      and L.fromid = ?2))
      and U.id = A.user
      and OU.id = A.user",
    params![pubid, publicid],
    |row| {
      Ok(ZkNote {
        id: row.get(0)?,
        uuid: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        user: row.get(4)?,
        username: row.get(5)?,
        usernote: row.get(6)?,
        pubid: row.get(7)?,
        editable: false,
        editableValue: row.get(8)?,
        showtitle: row.get(9)?,
        deleted: row.get(10)?,
        is_file: { let wat : Option<i64> = row.get(11)?;
          match wat {
          Some(_) => true,
          None => false,
        }},
        createdate: row.get(12)?,
        changeddate: row.get(13)?,
        sysids: Vec::new(),
      })
    },
  )?;
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, note.id)?;

  note.sysids = sysids;

  match zknote_access(conn, uid, &note) {
    Ok(zna) => match zna {
      Access::ReadWrite => {
        note.editable = true;
        Ok(note)
      }
      Access::Read => {
        note.editable = false;
        Ok(note)
      }
      // Access::Private => Err(Box::new(std::io::Error::new(
      //   std::io::ErrorKind::PermissionDenied,
      //   "can't read zknote; note is private",
      // ))),
      Access::Private => Err("can't read zknote; note is private".into()),
    },
    Err(e) => Err(e),
  }
}

pub fn delete_zknote(
  conn: &Connection,
  file_path: PathBuf,
  uid: i64,
  noteid: i64,
) -> Result<(), orgauth::error::Error> {
  match zknote_access_id(&conn, Some(uid), noteid)? {
    Access::ReadWrite => Ok::<(), orgauth::error::Error>(()),
    _ => Err("can't delete zknote; write permission denied.".into()),
  }?;

  archive_zknote(&conn, noteid)?;

  // get file info, if any.
  let filerec: Option<(i64, String)> = match conn.query_row(
    "select F.id, F.hash from zknote N, file F
    where N.id = ?1
    and N.file = F.id",
    params![noteid],
    |row| Ok((row.get(0)?, row.get(1)?)),
  ) {
    Ok(fr) => Some(fr),
    Err(rusqlite::Error::QueryReturnedNoRows) => None,
    Err(e) => Err(e)?,
  };

  // only delete when user is the owner.
  conn.execute(
    "update zknote set deleted = 1, title = '<deleted>', content = '', file = null
      where id = ?1
      and user = ?2",
    params![noteid, uid],
  )?;

  // is this file referred to by any other notes?
  match filerec {
    Some((fileid, hash)) => {
      let usecount: i32 = conn.query_row(
        "select count(*) from zknote N
        where N.file = ?1",
        params![fileid],
        |row| Ok(row.get(0)?),
      )?;
      if usecount == 0 {
        let mut rpath = file_path.clone();
        rpath.push(Path::new(&hash));

        std::fs::remove_file(rpath.as_path())?;

        conn.execute("delete from file where id = ?1", params![fileid])?;
      }
    }
    None => (),
  }

  Ok(())
}

pub fn save_zklinks(
  dbfile: &Path,
  uid: i64,
  zklinks: Vec<ZkLink>,
) -> Result<(), orgauth::error::Error> {
  let conn = connection_open(dbfile)?;

  for zklink in zklinks.iter() {
    if zklink.user == uid {
      if zklink.delete == Some(true) {
        // create archive record.
        let now = now()?;
        conn.execute(
          "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
            select fromid, toid, user, linkzknote, createdate, ?1 from zklink
            where fromid = ?2 and toid = ?3 and user = ?4",
          params![now, zklink.from, zklink.to, uid],
        )?;
        // delete link.
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![zklink.from, zklink.to, uid],
        )?;
      } else {
        save_zklink(&conn, zklink.from, zklink.to, uid, zklink.linkzknote)?;
      }
    }
  }

  Ok(())
}

pub fn save_savezklinks(
  conn: &Connection,
  uid: i64,
  zknid: i64,
  zklinks: Vec<SaveZkLink>,
) -> Result<(), orgauth::error::Error> {
  for link in zklinks.iter() {
    let (from, to) = match link.direction {
      Direction::From => (link.otherid, zknid),
      Direction::To => (zknid, link.otherid),
    };
    if link.user == uid {
      if link.delete == Some(true) {
        // create archive record.
        let now = now()?;
        conn.execute(
          "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
            select fromid, toid, user, linkzknote, createdate, ?1 from zklink
            where fromid = ?2 and toid = ?3 and user = ?4",
          params![now, from, to, uid],
        )?;
        // delete the link.
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![from, to, uid],
        )?;
      } else {
        save_zklink(&conn, from, to, uid, link.zknote)?;
      }
    }
  }

  Ok(())
}

pub fn read_zklinks(
  conn: &Connection,
  uid: i64,
  gzl: &GetZkLinks,
) -> Result<Vec<EditLink>, orgauth::error::Error> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;

  let usershares = user_shares(&conn, uid)?;

  let unid = user_note_id(&conn, uid)?;

  // user shares in '1,3,4,5,6' form (minus the quotes!)
  let mut s = usershares
    .iter()
    .map(|x| {
      let mut s = x.to_string();
      s.push_str(",");
      s
    })
    .collect::<String>();
  s.truncate(s.len() - 1);

  // good old fashioned string templating here, since I can't figure out how to
  // do array parameters.
  //
  // zklinks that are mine.
  // +
  // not-mine zklinks with from = this note and toid = note that ISA public.
  // +
  // not-mine zklinks with from = note that is public, and to = this.
  // +
  // not-mine zklinks with from/to = this, and to/from in usershares.
  // +
  // not-mine zklinks from/to notes that link to my usernote.

  let sqlstr = format!(
    "select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user = ?1 and (A.fromid = ?2 or A.toid = ?2)
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.fromid = ?2
      and B.fromid = A.toid
      and B.toid = ?3
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.toid = ?2
      and B.fromid = A.fromid
      and B.toid = ?3
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
        from zklink A, zklink B
        inner join zknote as L ON A.fromid = L.id
        inner join zknote as R ON A.toid = R.id
        where A.user != ?1 and
          ((A.toid = ?2 and A.fromid = B.fromid and B.toid in ({})) or
           (A.toid = ?2 and A.fromid = B.toid and B.fromid in ({})) or
           (A.fromid = ?2 and A.toid = B.fromid and B.toid in ({})) or
           (A.fromid = ?2 and A.toid = B.toid and B.fromid in ({})))
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
        from zklink A, zklink B
        inner join zknote as L ON A.fromid = L.id
        inner join zknote as R ON A.toid = R.id
        where A.user != ?1 and
          ((A.toid = ?2 and A.fromid = B.fromid and B.toid = ?4) or
           (A.toid = ?2 and A.fromid = B.toid and B.fromid = ?4) or
           (A.fromid = ?2 and A.toid = B.fromid and B.toid = ?4) or
           (A.fromid = ?2 and A.toid = B.toid and B.fromid = ?4)) ",
    s, s, s, s
  );

  let mut pstmt = conn.prepare(sqlstr.as_str())?;
  let r = Ok(
    pstmt
      .query_map(params![uid, gzl.zknote, pubid, unid], |row| {
        let fromid = row.get(0)?;
        let toid = row.get(1)?;
        let otherid = if fromid == gzl.zknote { toid } else { fromid };
        let sysids = get_sysids(&conn, sysid, otherid)?;
        Ok(EditLink {
          otherid: otherid,
          direction: if fromid == gzl.zknote {
            Direction::To
          } else {
            Direction::From
          },
          user: row.get(2)?,
          zknote: row.get(3)?,
          othername: if fromid == gzl.zknote {
            row.get(5)?
          } else {
            row.get(4)?
          },
          sysids: sysids,
        })
      })?
      .filter_map(|x| x.ok())
      .collect(),
  );
  r
}

pub fn read_public_zklinks(
  conn: &Connection,
  noteid: i64,
) -> Result<Vec<EditLink>, orgauth::error::Error> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;

  let mut pstmt = conn.prepare(
    // return zklinks that link to or from notes that link to 'public'.
    "select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
       from zklink A, zklink B
       inner join zknote as L ON A.fromid = L.id
       inner join zknote as R ON A.toid = R.id
     where
       (L.user != ?3 and R.user != ?3) and
       ((A.toid = ?1 and A.fromid = B.fromid and B.toid = ?2) or
        (A.fromid = ?1 and A.toid = B.toid and B.fromid = ?2) or
        (A.fromid = ?1 and A.toid = B.fromid and B.toid = ?2))",
  )?;

  let r = Ok(
    pstmt
      .query_map(params![noteid, pubid, sysid], |row| {
        let fromid = row.get(0)?;
        let toid = row.get(1)?;
        let otherid = if fromid == noteid { toid } else { fromid };
        Ok(EditLink {
          otherid: otherid,
          direction: if fromid == noteid {
            Direction::To
          } else {
            Direction::From
          },
          user: row.get(2)?,
          zknote: row.get(3)?,
          othername: if fromid == noteid {
            row.get(5)?
          } else {
            row.get(4)?
          },
          sysids: get_sysids(&conn, sysid, otherid)?,
        })
      })?
      .filter_map(|x| x.ok())
      .collect(),
  );
  r
}

pub fn read_zknotecomments(
  conn: &Connection,
  uid: i64,
  gznc: &GetZkNoteComments,
) -> Result<Vec<ZkNote>, orgauth::error::Error> {
  let cid = note_id(&conn, "system", "comment")?;

  // notes with a TO link to our note
  // and a TO link to 'comment'
  let mut stmt = conn.prepare(
    "select N.fromid from  zklink C, zklink N
      where N.fromid = C.fromid
      and N.toid = ?1 and C.toid = ?2",
  )?;
  let c_iter = stmt.query_map(params![gznc.zknote, cid], |row| Ok(row.get(0)?))?;

  let mut nv = Vec::new();

  for id in c_iter {
    match id {
      Ok(id) => match read_zknote(&conn, Some(uid), id) {
        Ok(note) => {
          nv.push(note);
          match gznc.limit {
            Some(l) => {
              if nv.len() >= l as usize {
                break;
              }
            }
            None => (),
          }
        }
        Err(_) => (),
      },
      Err(_) => (),
    }
  }

  Ok(nv)
}

pub fn read_zknotearchives(
  conn: &Connection,
  uid: i64,
  gzna: &GetZkNoteArchives,
) -> Result<Vec<ZkListNote>, orgauth::error::Error> {
  let aid = note_id(&conn, "system", "archive")?;
  let sysid = user_id(&conn, "system")?;

  // if this is the archive note itself, return empty array.
  // otherwise we can see all the archives for everyone's notes!
  if gzna.zknote == aid {
    return Ok(Vec::new());
  }

  // users that can't see a note, can't see the archives either.
  read_zknote(&conn, Some(uid), gzna.zknote)?;

  // notes with a TO link to our note
  // and a TO link to 'archive'
  let mut stmt = conn.prepare(
    "select N.fromid from zknote, zklink C, zklink N
      where N.fromid = C.fromid
      and zknote.id = N.fromid
      and N.toid = ?1 and C.toid = ?2
      order by zknote.changeddate desc",
  )?;

  let c_iter = stmt
    .query_map(params![gzna.zknote, aid], |row| Ok(row.get(0)?))?
    .skip(gzna.offset as usize);

  let mut nv = Vec::new();

  for id in c_iter {
    match id {
      Ok(id) => {
        let note = read_zklistnote(&conn, Some(sysid), id)?;
        nv.push(note);
        match gzna.limit {
          Some(l) => {
            if nv.len() >= l as usize {
              break;
            }
          }
          None => (),
        }
      }
      Err(_) => (),
    }
  }

  Ok(nv)
}

pub fn read_archivezknote(
  conn: &Connection,
  uid: i64,
  gazn: &GetArchiveZkNote,
) -> Result<ZkNote, orgauth::error::Error> {
  let archiveid = note_id(&conn, "system", "archive")?;
  let sysid = user_id(&conn, "system")?;

  if gazn.parentnote == archiveid {
    bail!("query not allowed!");
  }

  // have access to the parent note?
  read_zknote(conn, Some(uid), gazn.parentnote)?;

  // archive note should have a TO link to the parent note
  // and a TO link to 'archive'
  let _i: i64 = match conn.query_row(
    "select N.fromid from zklink C, zklink N
      where N.fromid = C.fromid
      and N.toid = ?1 and C.toid = ?2 and N.fromid = ?3",
    params![gazn.parentnote, archiveid, gazn.noteid],
    |row| Ok(row.get(0)?),
  ) {
    Ok(v) => Ok::<i64, orgauth::error::Error>(v),
    Err(rusqlite::Error::QueryReturnedNoRows) => Err("not an archive note!".into()),
    Err(x) => Err(x.into()),
  }?;

  // now read the archive note AS SYSTEM.
  read_zknote(conn, Some(sysid), gazn.noteid)
}

pub fn read_archivezklinks(
  conn: &Connection,
  uid: i64,
  after: i64,
) -> Result<Vec<ArchiveZkLink>, orgauth::error::Error> {
  let (acc_sql, acc_args) = accessible_notes(&conn, uid)?;

  let mut pstmt = conn.prepare(
    format!(
      "with accessible_notes as ({})
      select ZLA.user, FN.uuid, TN.uuid, LN.uuid, ZLA.createdate, ZLA.deletedate
      from zklinkarchive ZLA, zknote FN, zknote TN, zknote LN
      where FN.id = ZLA.fromid
      and TN.id = ZLA.toid
      and LN.id = ZLA.toid
      and ZLA.fromid in accessible_notes
      and ZLA.toid in accessible_notes",
      acc_sql
    )
    .as_str(),
  )?;

  let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
    Ok(ArchiveZkLink {
      userUuid: row.get(0)?,
      fromUuid: row.get(1)?,
      toUuid: row.get(2)?,
      linkUuid: row.get(3)?,
      createdate: row.get(4)?,
      deletedate: row.get(5)?,
    })
  })?;

  Ok(rec_iter.filter_map(|x| x.ok()).collect())
}

pub fn read_archivezklinks_stream(
  conn: Arc<Connection>,
  uid: i64,
  after: i64,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> + 'static {
  // {
  try_stream! {
    let (acc_sql, acc_args) = accessible_notes(&conn, uid)?;

    let mut pstmt = conn.prepare(
      format!(
        "with accessible_notes as ({})
      select ZLA.user, FN.uuid, TN.uuid, LN.uuid, ZLA.createdate, ZLA.deletedate
      from zklinkarchive ZLA, zknote FN, zknote TN, zknote LN
      where FN.id = ZLA.fromid
      and TN.id = ZLA.toid
      and LN.id = ZLA.toid
      and ZLA.fromid in accessible_notes
      and ZLA.toid in accessible_notes",
        acc_sql
      )
      .as_str(),
    )?;

    let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
      Ok(ArchiveZkLink {
        userUuid: row.get(0)?,
        fromUuid: row.get(1)?,
        toUuid: row.get(2)?,
        linkUuid: row.get(3)?,
        createdate: row.get(4)?,
        deletedate: row.get(5)?,
      })
    })?;

    {
      let mut s = serde_json::to_value(PrivateReplyMessage {
        what: PrivateReplies::ArchiveZkLinks, // "archivezklinks".to_string(),
        content: serde_json::Value::Null,
      })?
      .to_string();
      s.push_str("\n");
      yield Bytes::from(s);
    }

    for rec in rec_iter {
      if let Ok(r) = rec {
        let mut s = serde_json::to_value(r)?.to_string();
        s.push_str("\n");
        yield Bytes::from(s);
      }
    }
  }
}

pub fn read_zklinks_since(
  conn: &Connection,
  uid: i64,
  after: i64,
) -> Result<Vec<UuidZkLink>, orgauth::error::Error> {
  let (acc_sql, acc_args) = accessible_notes(&conn, uid)?;

  println!("acc_sql {}", acc_sql);
  println!("acc_args {:?}", acc_args);

  println!("anotes");

  let mut astmt = conn.prepare(acc_sql.as_str())?;

  let arec_iter = astmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
    println!("accnote {:?}", row.get::<usize, i64>(0)?);
    Ok(())
  })?;

  for ar in arec_iter {
    println!("ac {:?}", ar);
  }

  println!("linzk");

  let mut pstmt = conn.prepare(
    format!(
      "with accessible_notes as ({})
        select OU.uuid, FN.uuid, TN.uuid, ZL.createdate
        from zklink ZL, zknote FN, zknote TN, orgauth_user OU
        where FN.id = ZL.fromid
        and TN.id = ZL.toid
        and ZL.user = OU.id
        and ZL.fromid in accessible_notes
        and ZL.toid in accessible_notes",
      acc_sql
    )
    .as_str(),
  )?;

  println!("accarts {}", acc_args.len());

  let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
    println!("uuidzklink {:?}", row.get::<usize, String>(0)?);
    Ok(UuidZkLink {
      userUuid: row.get(0)?,
      fromUuid: row.get(1)?,
      toUuid: row.get(2)?,
      linkUuid: None,
      createdate: row.get(3)?,
    })
  })?;

  Ok(rec_iter.collect::<Result<Vec<UuidZkLink>, rusqlite::Error>>()?)
}
pub fn read_zklinks_since_stream(
  conn: Arc<Connection>,
  uid: i64,
  after: i64,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> + 'static {
  // {
  try_stream! {
    let (acc_sql, acc_args) = accessible_notes(&conn, uid)?;

    let mut astmt = conn.prepare(acc_sql.as_str())?;

    let arec_iter = astmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
      println!("accnote {:?}", row.get::<usize, i64>(0)?);
      Ok(())
    })?;

    for ar in arec_iter {
      println!("ac {:?}", ar);
    }

    println!("linzk");

    let mut pstmt = conn.prepare(
      format!(
        "with accessible_notes as ({})
        select OU.uuid, FN.uuid, TN.uuid, ZL.createdate
        from zklink ZL, zknote FN, zknote TN, orgauth_user OU
        where FN.id = ZL.fromid
        and TN.id = ZL.toid
        and ZL.user = OU.id
        and ZL.fromid in accessible_notes
        and ZL.toid in accessible_notes",
        acc_sql
      )
      .as_str(),
    )?;

    println!("accarts {}", acc_args.len());

    {
      let mut s = serde_json::to_value(PrivateReplyMessage {
        what: PrivateReplies::ZkLinks,
        content: serde_json::Value::Null,
      })?
      .to_string();
      s.push_str("\n");
      yield Bytes::from(s);
    }

    let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
      println!("uuidzklink {:?}", row.get::<usize, String>(0)?);
      Ok(UuidZkLink {
        userUuid: row.get(0)?,
        fromUuid: row.get(1)?,
        toUuid: row.get(2)?,
        linkUuid: None,
        createdate: row.get(3)?,
      })
    })?;

    for rec in rec_iter {
      if let Ok(r) = rec {
        let mut s = serde_json::to_value(r)?.to_string();
        s.push_str("\n");
        yield Bytes::from(s);
      }
    }
  }
}

pub fn accessible_notes(
  conn: &Connection,
  uid: i64,
) -> Result<(String, Vec<String>), orgauth::error::Error> {
  let publicid = note_id(&conn, "system", "public")?;
  let archiveid = note_id(&conn, "system", "archive")?;
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = user_note_id(&conn, uid)?;

  // query: archivelinks that attach to notes I can access.
  // my notes + public notes + shared notes + ??

  let (mut sqlbase, mut baseargs) = {
    // notes that are mine.
    (
      format!(
        "select N.id
        from zknote N where N.user = ?"
      ),
      vec![uid.to_string()],
    )
  };

  // notes that are public, and not mine.
  let (sqlpub, mut pubargs) = {
    (
      format!(
        "select N.id
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)"
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
  let (sqlshare, mut shareargs) = {
    (
      format!(
        "select N.id
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ?
        and M.toid = ?
        and ((L.fromid = N.id and L.toid = M.fromid )
             or (L.toid = N.id and L.fromid = M.fromid ))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))",
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
  let (sqluser, mut userargs) = {
    (
      format!(
        "select N.id
      from zknote N, zklink L
      where (
        N.user != ? and
        ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?)))"
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  };

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlpub.as_str());
  baseargs.append(&mut pubargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlshare.as_str());
  baseargs.append(&mut shareargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqluser.as_str());
  baseargs.append(&mut userargs);

  Ok((sqlbase, baseargs))
}

pub fn read_zknoteedit(
  conn: &Connection,
  uid: Option<i64>,
  zknoteid: i64,
) -> Result<ZkNoteAndLinks, orgauth::error::Error> {
  // should do an ownership check for us
  let zknote = read_zknote(conn, uid, zknoteid)?;

  let zklinks = match uid {
    Some(uid) => read_zklinks(conn, uid, &GetZkLinks { zknote: zknote.id })?,

    None => read_public_zklinks(conn, zknote.id)?,
  };

  Ok(ZkNoteAndLinks {
    zknote: zknote,
    links: zklinks,
  })
}

pub fn read_zneifchanged(
  conn: &Connection,
  uid: Option<i64>,
  gzic: &GetZnlIfChanged,
) -> Result<Option<ZkNoteAndLinks>, orgauth::error::Error> {
  let changeddate: i64 = conn.query_row(
    "select changeddate from zknote N
      where N.id = ?1",
    params![gzic.zknote],
    |row| Ok(row.get(0)?),
  )?;

  if changeddate > gzic.changeddate {
    return read_zknoteedit(conn, uid, gzic.zknote).map(Some);
  } else {
    Ok(None)
  }
}

pub fn save_importzknotes(
  conn: &Connection,
  uid: i64,
  izns: Vec<ImportZkNote>,
) -> Result<(), orgauth::error::Error> {
  for izn in izns.iter() {
    // create the note if it doesn't exist.
    let nid = match note_id2(&conn, uid, izn.title.as_str())? {
      Some(i) => {
        // update the content.
        conn.execute(
          "update zknote set content = ?1 where
            user = ?2 and id = ?3",
          params![izn.content, uid, i],
        )?;

        i
      }
      None => {
        // new note.
        save_zknote(
          &conn,
          uid,
          &SaveZkNote {
            id: None,
            title: izn.title.clone(),
            pubid: None,
            content: izn.content.clone(),
            editable: false,
            showtitle: true,
            deleted: false,
          },
        )?
        .id
      }
    };
    // now add the 'from' links.
    for title in izn.fromLinks.iter() {
      // if the 'from' note doesn't exist, create it.
      let fromid = match note_id2(&conn, uid, title)? {
        Some(n) => n,
        None => {
          // new note.
          save_zknote(
            &conn,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
              deleted: false,
            },
          )?
          .id
        }
      };

      // save link.
      save_zklink(&conn, fromid, nid, uid, None)?;
    }
    // add the 'to' links (and their notes)
    for title in izn.toLinks.iter() {
      let toid = match note_id2(&conn, uid, title)? {
        Some(n) => n,
        None => {
          // new note.
          save_zknote(
            &conn,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
              deleted: false,
            },
          )?
          .id
        }
      };

      // save link.
      save_zklink(&conn, nid, toid, uid, None)?;
    }
  }

  Ok(())
}

pub fn make_file_note(
  conn: &Connection,
  uid: i64,
  name: &String,
  fpath: &Path,
) -> Result<(i64, i64), orgauth::error::Error> {
  // compute hash.
  // let fpath = Path::new(&filepath);
  let fh = sha256::try_digest(fpath)?;
  let size = std::fs::metadata(fpath)?.len();
  let fhp = format!("files/{}", fh);
  let hashpath = Path::new(&fhp);

  // file exists?
  if hashpath.exists() {
    // new file already exists.
    std::fs::remove_file(fpath)?;
  } else {
    // move into hashed-files dir.
    std::fs::rename(fpath, hashpath)?;
  }

  // table entry exists?
  let oid: Option<i64> =
    match conn.query_row("select id from file where hash = ?1", params![fh], |row| {
      Ok(row.get(0)?)
    }) {
      Ok(v) => Ok(Some(v)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(x) => Err(x),
    }?;

  // use existing id, or create new
  let fid = match oid {
    Some(id) => id,
    None => {
      let now = now()?;

      // add table entry
      conn.execute(
        "insert into file (hash, createdate, size)
                 values (?1, ?2, ?3)",
        params![fh, now, size],
      )?;
      conn.last_insert_rowid()
    }
  };

  // now make a new note.
  let sn = save_zknote(
    &conn,
    uid,
    &SaveZkNote {
      id: None,
      title: name.to_string(),
      pubid: None,
      content: "".to_string(),
      editable: false,
      showtitle: false,
      deleted: false,
    },
  )?;

  // set the file id in that note.
  set_zknote_file(&conn, sn.id, fid)?;

  Ok((sn.id, fid))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZkDatabase {
  notes: Vec<ZkNote>,
  links: Vec<ZkLink>,
  users: Vec<User>,
}

pub fn export_db(_dbfile: &Path) -> Result<ZkDatabase, orgauth::error::Error> {
  /*  let conn = connection_open(dbfile)?;
  let sysid = user_id(&conn, "system")?;

  // Users
  let mut ustmt = conn.prepare(
    "select id, name, zknote, homenote, hashwd, salt, email, registration_key
      from user",
  )?;

  let u_iter = ustmt.query_map(params![], |row| {
    Ok(User {
      id: row.get(0)?,
      name: row.get(1)?,
      noteid: row.get(2)?,
      homenoteid: row.get(3)?,
      hashwd: row.get(4)?,
      salt: row.get(5)?,
      email: row.get(6)?,
      registration_key: row.get(7)?,
    })
  })?;

  let uv = u_iter.filter_map(|x| x.ok()).collect();

  // Notes
  let mut nstmt = conn.prepare(
    "select ZN.id, ZN.title, ZN.content, ZN.user, U.name, U.zknote, ZN.pubid, ZN.editable, ZN.createdate, ZN.changeddate
      from zknote ZN, user U
      where ZN.user == U.id",
  )?;

  let n_iter = nstmt.query_map(params![], |row| {
    let sysids = get_sysids(&conn, sysid, row.get(0)?)?;
    Ok(ZkNote {
      id: row.get(0)?,
      title: row.get(1)?,
      content: row.get(2)?,
      user: row.get(3)?,
      username: row.get(4)?,
      usernote: row.get(5)?,
      pubid: row.get(6)?,
      editable: false,
      editableValue: row.get(7)?,
      showtitle: row.get(8)?,
      createdate: row.get(9)?,
      changeddate: row.get(10)?,
      sysids: sysids,
    })
  })?;

  let nv = n_iter.filter_map(|x| x.ok()).collect();

  // Links
  let mut lstmt = conn.prepare(
    "select A.fromid, A.toid, A.user, A.linkzknote
      from zklink A",
  )?;

  let l_iter = lstmt.query_map(params![], |row| {
    Ok(ZkLink {
      from: row.get(0)?,
      to: row.get(1)?,
      user: row.get(2)?,
      delete: None,
      linkzknote: row.get(3)?,
      fromname: None,
      toname: None,
    })
  })?;

  let lv = l_iter.filter_map(|x| x.ok()).collect();

  Ok(ZkDatabase {
    notes: nv,
    links: lv,
    users: uv,
  })*/

  bail!("unimplemented");
}
