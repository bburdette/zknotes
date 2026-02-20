use crate::error as zkerr;
use crate::error::to_orgauth_error;
use crate::migrations as zkm;
use async_stream::try_stream;
use barrel::backend::Sqlite;
use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use lapin::Channel;
use log::{error, info};
use orgauth::data::{RegistrationData, UserId};
use orgauth::dbfun::user_id;
use orgauth::endpoints::Callbacks;
use orgauth::util::now;
use rusqlite::Row;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::constants::SpecialUuids;
use zkprotocol::content::{
  ArchiveZkLink, Direction, EditLink, ExtraLoginData, FileInfo, FileStatus, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, LzLink, OnMakeFileNote, OnSavedZkNote,
  SaveZkLink, SaveZkLink2, SaveZkNote, SavedZkNote, Server, Sysids, UuidZkLink, ZkLink, ZkListNote,
  ZkNote, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteId,
};
use zkprotocol::sync_data::SyncMessage;

pub fn zknotes_callbacks() -> Callbacks {
  Callbacks {
    on_new_user: Box::new(on_new_user),
    extra_login_data: Box::new(extra_login_data_callback),
    on_delete_user: Box::new(on_delete_user),
  }
}

pub fn local_server_id(conn: &Connection) -> Result<Server, zkerr::Error> {
  Ok(conn.query_row(
    "select id, uuid from server where uuid = (select value from singlevalue where name = 'server_id')",
    params![],
    |row| Ok( Server { id: row.get(0)?, uuid: row.get(1)?}),
  )?)
}

pub fn server_id(conn: &Connection, uuid: &str) -> Result<i64, zkerr::Error> {
  let id: i64 = conn.query_row(
    "select id from server
      where uuid = ?1",
    params![uuid],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn on_new_user(
  conn: &Connection,
  rd: &RegistrationData,
  data: Option<String>,
  remote_data: Option<String>,
  creator: Option<UserId>,
  uid: UserId,
) -> Result<(), orgauth::error::Error> {
  let usernoteid = note_id(&conn, "system", "user")?;
  let publicnoteid = note_id(&conn, "system", "public")?;
  let systemid = user_id(&conn, "system")?;

  let now = now()?;

  let user_note_uuid = match remote_data {
    Some(remote_data) => {
      let remd: ExtraLoginData = serde_json::from_str(remote_data.as_str())?;
      remd.zknote
    }
    None => ZkNoteId::Zni(uuid::Uuid::new_v4()),
  };

  let server = local_server_id(conn).map_err(zkerr::to_orgauth_error)?;

  // make a corresponding note,
  conn.execute(
    "insert into zknote (title, content, user, editable, showtitle, deleted, uuid, server, createdate, changeddate)
     values (?1, ?2, ?3, 0, 1, 0, ?4, ?5, ?6, ?7)",
    params![rd.uid, "", systemid.to_i64(), user_note_uuid.to_string(), server.id, now, now],
  )?;

  let zknid = conn.last_insert_rowid();

  // make a user record.
  conn.execute(
    "insert into user (id, zknote)
      values (?1, ?2)",
    params![uid.to_i64(), zknid],
  )?;

  conn.execute(
    "update zknote set sysdata = ?1
        where id = ?2",
    params![systemid.to_i64(), uid.to_string().as_str()],
  )?;

  // indicate a 'user' record, and 'public'
  save_zklink(&conn, zknid, usernoteid, systemid, None).map_err(zkerr::to_orgauth_error)?;
  save_zklink(&conn, zknid, publicnoteid, systemid, None).map_err(zkerr::to_orgauth_error)?;

  // add extra links from 'data'
  match (&data, creator) {
    (Some(data), Some(creator)) => {
      let extra_links: Vec<SaveZkLink> = serde_json::from_str(data.as_str())?;
      save_savezklinks(&conn, creator, user_note_uuid, &extra_links)
        .map_err(zkerr::to_orgauth_error)?;
    }
    _ => (),
  }

  Ok(())
}

// callback to pass to orgauth
pub fn extra_login_data_callback(
  conn: &Connection,
  uid: UserId,
) -> Result<Option<serde_json::Value>, orgauth::error::Error> {
  Ok(Some(serde_json::to_value(
    read_extra_login_data(&conn, uid).map_err(to_orgauth_error)?,
  )?))
}

// for-real delete of user - no archives?
pub fn on_delete_user(conn: &Connection, uid: UserId) -> Result<bool, orgauth::error::Error> {
  // try deleting all their links and notes.
  // TODO: delete archive notes that have system ownership.
  conn.execute(
    "delete from zklinkarchive where user = ?1",
    params!(uid.to_i64()),
  )?;
  conn.execute("delete from zklink where user = ?1", params!(uid.to_i64()))?;
  conn.execute("delete from zknote where user = ?1", params!(uid.to_i64()))?;
  conn.execute("delete from user where id = ?1", params!(uid.to_i64()))?;
  Ok(true)
}

pub fn sysids() -> Result<Sysids, zkerr::Error> {
  Ok(Sysids {
    publicid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::Public.str())?),
    commentid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::Comment.str())?),
    shareid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::Share.str())?),
    searchid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::Search.str())?),
    userid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::User.str())?),
    systemid: ZkNoteId::Zni(Uuid::parse_str(SpecialUuids::System.str())?),
  })
}

// will this work??
pub fn set_homenote(
  conn: &Connection,
  uid: UserId,
  homenote: &ZkNoteId,
) -> Result<(), zkerr::Error> {
  conn.execute(
    "update user set homenote = zknote.id
        from zknote
           where user.id = ?2 and zknote.uuid = ?1",
    params![homenote.to_string(), uid.to_i64()],
  )?;

  Ok(())
}

pub fn connection_open(dbfile: &Path) -> Result<Connection, zkerr::Error> {
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

pub fn get_single_value(conn: &Connection, name: &str) -> Result<Option<String>, zkerr::Error> {
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

pub fn set_single_value(conn: &Connection, name: &str, value: &str) -> Result<(), zkerr::Error> {
  conn.execute(
    "insert into singlevalue (name, value) values (?1, ?2)
        on conflict (name) do update set value = ?2 where name = ?1",
    params![name, value],
  )?;
  Ok(())
}

// update the single value with a function.  should be atomic, ie if someone else gets there
// first it will try again.
pub fn update_single_value(
  conn: &Connection,
  name: &str,
  updatefn: fn(x: &str) -> String,
) -> Result<String, zkerr::Error> {
  loop {
    let val: String = conn.query_row(
      "select value from singlevalue where name = ?1",
      params![name],
      |row| Ok(row.get(0)?),
    )?;

    let newval = updatefn(val.as_str());

    let rows = conn.execute(
      "update singlevalue set value = ?1 where name = ?2 and value = ?3",
      params![newval, name, val],
    )?;

    if rows == 1 {
      return Ok(newval);
    }
  }
}

pub fn dbinit(dbfile: &Path, token_expiration_ms: Option<i64>) -> Result<Server, zkerr::Error> {
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
  if nlevel < 31 {
    info!("udpate31");
    zkm::udpate31(&dbfile)?;
    set_single_value(&conn, "migration_level", "31")?;
  }
  if nlevel < 32 {
    info!("udpate32");
    zkm::udpate32(&dbfile)?;
    set_single_value(&conn, "migration_level", "32")?;
  }
  if nlevel < 33 {
    info!("udpate33");
    zkm::udpate33(&dbfile)?;
    set_single_value(&conn, "migration_level", "33")?;
  }
  if nlevel < 34 {
    info!("udpate34");
    zkm::udpate34(&dbfile)?;
    set_single_value(&conn, "migration_level", "34")?;
  }
  if nlevel < 35 {
    info!("udpate35");
    zkm::udpate35(&dbfile)?;
    set_single_value(&conn, "migration_level", "35")?;
  }
  if nlevel < 36 {
    info!("udpate36");
    zkm::udpate36(&dbfile)?;
    set_single_value(&conn, "migration_level", "36")?;
  }
  if nlevel < 37 {
    info!("udpate37");
    zkm::udpate37(&dbfile)?;
    set_single_value(&conn, "migration_level", "37")?;
  }
  if nlevel < 38 {
    info!("udpate38");
    zkm::udpate38(&dbfile)?;
    set_single_value(&conn, "migration_level", "38")?;
  }
  if nlevel < 39 {
    info!("udpate39");
    zkm::udpate39(&dbfile)?;
    set_single_value(&conn, "migration_level", "39")?;
  }
  if nlevel < 40 {
    info!("udpate40");
    zkm::udpate40(&dbfile)?;
    set_single_value(&conn, "migration_level", "40")?;
  }

  info!("db up to date.");

  if let Some(expms) = token_expiration_ms {
    orgauth::dbfun::purge_login_tokens(&conn, expms)?;
  }

  let server = local_server_id(&conn)?;

  Ok(server)
}

// read the uuidzklink.
pub fn read_uuidzklink(
  conn: &Connection,
  fromid: i64,
  toid: i64,
  user: UserId,
) -> Result<UuidZkLink, zkerr::Error> {
  conn
    .query_row(
      "select F.uuid, T.uuid, OU.uuid, LN.uuid, zklink.createdate 
      from zklink, zknote F, zknote T, orgauth_user OU
      left join zknote LN on zklink.linkzknote = LN.id
      where zklink.fromid = F.id
       and F.id = ?1
       and  zklink.toid = T.id
       and T.id = ?2
       and zklink.user = OU.id
       and OU.id = ?3",
      params![fromid, toid, user.to_i64()],
      |row| {
        Ok(UuidZkLink {
          fromUuid: row.get(0)?,
          toUuid: row.get(1)?,
          userUuid: row.get(2)?,
          linkUuid: row.get(3)?,
          createdate: row.get(4)?,
        })
      },
    )
    .map_err(|e| e.into())
}

// really just for checking existence of the zklink.
pub fn read_uuidzklink_linkzknote(
  conn: &Connection,
  fromid: &str,
  toid: &str,
  user: &str,
) -> Result<Option<String>, zkerr::Error> {
  conn
    .query_row(
      "select L.uuid from
      zklink, zknote F, zknote T, orgauth_user OU
      left join zknote L on L.id = zklink.linkzknote
      where zklink.fromid = F.id
       and F.uuid = ?1
       and zklink.toid = T.id
       and T.uuid = ?2
       and zklink.user = OU.id
       and OU.uuid = ?3",
      params![fromid, toid, user],
      |row| Ok(row.get(0)?),
    )
    .map_err(|e| e.into())
}

// // really just for checking existence of the zklink.
// pub fn read_uuidzklink_linkzknote(
//   conn: &Connection,
//   fromid: &str,
//   toid: &str,
//   user: &str,
// ) -> Result<Option<String>, zkerr::Error> {
//   let (l, r, u, lz) = conn.query_row(
//     "select F.id, T.id, zklink.user, L.uuid from
//       zklink, zknote F, zknote T, orgauth_user OU
//       left join zknote L on L.id = zklink.linkzknote
//       where zklink.fromid = F.id
//        and F.uuid = ?1
//        and zklink.toid = T.id
//        and T.uuid = ?2
//        and zklink.user = OU.id
//        and OU.uuid = ?3",
//     params![fromid, toid, user],
//     |row| {
//       Ok((
//         row.get::<usize, i64>(0)?,
//         row.get::<usize, i64>(1)?,
//         row.get::<usize, i64>(2)?,
//         row.get::<usize, Option<String>>(3)?,
//       ))
//     },
//   )?;
//   // .map_err(|e| e.into())?;
//   println!("(l, r, u, lz) {:?}", (l, r, u, &lz));
//   Ok(lz)
// }

// user CRUD

pub fn save_zklink(
  conn: &Connection,
  fromid: i64,
  toid: i64,
  user: UserId,
  linkzknote: Option<i64>,
) -> Result<i64, zkerr::Error> {
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
  (if authed {
    Ok(())
  } else {
    Err(zkerr::Error::String("link not allowed".into()))
  })?;

  let now = now()?;

  // if there's already a record that is NOT like this record, make an archive record.
  // if the record is just like what we have now, nothing happens.
  conn.execute(
    "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
      select fromid, toid, user, linkzknote, createdate, ?1 from zklink
      where fromid = ?2 and toid = ?3 and user = ?4 and linkzknote <> ?5",
    params![now, fromid, toid, user.to_i64(), linkzknote],
  )?;

  // now create the new record or modify the existing.
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote, createdate) values (?1, ?2, ?3, ?4, ?5)
      on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
    params![fromid, toid, user.to_i64(), linkzknote, now],
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

pub fn note_id2(conn: &Connection, uid: UserId, title: &str) -> Result<Option<i64>, zkerr::Error> {
  match conn.query_row(
    "select zknote.id from
      zknote
      where zknote.title = ?2
      and zknote.user = ?1",
    params![uid.to_i64(), title],
    |row| Ok(row.get(0)?),
  ) {
    Err(e) => match e {
      rusqlite::Error::QueryReturnedNoRows => Ok(None),
      _ => Err(e.into()),
    },
    Ok(i) => Ok(i),
  }
}

pub fn uuid_for_note_id(conn: &Connection, id: i64) -> Result<Uuid, zkerr::Error> {
  let s: String = conn.query_row(
    "select zknote.uuid from zknote
      where zknote.id = ?1",
    params![id],
    |row| Ok(row.get(0)?),
  )?;
  Ok(Uuid::parse_str(s.as_str())?)
}

pub fn note_id_for_zknoteid(conn: &Connection, zknoteid: &ZkNoteId) -> Result<i64, zkerr::Error> {
  match zknoteid {
    ZkNoteId::Zni(uuid) => note_id_for_uuid(conn, uuid),
    ZkNoteId::ArchiveZni(_, _) => Err(zkerr::Error::ArchiveNoteNotAllowed),
  }
}
pub fn archive_note_id_for_zknoteid(
  conn: &Connection,
  zknoteid: &ZkNoteId,
) -> Result<i64, zkerr::Error> {
  match zknoteid {
    ZkNoteId::Zni(_uuid) => Err(zkerr::Error::ArchiveNoteRequired),
    ZkNoteId::ArchiveZni(uuid, _) => archive_note_id_for_uuid(conn, uuid),
  }
}

pub fn note_id_for_uuid(conn: &Connection, uuid: &Uuid) -> Result<i64, zkerr::Error> {
  let id: i64 = conn
    .query_row(
      "select zknote.id from zknote
      where zknote.uuid = ?1",
      params![uuid.to_string()],
      |row| Ok(row.get(0)?),
    )
    .map_err(|e| zkerr::annotate_string("note not found".to_string(), e.into()))?;
  Ok(id)
}

pub fn archive_note_id_for_uuid(conn: &Connection, uuid: &Uuid) -> Result<i64, zkerr::Error> {
  let id: i64 = conn
    .query_row(
      "select id from zkarch
      where uuid = ?1",
      params![uuid.to_string()],
      |row| Ok(row.get(0)?),
    )
    .map_err(|e| zkerr::annotate_string("note not found".to_string(), e.into()))?;
  Ok(id)
}

pub fn user_note_id(conn: &Connection, uid: UserId) -> Result<i64, zkerr::Error> {
  let id: i64 = conn.query_row(
    "select zknote from user
      where user.id = ?1",
    params![uid.to_i64()],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn user_shares(conn: &Connection, uid: UserId) -> Result<Vec<i64>, zkerr::Error> {
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
  uid: UserId,
) -> Result<bool, zkerr::Error> {
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  let ret = are_notes_linked(&conn, zknoteid, usernoteid)?;

  Ok(ret)
}

pub fn is_zknote_shared(
  conn: &Connection,
  zknoteid: i64,
  uid: UserId,
) -> Result<bool, zkerr::Error> {
  let shareid: i64 = note_id(conn, "system", "share")?;
  let publicid: i64 = note_id(conn, "system", "public")?;
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  // does note link to a note that links to share?
  // and does that note link to usernoteid?
  let ret: Result<bool, zkerr::Error> = match conn.query_row(
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

pub fn is_zknote_public(conn: &Connection, zknoteid: i64) -> Result<bool, zkerr::Error> {
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
  userid: UserId,
) -> Result<bool, zkerr::Error> {
  match conn.query_row(
    "select count(*) from
      zknote
      where id = ?1 and user = ?2",
    params![zknoteid, userid.to_i64()],
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

pub fn are_notes_linked(conn: &Connection, nid1: i64, nid2: i64) -> Result<bool, zkerr::Error> {
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

pub fn archive_zknote_i64(conn: &Connection, noteid: i64) -> Result<(), zkerr::Error> {
  let uuid = uuid::Uuid::new_v4();
  conn.execute(
    "insert into zkarch (zknote, title, content, user, editable, showtitle, deleted, uuid, createdate, changeddate, server)
     select id, title, content, user, editable, showtitle, deleted, ?1, createdate, changeddate, server from
         zknote where id = ?2",
    params![uuid.to_string(), noteid],
  )?;

  Ok(())
}

// write a zknote straight to archives.  should only happen during sync.
pub fn archive_zknote(
  conn: &Connection,
  noteid: i64,
  uid: &UserId,
  note: &ZkNote,
) -> Result<(i64, SavedZkNote), zkerr::Error> {
  let uuid = uuid::Uuid::new_v4();
  // copy the note, with user 'system'.
  // exclude pubid, to avoid unique constraint problems.
  conn.execute(
    "insert into zkarch (zknote, title, content, user, editable, showtitle, deleted, uuid, createdate, changeddate, server)
     values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, (select id from server where uuid = ?11))",
    params![
      noteid,
      note.title,
      note.content,
      uid.to_i64(),
      note.editable,
      note.showtitle,
      note.deleted,
      uuid.to_string(),
      note.createdate,
      note.changeddate,
      note.server
    ])?;

  let archive_note_id = conn.last_insert_rowid();

  Ok((
    archive_note_id,
    SavedZkNote {
      id: ZkNoteId::Zni(uuid),
      changeddate: note.changeddate,
      server: note.server.clone(),
      what: None,
    },
  ))
}

pub fn set_zknote_file(conn: &Connection, noteid: i64, fileid: i64) -> Result<(), zkerr::Error> {
  conn.execute(
    "update zknote set file = ?1
         where id = ?2",
    params![fileid, noteid],
  )?;
  Ok(())
}

#[derive(Debug)]
pub struct LapinInfo {
  pub channel: Channel,
  pub token: String,
}

pub async fn make_lapin_info(
  conn: Option<&lapin::Connection>,
  token: Option<String>,
) -> Option<LapinInfo> {
  match (conn, token) {
    (Some(conn), Some(token)) => match make_lapin_channels(conn).await {
      Ok(lc) => Some(LapinInfo { channel: lc, token }),
      Err(e) => {
        error!("{e}");
        None
      }
    },
    _ => None,
  }
}

pub async fn make_lapin_channels(conn: &lapin::Connection) -> Result<lapin::Channel, zkerr::Error> {
  let chan = conn.create_channel().await?;
  info!("lapin channel created {:?}", chan);
  chan
    .queue_declare(
      "on_save_zknote",
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await?;
  chan
    .queue_declare(
      "on_make_file_note",
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await?;
  Ok(chan)
}

pub struct NoteDates {
  pub createdate: i64,
  pub changeddate: i64,
}

pub async fn save_zknote(
  conn: &Connection,
  lapin_info: &Option<LapinInfo>,
  server: &Server,
  uid: UserId,
  note: &SaveZkNote,
  dates: Option<NoteDates>, // just used during sync!
) -> Result<(i64, SavedZkNote), zkerr::Error> {
  let (createdate, changeddate) = match dates {
    Some(dates) => (dates.createdate, dates.changeddate),
    None => {
      let now = now()?;
      (now, now)
    }
  };

  async fn publish_szn(
    uid: UserId,
    lapin_info: &Option<LapinInfo>,
    szn: &SavedZkNote,
  ) -> Result<(), zkerr::Error> {
    if let Some(li) = lapin_info {
      let oszn = OnSavedZkNote {
        id: szn.id,
        user: uid,
        token: li.token.clone(),
      };
      match li
        .channel
        .basic_publish(
          "",
          "on_save_zknote",
          lapin::options::BasicPublishOptions::default(),
          &serde_json::to_vec(&oszn)?[..],
          lapin::BasicProperties::default(),
        )
        .await
      {
        Ok(_) => info!("published to amqp on_save_zknote"),
        Err(e) => error!("error publishing to AMQP: {:?}", e),
      }
    }

    Ok(())
  }

  match note.id {
    Some(uuid) => {
      let id = note_id_for_zknoteid(conn, &uuid)?;

      // check access before creating the archive note.
      match zknote_access_id(&conn, Some(uid), id)? {
        Access::Private => return Err(zkerr::Error::NoteIsPrivate),
        Access::Read => return Err(zkerr::Error::NoteIsReadOnly),
        Access::ReadWrite => (),
      };

      archive_zknote_i64(&conn, id)?;
      // existing note.  update IF mine.
      match conn.execute(
        "update zknote set title = ?1,
           content = ?2,
           changeddate = ?3,
           pubid = ?4,
           editable = ?5,
           showtitle = ?6,
           deleted = ?7,
           server = ?8
         where id = ?9 and user = ?10",
        params![
          note.title,
          note.content,
          changeddate,
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          server.id,
          id,
          uid.to_i64(),
        ],
      ) {
        Ok(1) => {
          let szn = SavedZkNote {
            id: uuid,
            changeddate,
            server: server.uuid.clone(),
            what: note.what.clone(),
          };
          publish_szn(uid, lapin_info, &szn).await?;
          Ok((id, szn))
        }
        Ok(0) => {
          // editable flag must be true
          match conn.execute(
            "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4, showtitle = ?5, server = ?6
             where id = ?7 and editable = 1",
            params![note.title, note.content, changeddate, note.pubid, note.showtitle, server.id, id],
          )? {
            0 => Err( zkerr::Error::String(format!("can't update; note is not writable {} {}", note.title, id))),
            // params![note.title, note.content, now, note.pubid, note.showtitle, server.id, id],
            1 => {
              let szn = SavedZkNote {
                id: uuid,
                changeddate,
                server: server.uuid.clone(),
                what: note.what.clone(),
              };
              publish_szn( uid, lapin_info, &szn).await?;
              Ok((id, szn))},
            _ => bail!("unexpected update success!"),
          }
        }
        Ok(_) => bail!("unexpected update success!"),
        Err(e) => Err(e)?,
      }
    }
    None => {
      // new note!

      let uuid = uuid::Uuid::new_v4();
      conn.execute(
        "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate, server)
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
          note.title,
          note.content,
          uid.to_i64(),
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          uuid.to_string(),
          createdate,
          changeddate,
          server.id,
        ],
      )?;
      let id = conn.last_insert_rowid();
      let szn = SavedZkNote {
        id: ZkNoteId::Zni(uuid),
        changeddate,
        server: server.uuid.clone(),
        what: note.what.clone(),
      };
      publish_szn(uid, lapin_info, &szn).await?;
      Ok((id, szn))
    }
  }
}

pub fn get_sysids(
  conn: &Connection,
  sysid: UserId,
  noteid: i64,
) -> Result<Vec<ZkNoteId>, rusqlite::Error> {
  let mut pstmt = conn.prepare(
    // return system notes that are linked TO by noteid.
    "select N.uuid
       from zklink A, zknote N
      where
       (A.fromid = ?1 and A.toid = N.id and N.user = ?2)
       order by N.title",
  )?;

  let r = Ok(
    pstmt
      .query_map(params![noteid, sysid.to_i64()], |row| Ok(row.get(0)?))?
      .filter_map(|x| {
        x.ok()
          .and_then(|s: String| Uuid::parse_str(s.as_str()).ok().map(|x| ZkNoteId::Zni(x)))
      })
      .collect(),
  );

  r
}

pub fn read_extra_login_data(
  conn: &Connection,
  id: UserId,
) -> Result<ExtraLoginData, zkerr::Error> {
  let server = local_server_id(conn)?;

  let (uid, noteid, hn) = conn.query_row(
    "select user.id, zknote.uuid, homenote
      from user, zknote where user.id = ?1
      and zknote.id = user.zknote",
    params![id.to_i64()],
    |row| {
      Ok((
        UserId::Uid(row.get(0)?),
        row.get::<usize, String>(1)?,
        row.get::<usize, Option<i64>>(2)?,
      ))
    },
  )?;

  let hnid = match hn {
    Some(i) => Some(uuid_for_note_id(&conn, i)?),
    None => None,
  };
  let eld = ExtraLoginData {
    userid: uid,
    zknote: ZkNoteId::Zni(Uuid::parse_str(noteid.as_str())?),
    homenote: hnid.map(|x| ZkNoteId::Zni(x)),
    server: server.uuid.clone(),
  };

  Ok(eld)
}

pub fn read_zknote_i64(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  id: i64,
) -> Result<ZkNote, zkerr::Error> {
  read_zknote(
    &conn,
    &files_dir,
    uid,
    &ZkNoteId::Zni(uuid_for_note_id(&conn, id)?),
  )
  .map(|x| x.1)
}

pub fn file_status(
  conn: &Connection,
  filedir: &Path,
  file_id: Option<i64>,
) -> Result<FileStatus, zkerr::Error> {
  match file_id {
    Some(fid) => {
      if file_exists(&conn, filedir, fid)? {
        Ok(FileStatus::FilePresent)
      } else {
        Ok(FileStatus::FileMissing)
      }
    }
    None => Ok(FileStatus::NotAFile),
  }
}

// read zknote without any access checking.
pub fn read_zknote_unchecked(
  conn: &Connection,
  filedir: &Path,
  id: &ZkNoteId,
) -> Result<(i64, ZkNote), zkerr::Error> {
  let closure = |row: &Row<'_>| {
    Ok::<_, zkerr::Error>((
      row.get(0)?,
      ZkNote {
        id: ZkNoteId::Zni(Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?),
        title: row.get(2)?,
        content: row.get(3)?,
        user: UserId::Uid(row.get(4)?),
        username: row.get(5)?,
        usernote: ZkNoteId::Zni(Uuid::parse_str(row.get::<usize, String>(6)?.as_str())?),
        pubid: row.get(7)?,
        editable: row.get(8)?,      // editable same as editableValue!
        editableValue: row.get(8)?, // <--- same index.
        showtitle: row.get(9)?,
        deleted: row.get(10)?,
        filestatus: file_status(&conn, &filedir, row.get(11)?)?,
        createdate: row.get(12)?,
        changeddate: row.get(13)?,
        server: row.get(14)?,
        sysids: Vec::new(),
      },
    ))
  };

  conn.query_row_and_then(
        "select ZN.id, ZN.uuid, ZN.title, ZN.content, ZN.user, OU.name, ZKN.uuid,
            ZN.pubid, ZN.editable, ZN.showtitle, ZN.deleted, ZN.file, ZN.createdate, ZN.changeddate, S.uuid
          from zknote ZN, orgauth_user OU, user U, zknote ZKN, server S
          where ZN.uuid = ?1 and U.id = ZN.user and OU.id = ZN.user and ZKN.id = U.zknote and S.id = ZN.server",
          params![id.to_string()],
        closure).map_err(|e| zkerr::annotate_string(format!("note not found: {}", id), e ))
}

pub fn read_zkarch_unchecked(
  conn: &Connection,
  filedir: &Path,
  uuid: &Uuid,
) -> Result<(i64, ZkNote), zkerr::Error> {
  let closure = |row: &Row<'_>| {
    Ok::<_, zkerr::Error>((
      row.get(0)?,
      ZkNote {
        id: ZkNoteId::ArchiveZni(
          Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?,
          Uuid::parse_str(row.get::<usize, String>(2)?.as_str())?,
        ),
        title: row.get(3)?,
        content: row.get(4)?,
        user: UserId::Uid(row.get(5)?),
        username: row.get(6)?,
        usernote: ZkNoteId::Zni(Uuid::parse_str(row.get::<usize, String>(7)?.as_str())?),
        pubid: row.get(8)?,
        editable: row.get(9)?,      // editable same as editableValue!
        editableValue: row.get(9)?, // <--- same index.
        showtitle: row.get(10)?,
        deleted: row.get(11)?,
        filestatus: file_status(&conn, &filedir, row.get(12)?)?,
        createdate: row.get(13)?,
        changeddate: row.get(14)?,
        server: row.get(15)?,
        sysids: Vec::new(),
      },
    ))
  };

  conn.query_row_and_then(
        "select ZN.id, ZN.uuid, ZKP.uuid, ZN.title, ZN.content, ZN.user, OU.name, ZKN.uuid,
            ZN.pubid, ZN.editable, ZN.showtitle, ZN.deleted, ZN.file, ZN.createdate, ZN.changeddate, S.uuid
          from zkarch ZN, orgauth_user OU, user U, zknote ZKN, zknote ZKP, server S
          where ZN.uuid = ?1 and U.id = ZN.user and OU.id = ZN.user and ZKP.id = ZN.zknote and ZKN.id = U.zknote and S.id = ZN.server",
          params![uuid.to_string()],
        closure).map_err(|e| zkerr::annotate_string(format!("archive note not found: {}", uuid), e ))
}

pub fn read_file_info(conn: &Connection, noteid: i64) -> Result<FileInfo, zkerr::Error> {
  let closure = |row: &Row<'_>| {
    Ok::<_, zkerr::Error>(FileInfo {
      hash: row.get(0)?,
      size: row.get(1)?,
    })
  };

  conn
    .query_row_and_then(
      "select F.hash, F.size
          from file F, zknote N
          where N.id = ?1
          and N.file = F.id",
      params![noteid],
      closure,
    )
    .map_err(|e| zkerr::annotate_string(format!("file record not found for note: {}", noteid), e))
}

// 'normal' zknote read with access checking
pub fn read_zknote(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  id: &ZkNoteId,
) -> Result<(i64, ZkNote), zkerr::Error> {
  match id {
    ZkNoteId::Zni(_nid) => {
      let (id, mut note) = read_zknote_unchecked(&conn, &files_dir, id)?;

      let sysid = user_id(&conn, "system")?;
      let sysids = get_sysids(conn, sysid, id)?;

      note.sysids = sysids;

      match zknote_access(conn, uid, id, &note) {
        Ok(zna) => match zna {
          Access::ReadWrite => {
            note.editable = true;
            Ok((id, note))
          }
          Access::Read => {
            note.editable = false;
            Ok((id, note))
          }
          Access::Private => Err(zkerr::Error::NoteIsPrivate),
        },
        Err(e) => Err(e),
      }
    }
    ZkNoteId::ArchiveZni(nid, parentid) => {
      let (pid, parentnote) = read_zknote_unchecked(&conn, &files_dir, &ZkNoteId::Zni(*parentid))?;

      let sysid = user_id(&conn, "system")?;
      let sysids = get_sysids(conn, sysid, pid)?;

      match zknote_access(conn, uid, pid, &parentnote) {
        Ok(zna) => match zna {
          Access::ReadWrite => {
            let (id, mut note) = read_zkarch_unchecked(&conn, &files_dir, nid)?;
            note.sysids = sysids;
            note.editable = true;
            Ok((id, note))
          }
          Access::Read => {
            let (id, mut note) = read_zkarch_unchecked(&conn, &files_dir, nid)?;
            note.sysids = sysids;
            note.editable = false;
            Ok((id, note))
          }
          Access::Private => Err(zkerr::Error::NoteIsPrivate),
        },
        Err(e) => Err(e),
      }
    }
  }
}

pub fn read_zklistnote(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  id: i64,
) -> Result<ZkListNote, zkerr::Error> {
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  // access check
  let zna = zknote_access_id(conn, uid, id)?;
  match zna {
    Access::Private => Err::<_, zkerr::Error>(zkerr::Error::NoteIsPrivate),
    _ => Ok(()),
  }?;

  let note = conn.query_row_and_then(
    "select ZN.uuid, ZN.title, ZN.file, ZN.user, ZN.createdate, ZN.changeddate
      from zknote ZN, orgauth_user OU, user U where ZN.id = ?1 and U.id = ZN.user and OU.id = ZN.user",
    params![id],
    |row| {
      let zln = ZkListNote {
        id:  ZkNoteId::Zni(Uuid::parse_str(row.get::<usize,String>(0)?.as_str())?),
        title: row.get(1)?,
        filestatus: file_status(&conn, &files_dir, row.get(2)?)?,
        user: UserId::Uid(row.get(3)?),
        createdate: row.get(4)?,
        changeddate: row.get(5)?,
        sysids,
      };
      Ok::<_, zkerr::Error>(zln)
    },
  )?;

  Ok(note)
}

pub fn read_zklistarchnote(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  id: i64,
) -> Result<ZkListNote, zkerr::Error> {
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  // access check
  let zna = zknote_access_id(conn, uid, id)?;
  match zna {
    Access::Private => Err::<_, zkerr::Error>(zkerr::Error::NoteIsPrivate),
    _ => Ok(()),
  }?;

  let note = conn.query_row_and_then(
    "select ZN.uuid, ZKP.uuid, ZN.title, ZN.file, ZN.user, ZN.createdate, ZN.changeddate
      from zkarch ZN, zkarch ZKP, orgauth_user OU, user U
      where ZN.id = ?1 and U.id = ZN.user and OU.id = ZN.user and ZKP.id = ZN.zknote",
    params![id],
    |row| {
      let zln = ZkListNote {
        id: ZkNoteId::ArchiveZni(
          Uuid::parse_str(row.get::<usize, String>(0)?.as_str())?,
          Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?,
        ),
        title: row.get(2)?,
        filestatus: file_status(&conn, &files_dir, row.get(3)?)?,
        user: UserId::Uid(row.get(4)?),
        createdate: row.get(5)?,
        changeddate: row.get(6)?,
        sysids,
      };
      Ok::<_, zkerr::Error>(zln)
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
  uid: Option<UserId>,
  id: i64,
  note: &ZkNote,
) -> Result<Access, zkerr::Error> {
  match uid {
    Some(uid) => {
      if uid == note.user {
        Ok(Access::ReadWrite)
      } else if is_zknote_usershared(conn, id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_shared(conn, id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_public(conn, id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        // returns private if not an accessible archive note.
        zknote_archive_access(conn, uid, id)
      }
    }
    None => {
      if is_zknote_public(conn, id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
  }
}

pub fn is_zknote_archive(conn: &Connection, noteid: i64) -> Result<Option<i64>, zkerr::Error> {
  let aid = note_id(&conn, "system", "archive")?;

  match conn.query_row(
    "select A.toid from
      zklink A, zklink B
      where A.fromid = ?1 and B.fromid = ?1 and B.toid = ?2",
    params![noteid, aid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(i) => Ok(Some(i)),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
    Err(e) => Err(zkerr::Error::Rusqlite(e)),
  }
}

pub fn zknote_archive_access(
  conn: &Connection,
  uid: UserId,
  noteid: i64,
) -> Result<Access, zkerr::Error> {
  let id = match is_zknote_archive(&conn, noteid)? {
    Some(id) => id,
    None => return Ok(Access::Private),
  };

  zknote_access_id(conn, Some(uid), id)
}

pub fn zknote_access_id(
  conn: &Connection,
  uid: Option<UserId>,
  noteid: i64,
) -> Result<Access, zkerr::Error> {
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
        // returns private if not an accessible archive.
        zknote_archive_access(conn, uid, noteid)
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
  uid: Option<UserId>,
  noteid: i64,
) -> Result<Option<String>, zkerr::Error> {
  if zknote_access_id(&conn, uid, noteid)? != Access::Private {
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

pub fn file_exists(conn: &Connection, filedir: &Path, file_id: i64) -> Result<bool, zkerr::Error> {
  let hash: String = conn.query_row(
    "select F.hash from file F
      where F.id = ?1",
    params![file_id],
    |row| Ok(row.get(0)?),
  )?;

  Ok(filedir.join(hash).as_path().exists())
}

pub fn read_zknotepubid(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  pubid: &str,
) -> Result<ZkNote, zkerr::Error> {
  let publicid = note_id(&conn, "system", "public")?;
  let (id, mut note) = match conn.query_row_and_then(
    "select A.id, A.uuid, A.title, A.content, A.user, OU.name, ZU.uuid,
        A.pubid, A.editable, A.showtitle, A.deleted, A.file, A.createdate, A.changeddate, S.uuid
      from zknote A, user U, orgauth_user OU, zklink L, server S
      left join zknote ZU on ZU.id = U.zknote
      where A.pubid = ?1
      and ((A.id = L.fromid
      and L.toid = ?2) or (A.id = L.toid
      and L.fromid = ?2))
      and U.id = A.user
      and OU.id = A.user
      and S.id = A.server",
    params![pubid, publicid],
    |row| {
      Ok::<_, zkerr::Error>((
        row.get(0)?,
        ZkNote {
          id: ZkNoteId::Zni(Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?),
          title: row.get(2)?,
          content: row.get(3)?,
          user: UserId::Uid(row.get(4)?),
          username: row.get(5)?,
          usernote: ZkNoteId::Zni(Uuid::parse_str(row.get::<usize, String>(6)?.as_str())?),
          pubid: row.get(7)?,
          editable: false,
          editableValue: row.get(8)?,
          showtitle: row.get(9)?,
          deleted: row.get(10)?,
          filestatus: file_status(&conn, &files_dir, row.get(11)?)?,
          createdate: row.get(12)?,
          changeddate: row.get(13)?,
          server: row.get(14)?,
          sysids: Vec::new(),
        },
      ))
    },
  ) {
    Ok(x) => Ok(x),
    Err(zkerr::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Err(zkerr::Error::String(
      format!("note not found for public id: {}", pubid),
    )),
    Err(e) => Err(e),
  }?;
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  note.sysids = sysids;

  match zknote_access(conn, uid, id, &note) {
    Ok(zna) => match zna {
      Access::ReadWrite => {
        note.editable = true;
        Ok(note)
      }
      Access::Read => {
        note.editable = false;
        Ok(note)
      }
      Access::Private => Err(zkerr::Error::NoteIsPrivate),
    },
    Err(e) => Err(e),
  }
}

pub fn delete_zknote(
  conn: &Connection,
  file_path: PathBuf,
  uid: UserId,
  noteid: &ZkNoteId,
) -> Result<(), zkerr::Error> {
  let nid = note_id_for_zknoteid(&conn, &noteid)?;
  match zknote_access_id(&conn, Some(uid), nid)? {
    Access::ReadWrite => Ok::<_, zkerr::Error>(()),
    _ => Err("can't delete zknote; write permission denied.".into()),
  }?;

  archive_zknote_i64(&conn, nid)?;

  // get file info, if any.
  let filerec: Option<(i64, String)> = match conn.query_row(
    "select F.id, F.hash from zknote N, file F
    where N.uuid = ?1
    and N.file = F.id",
    params![noteid.to_string()],
    |row| Ok((row.get(0)?, row.get(1)?)),
  ) {
    Ok(fr) => Some(fr),
    Err(rusqlite::Error::QueryReturnedNoRows) => None,
    Err(e) => Err(e)?,
  };

  let now = now()?;

  // only delete when user is the owner.
  conn.execute(
    "update zknote set deleted = 1, title = '<deleted>', content = '', file = null, changeddate = ?1
      where uuid = ?2
      and user = ?3",
    params![now, noteid.to_string(), uid.to_i64()],
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
  uid: UserId,
  zklinks: &Vec<SaveZkLink2>,
) -> Result<(), zkerr::Error> {
  let conn = connection_open(dbfile)?;

  for zklink in zklinks.iter() {
    // TODO: integrate into sql instead of separate queries.
    let to = note_id_for_zknoteid(&conn, &zklink.to)?;
    let from = note_id_for_zknoteid(&conn, &zklink.from)?;
    if zklink.delete == Some(true) {
      // if delete, create archive record.
      let now = now()?;
      conn.execute(
        "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
            select fromid, toid, user, linkzknote, createdate, ?1 from zklink
            where fromid = ?2 and toid = ?3 and user = ?4",
        params![now, from, to, uid.to_i64()],
      )?;

      // delete link.
      conn.execute(
        "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
        params![from, to, uid.to_i64()],
      )?;
    } else {
      let lzn = match zklink.linkzknote {
        Some(zknid) => Some(note_id_for_zknoteid(&conn, &zknid)?),
        None => None,
      };

      save_zklink(&conn, from, to, uid, lzn)?;
    }
  }

  Ok(())
}

pub fn save_savezklinks(
  conn: &Connection,
  uid: UserId,
  zknid: ZkNoteId,
  zklinks: &Vec<SaveZkLink>,
) -> Result<(), zkerr::Error> {
  for link in zklinks.iter() {
    let (uufrom, uuto) = match link.direction {
      Direction::From => (link.otherid, zknid),
      Direction::To => (zknid, link.otherid),
    };
    // TODO: integrate into sql instead of separate queries.
    let to = note_id_for_zknoteid(&conn, &uuto)?;
    let from = note_id_for_zknoteid(&conn, &uufrom)?;
    if link.user == uid {
      if link.delete == Some(true) {
        // create archive record.
        let now = now()?;
        conn.execute(
          "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
            select fromid, toid, user, linkzknote, createdate, ?1 from zklink
            where fromid = ?2 and toid = ?3 and user = ?4",
          params![now, from, to, uid.to_i64()],
        )?;
        // delete the link.
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![from, to, uid.to_i64()],
        )?;
      } else {
        let linkzknote = link
          .zknote
          .and_then(|lzn| note_id_for_zknoteid(&conn, &lzn).ok());
        save_zklink(&conn, from, to, uid, linkzknote)?;
      }
    }
  }

  Ok(())
}

pub fn read_zklinks(
  conn: &Connection,
  uid: UserId,
  zknid: i64,
) -> Result<Vec<EditLink>, zkerr::Error> {
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
  // should be ok because the strings are built from vec<i64> returned by user_shares().
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

  // TODO: integrate sysid lookup in the query?
  let sqlstr = format!(
    "select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
      from zklink A
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user = ?1 and (A.fromid = ?2 or A.toid = ?2)
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.fromid = ?2
      and B.fromid = A.toid
      and B.toid = ?3
      union
   select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.toid = ?2
      and B.fromid = A.fromid
      and B.toid = ?3
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
        from zklink A, zklink B
        inner join zknote as L ON A.fromid = L.id
        inner join zknote as R ON A.toid = R.id
        where A.user != ?1 and
          ((A.toid = ?2 and A.fromid = B.fromid and B.toid in ({})) or
           (A.toid = ?2 and A.fromid = B.toid and B.fromid in ({})) or
           (A.fromid = ?2 and A.toid = B.fromid and B.toid in ({})) or
           (A.fromid = ?2 and A.toid = B.toid and B.fromid in ({})))
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
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
  let r = Result::from_iter(pstmt.query_and_then(
    params![uid.to_i64(), zknid, pubid, unid],
    |row| {
      let fromid = row.get(0)?;
      let toid = row.get(1)?;
      let (otherid, otheruuid, othername, direction) = if fromid == zknid {
        (
          toid,
          row.get::<usize, String>(6)?,
          row.get(7)?,
          Direction::To,
        )
      } else {
        (fromid, row.get(4)?, row.get(5)?, Direction::From)
      };
      let sysids = get_sysids(&conn, sysid, otherid)?;
      let zknotei64 = row.get::<usize, Option<i64>>(3)?;
      let zknote = match zknotei64 {
        Some(i) => Some(uuid_for_note_id(&conn, i)?),
        None => None,
      };
      Ok::<_, zkerr::Error>(EditLink {
        otherid: ZkNoteId::Zni(Uuid::parse_str(otheruuid.as_str())?),
        direction,
        user: UserId::Uid(row.get(2)?),
        zknote: zknote.map(|x| ZkNoteId::Zni(x)),
        othername,
        sysids,
        delete: None,
      })
    },
  )?);
  r
}

pub fn read_lzlinks(
  conn: &Connection,
  uid: UserId,
  zknid: i64,
) -> Result<Vec<LzLink>, zkerr::Error> {
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
  // should be ok because the strings are built from vec<i64> returned by user_shares().
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
  //
  // lzlinks that are visible:
  //
  // lzlinks where both notes are visible.
  //
  // note is visible when any of:
  //   it is mine
  //   ISA public
  //   ISA usershare
  //   links to my usernote.

  // // TODO: integrate sysid lookup in the query?
  // let sqlstr = format!(
  //   "select A.user, L.uuid, L.title, R.uuid, R.title
  //     from zklink A, zklink B
  //     inner join zknote as L ON A.fromid = L.id
  //     inner join zknote as R ON A.toid = R.id
  //     where A.linkzknote = ?2
  //     and
  //       -- FROM is visible??  one of:
  //      (
  //       -- from is mine
  //       L.user = ?1 or
  //       -- from is public
  //       (B.fromid = A.fromid and B.toid = ?3) or
  //        -- from links to usershare
  //       ((A.fromid = B.fromid and B.toid in ({})) or
  //        (A.fromid = B.toid and B.fromid in ({})))
  //       -- from links to usernote
  //       (A.fromid == B.fromid and B.toid = ?4)
  //      )
  //      and
  //       -- TO is visible??  one of:
  //      (
  //       -- to is mine
  //       L.user = ?1 or
  //       -- to is public
  //       (A.toid = B.fromid and B.toid = ?3) or
  //        -- to links to usershare
  //       ((A.toid = B.fromid and B.toid in ({})) or
  //        (A.toid = B.toid and B.fromid in ({})))
  //       -- to links to usernote
  //       (A.toid == B.fromid and B.toid = ?4)
  //      )
  //      -- only return one copy of each
  //      group by A.user, L.uuid, R.uuid
  //     ",
  //   s, s, s, s
  // );

  // TODO: integrate sysid lookup in the query?
  let sqlstr = format!(
    "select A.user, L.uuid, L.title, R.uuid, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.linkzknote = ?2
      and
       (
        (L.user = 2 and A.fromid = B.fromid and A.toid = B.toid and A.user = B.user) or
        (B.fromid = A.fromid and B.toid = ?3) or
        ((A.fromid = B.fromid and B.toid in ({})) or
         (A.fromid = B.toid and B.fromid in ({}))) or
        (A.fromid == B.fromid and B.toid = ?4)
       )
       and
       (
        -- comment
        (R.user = 2 and A.fromid = B.fromid and A.toid = B.toid and A.user = B.user) or
        (A.toid = B.fromid and B.toid = ?3) or
        ((A.toid = B.fromid and B.toid in ({})) or
         (A.toid = B.toid and B.fromid in ({}))) or
        (A.toid == B.fromid and B.toid = ?4)
       )
      ",
    s, s, s, s
  );

  println!("lzquery: {}", sqlstr);

  println!("lzargs: {:?}", (uid.to_i64(), zknid, pubid, unid));

  let mut pstmt = conn.prepare(sqlstr.as_str())?;
  let r = Result::from_iter(
    pstmt
      .query_and_then(params![uid.to_i64(), zknid, pubid, unid], |row| {
        Ok((
          row.get::<usize, String>(1)?,
          row.get::<usize, String>(3)?,
          row.get::<usize, i64>(0)?,
          row.get::<usize, String>(2)?,
          row.get::<usize, String>(4)?,
        ))
      })?
      .map(|x| match x {
        Ok((lid, rid, u, lname, rname)) => Ok(LzLink {
          from: ZkNoteId::Zni(Uuid::parse_str(lid.as_str())?),
          to: ZkNoteId::Zni(Uuid::parse_str(rid.as_str())?),
          user: UserId::Uid(u),
          fromname: lname,
          toname: rname,
        }),
        Err(a) => Err(a),
      }),
  );

  r
}

pub fn read_public_zklinks(
  conn: &Connection,
  noteid: &ZkNoteId,
) -> Result<Vec<EditLink>, zkerr::Error> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;
  let zknid = note_id_for_zknoteid(&conn, noteid)?;

  // TODO: integrate sysid lookup in the query?
  let mut pstmt = conn.prepare(
    // return zklinks that link to or from notes that link to 'public'.
    "select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
       from zklink A, zklink B
       inner join zknote as L ON A.fromid = L.id
       inner join zknote as R ON A.toid = R.id
     where
       (L.user != ?3 and R.user != ?3) and
       ((A.toid = ?1 and A.fromid = B.fromid and B.toid = ?2) or
        (A.fromid = ?1 and A.toid = B.toid and B.fromid = ?2) or
        (A.fromid = ?1 and A.toid = B.fromid and B.toid = ?2))",
  )?;

  let r = Result::from_iter(pstmt.query_and_then(
    params![zknid, pubid, sysid.to_i64()],
    |row| {
      let fromid: i64 = row.get(0)?;
      let toid: i64 = row.get(1)?;
      let (otherid, direction, otheruuid, othername) = if fromid == zknid {
        (
          toid,
          Direction::To,
          Uuid::parse_str(row.get::<usize, String>(6)?.as_str()).map_err(|e| {
            zkerr::annotate_string(
              format!("error parsing link uuid: {:?}", row.get::<usize, String>(6)),
              e.into(),
            )
          })?,
          row.get(7)?,
        )
      } else {
        (
          fromid,
          Direction::From,
          Uuid::parse_str(row.get::<usize, String>(4)?.as_str()).map_err(|e| {
            zkerr::annotate_string(
              format!("error parsing link uuid: {:?}", row.get::<usize, String>(4)),
              e.into(),
            )
          })?,
          row.get(5)?,
        )
      };

      let zknotei64 = row.get::<usize, Option<i64>>(3)?;
      let zknote = match zknotei64 {
        Some(i) => Some(uuid_for_note_id(&conn, i)?),
        None => None,
      };

      Ok::<_, zkerr::Error>(EditLink {
        otherid: ZkNoteId::Zni(otheruuid),
        direction,
        user: UserId::Uid(row.get(2)?),
        zknote: zknote.map(|x| ZkNoteId::Zni(x)),
        othername,
        sysids: get_sysids(&conn, sysid, otherid)?,
        delete: None,
      })
    },
  )?);

  r
}

pub fn read_public_lzlinks(
  conn: &Connection,
  noteid: &ZkNoteId,
) -> Result<Vec<LzLink>, zkerr::Error> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;
  let zknid = note_id_for_zknoteid(&conn, noteid)?;

  // TODO: integrate sysid lookup in the query?
  let mut pstmt = conn.prepare(
    // return zklinks with linkzknote = noteid, and
    // that link to or from notes that link to 'public'.
    "select A.fromid, A.toid, A.user, A.linkzknote, L.uuid, L.title, R.uuid, R.title
       from zklink A, zklink B, zklink C
       inner join zknote as L ON A.fromid = L.id
       inner join zknote as R ON A.toid = R.id
     where
       (L.user != ?3 and R.user != ?3) and
       (A.linkzknote = ?1 and
       (A.fromid = B.fromid and B.toid = ?2 or
        A.fromid = B.toid and B.fromid = ?2) and
       (A.toid = C.fromid and C.toid = ?2 or
        A.toid = C.toid and C.fromid = ?2))",
  )?;

  let r = Result::from_iter(pstmt.query_and_then(
    params![zknid, pubid, sysid.to_i64()],
    |row| {
      // let fromid: i64 = row.get(0)?;
      // let toid: i64 = row.get(1)?;
      let fromuuid = Uuid::parse_str(row.get::<usize, String>(4)?.as_str()).map_err(|e| {
        zkerr::annotate_string(
          format!("error parsing link uuid: {:?}", row.get::<usize, String>(4)),
          e.into(),
        )
      })?;
      let fromtitle = row.get::<usize, String>(5)?;

      let touuid = Uuid::parse_str(row.get::<usize, String>(6)?.as_str()).map_err(|e| {
        zkerr::annotate_string(
          format!("error parsing link uuid: {:?}", row.get::<usize, String>(6)),
          e.into(),
        )
      })?;
      let totitle = row.get::<usize, String>(7)?;

      // For test!
      let linkzknotei64 = row.get::<usize, i64>(3)?;
      assert!(linkzknotei64 == zknid);

      Ok::<_, zkerr::Error>(LzLink {
        from: ZkNoteId::Zni(fromuuid),
        to: ZkNoteId::Zni(touuid),
        user: UserId::Uid(row.get(2)?), // really?  i64?
        fromname: fromtitle,
        toname: totitle,
      })
    },
  )?);

  r
}

pub fn read_zknotecomments(
  conn: &Connection,
  files_dir: &Path,
  uid: UserId,
  gznc: &GetZkNoteComments,
) -> Result<Vec<ZkNote>, zkerr::Error> {
  let cid = note_id(&conn, "system", "comment")?;

  let zknid = note_id_for_zknoteid(&conn, &gznc.zknote)?;
  // notes with a TO link to our note
  // and a TO link to 'comment'
  let mut stmt = conn.prepare(
    "select N.fromid from  zklink C, zklink N
      where N.fromid = C.fromid
      and N.toid = ?1 and C.toid = ?2",
  )?;
  // TODO: return uuid here so we don't have to look it up in
  // read_zknote_i64.
  let c_iter = stmt.query_map(params![zknid, cid], |row| Ok(row.get(0)?))?;

  let mut nv = Vec::new();

  for id in c_iter {
    match id {
      Ok(id) => match read_zknote_i64(&conn, files_dir, Some(uid), id) {
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
  files_dir: &Path,
  uid: UserId,
  gzna: &GetZkNoteArchives,
) -> Result<Vec<ZkListNote>, zkerr::Error> {
  let sysid = user_id(&conn, "system")?;
  let id = note_id_for_zknoteid(conn, &gzna.zknote)?;
  let sysids = get_sysids(conn, sysid, id)?;

  // users that can't see a note, can't see the archives either.
  let zna = zknote_access_id(conn, Some(uid), id)?;
  match zna {
    Access::Private => Err::<_, zkerr::Error>(zkerr::Error::NoteIsPrivate),
    _ => Ok(()),
  }?;

  // read from zkarch table, the archive notes table.
  let mut stmt = conn.prepare(
    "select ZN.uuid, ZN.title, ZN.file, ZN.user, ZN.createdate, ZN.changeddate
      from zkarch ZN, orgauth_user OU, user U
      where ZN.zknote = ?1 and U.id = ZN.user and OU.id = ZN.user
      order by ZN.changeddate desc",
  )?;

  let c_iter = stmt
    .query_map(params![id], |row| {
      Ok((
        row.get::<usize, String>(0)?,
        row.get::<usize, String>(1)?,
        row.get::<usize, Option<i64>>(2)?,
        row.get::<usize, i64>(3)?,
        row.get::<usize, i64>(4)?,
        row.get::<usize, i64>(5)?,
      ))
    })?
    .skip(gzna.offset as usize);

  let mut nv = Vec::new();

  for x in c_iter {
    match x {
      Ok((v0, v1, v2, v3, v4, v5)) => {
        let zln = ZkListNote {
          id: ZkNoteId::Zni(Uuid::parse_str(v0.as_str())?),
          title: v1,
          filestatus: file_status(&conn, &files_dir, v2)?,
          user: UserId::Uid(v3),
          createdate: v4,
          changeddate: v5,
          sysids: sysids.clone(),
        };
        nv.push(zln);
        match gzna.limit {
          Some(l) => {
            if nv.len() >= l as usize {
              break;
            }
          }
          None => (),
        }
      }
      Err(e) => return Err(e.into()),
    }
  }

  Ok(nv)
}

pub fn read_archivezklinks(
  conn: &Connection,
  uid: UserId,
  after: Option<i64>,
) -> Result<Vec<ArchiveZkLink>, zkerr::Error> {
  let (acc_sql, mut acc_args) = accessible_notes(&conn, uid)?;

  let mut pstmt = conn.prepare(
    format!(
      "with accessible_notes as ({})
      select ZLA.user, FN.uuid, TN.uuid, LN.uuid, ZLA.createdate, ZLA.deletedate
      from zklinkarchive ZLA, zknote FN, zknote TN
      left join zknote LN on LN.id = ZLA.linkzknote
      where FN.id = ZLA.fromid
      and TN.id = ZLA.toid
      and ZLA.fromid in accessible_notes
      and ZLA.toid in accessible_notes
      {}",
      acc_sql,
      if after.is_some() {
        " and unlikely(ZLA.deletedate > ? or ZLA.createdate > ?)"
      } else {
        ""
      }
    )
    .as_str(),
  )?;

  if let Some(a64) = after {
    let a = a64.to_string();
    let mut av = vec![a.clone(), a.clone(), a.clone()];
    acc_args.append(&mut av);
  }

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
  uid: UserId,
  after: Option<i64>,
  exclude_archivelinks: Option<String>,
) -> impl futures_util::Stream<Item = Result<SyncMessage, Box<dyn std::error::Error>>> {
  // {
  try_stream! {
    let (acc_sql, mut acc_args) = accessible_notes(&conn, uid)?;

    let mut pstmt = conn.prepare(
      format!(
        "with accessible_notes as ({})
        select OU.uuid, FN.uuid, TN.uuid, LN.uuid, ZLA.createdate, ZLA.deletedate
        from zklinkarchive ZLA, zknote FN, zknote TN, orgauth_user OU
        left join zknote LN on LN.id = ZLA.linkzknote
        where FN.id = ZLA.fromid
        and TN.id = ZLA.toid
        and ZLA.user = OU.id
        and ZLA.fromid in accessible_notes
        and ZLA.toid in accessible_notes
        {}
        {}",
        acc_sql,
        if after.is_some() {
          " and unlikely(ZLA.deletedate > ? or ZLA.createdate > ?)"
        } else {
          ""
        },
        if let Some(el) = exclude_archivelinks {
          format!("and ZLA.id not in (select id from {})", el)
        }
        else { "".to_string()
        }

      )
      .as_str(),
    )?;

    if let Some(a64) = after {
      let a = a64.to_string();
      acc_args.push(a.clone());
      acc_args.push(a);
    }
    let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
      let azl = ArchiveZkLink {
        userUuid: row.get(0)?,
        fromUuid: row.get(1)?,
        toUuid: row.get(2)?,
        linkUuid: row.get(3)?,
        createdate: row.get(4)?,
        deletedate: row.get(5)?,
      };
      Ok(azl)
    })?;

    yield SyncMessage::ArchiveZkLinkHeader;

    for rec in rec_iter {
      if let Ok(r) = rec {
        yield SyncMessage::from(r);
      }
    }
  }
}

pub fn read_zklinks_since(
  conn: &Connection,
  uid: UserId,
  after: Option<i64>,
) -> Result<Vec<UuidZkLink>, zkerr::Error> {
  let (acc_sql, mut acc_args) = accessible_notes(&conn, uid)?;

  let mut pstmt = conn.prepare(
    format!(
      "with accessible_notes as ({})
        select OU.uuid, FN.uuid, TN.uuid, LN.uuid, ZL.createdate
        from zklink ZL, zknote FN, zknote TN, orgauth_user OU
        left join zknote LN
        on LN.id = ZL.linkzknote
        where FN.id = ZL.fromid
        and TN.id = ZL.toid
        and ZL.user = OU.id
        and ZL.fromid in accessible_notes
        and ZL.toid in accessible_notes
      {}",
      acc_sql,
      if after.is_some() {
        " and unlikely(ZL.createdate > ?)"
      } else {
        ""
      }
    )
    .as_str(),
  )?;

  if let Some(a64) = after {
    let a = a64.to_string();
    acc_args.push(a);
  }

  let rec_iter = pstmt.query_map(rusqlite::params_from_iter(acc_args.iter()), |row| {
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
  uid: UserId,
  after: Option<i64>,
  exclude_links: Option<String>,
) -> impl futures_util::Stream<Item = Result<SyncMessage, Box<dyn std::error::Error>>> {
  // {
  try_stream! {
    let (acc_sql, acc_args) = accessible_notes(&conn, uid)?;

    // make an accessible notes temp table.

    // add links for archive notes.
    // archive notes being:
    // notes that link to accessible notes, with links that have linknoteid = 'archive', and are owned by 'sysid'.
    // all links to such notes?
    // or, just the ones to 'archive' and to 'accessible notes' with linknoteid = 'archive'.

    // TODO: unique id for this temp table.
    let tabname = format!("accnotes_{}", uid);

    conn.execute(
      format!(
        "create temporary table if not exists {} (\"id\" integer primary key not null)",
        tabname
      )
      .as_str(),
      params![],
    )?;
    // in case the temp table exists, clear it out  (TODO give it a unique name just for this sync process?)
    conn.execute(format!("delete from {}", tabname).as_str(), params![])?;
    conn.execute(
      format!("insert into {} {}", tabname, acc_sql).as_str(),
      rusqlite::params_from_iter(acc_args.iter()),
    )?;

    // links.  Fact:  these records are only created when a link is deleted!
    // so the createdate is the date the original link was created, and the deletedate is the date the
    // zklinkarchive record was created.
    {
      let pstmt1 = conn.prepare(
        format!(
          "select OU.uuid, FN.uuid, TN.uuid, LN.uuid, ZL.createdate
          from zklink ZL, zknote FN, zknote TN, orgauth_user OU, {} FW, {} TW
          left join zknote LN on LN.id = ZL.linkzknote
          where FN.id = ZL.fromid
          and TN.id = ZL.toid
          and ZL.user = OU.id
          and ZL.fromid = FW.id
          and ZL.toid = TW.id
          {}
          {} ",
          tabname,
          tabname,
          if after.is_some() {
            " and unlikely(ZL.createdate > ?)"
          } else {
            ""
          },
          if let Some(el) = exclude_links {
            format!(" and (ZL.fromid, ZL.toid, ZL.user) not in (select fromid, toid, user from {})", el)
          }
        else { "".to_string()
        }
        )
        .as_str(),
      );

      let mut pstmt = pstmt1?;

      let mut lnargs = Vec::new();
      if let Some(a64) = after {
        lnargs.push(a64.to_string());
      }

      yield SyncMessage::UuidZkLinkHeader;

      let rec_iter = pstmt.query_map(rusqlite::params_from_iter(lnargs.iter()), |row| {
        Ok(SyncMessage::from(UuidZkLink {
          userUuid: row.get(0)?,
          fromUuid: row.get(1)?,
          toUuid: row.get(2)?,
          linkUuid: row.get(3)?,
          createdate: row.get(4)?,
        }))
      })?;
      for rec in rec_iter {
        if let Ok(r) = rec {
          yield r;
        }
      }
    }

    conn.execute(format!("drop table {}", tabname).as_str(), params![])?;
  }
}

pub fn accessible_notes(
  conn: &Connection,
  uid: UserId,
) -> Result<(String, Vec<String>), zkerr::Error> {
  let publicid = note_id(&conn, "system", "public")?;
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = user_note_id(&conn, uid)?;

  // query: archivelinks that attach to notes I can access.
  // my notes + public notes + shared notes + ??

  // notes that are mine.
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
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))",
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
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

  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqlpub.as_str());
  baseargs.append(&mut pubargs);

  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqlshare.as_str());
  baseargs.append(&mut shareargs);

  sqlbase.push_str("\nunion ");
  sqlbase.push_str(sqluser.as_str());
  baseargs.append(&mut userargs);

  sqlbase.push_str("\n order by id");

  Ok((sqlbase, baseargs))
}

pub fn read_zknoteandlinks(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  zknoteid: &ZkNoteId,
) -> Result<ZkNoteAndLinks, zkerr::Error> {
  // should do an ownership check for us
  let (id, zknote) = read_zknote(conn, files_dir, uid, zknoteid)?;

  let links = match uid {
    Some(uid) => read_zklinks(conn, uid, id)?,
    None => read_public_zklinks(conn, &zknote.id)?,
  };

  let lzlinks = match uid {
    Some(uid) => read_lzlinks(conn, uid, id)?,
    None => read_public_lzlinks(conn, &zknote.id)?,
  };

  Ok(ZkNoteAndLinks {
    zknote,
    links,
    lzlinks,
  })
}

pub fn read_zneifchanged(
  conn: &Connection,
  files_dir: &Path,
  uid: Option<UserId>,
  gzic: &GetZnlIfChanged,
) -> Result<Option<ZkNoteAndLinksWhat>, zkerr::Error> {
  let id = note_id_for_zknoteid(&conn, &gzic.zknote)?;
  let changeddate: i64 = conn.query_row(
    "select changeddate from zknote N
      where N.id = ?1",
    params![id],
    |row| Ok(row.get(0)?),
  )?;

  if changeddate > gzic.changeddate {
    return read_zknoteandlinks(conn, files_dir, uid, &gzic.zknote)
      .map(Some)
      .map(|opt| {
        opt.map(|znl| ZkNoteAndLinksWhat {
          what: gzic.what.clone(),
          edittab: gzic.edittab.clone(),
          znl,
        })
      });
  } else {
    Ok(None)
  }
}

pub async fn save_importzknotes(
  conn: &Connection,
  lapin_info: &Option<LapinInfo>,
  server: &Server,
  uid: UserId,
  izns: &Vec<ImportZkNote>,
) -> Result<(), zkerr::Error> {
  for izn in izns.iter() {
    // create the note if it doesn't exist.
    let nid = match note_id2(&conn, uid, izn.title.as_str())? {
      Some(i) => {
        // update the content.
        conn.execute(
          "update zknote set content = ?1 where
            user = ?2 and id = ?3",
          params![izn.content, uid.to_i64(), i],
        )?;

        i
      }
      None => {
        // new note.
        save_zknote(
          &conn,
          lapin_info,
          server,
          uid,
          &SaveZkNote {
            id: None,
            title: izn.title.clone(),
            pubid: None,
            content: izn.content.clone(),
            editable: false,
            showtitle: true,
            deleted: false,
            what: None,
          },
          None,
        )
        .await?
        .0
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
            lapin_info,
            server,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
              deleted: false,
              what: None,
            },
            None,
          )
          .await?
          .0
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
            lapin_info,
            server,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
              deleted: false,
              what: None,
            },
            None,
          )
          .await?
          .0
        }
      };

      // save link.
      save_zklink(&conn, nid, toid, uid, None)?;
    }
  }

  Ok(())
}

pub async fn make_file_note(
  conn: &Connection,
  server: &Server,
  lapin_info: &Option<LapinInfo>,
  files_dir: &Path,
  uid: UserId,
  name: &String,
  fpath: &Path,
  copy: bool,
) -> Result<(i64, ZkNoteId, i64), zkerr::Error> {
  // compute hash.
  let fh = sha256::try_digest(fpath)?;
  let size = std::fs::metadata(fpath)?.len();
  let hashpath = files_dir.join(Path::new(fh.as_str()));

  let mut existed = false;

  // file exists?
  if hashpath.exists() {
    existed = true;
    // file already exists.  don't need the new one.
    std::fs::remove_file(fpath)?;
  } else {
    if copy {
      // move into hashed-files dir.
      std::fs::copy(fpath, hashpath)?;
    } else {
      // move into hashed-files dir.
      std::fs::rename(fpath, hashpath)?;
    }
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

  let send_on_make_file_node = async |zni: ZkNoteId, title: &str| {
    if let Some(li) = lapin_info {
      let oszn = OnMakeFileNote {
        id: zni,
        user: uid,
        token: li.token.clone(),
        title: title.to_string(),
      };
      // send the message.
      match li
        .channel
        .basic_publish(
          "",
          "on_make_file_note",
          lapin::options::BasicPublishOptions::default(),
          &serde_json::to_vec(&oszn)?[..],
          lapin::BasicProperties::default(),
        )
        .await
      {
        Ok(_) => info!("OnMakeFileNote published to amqp"),
        Err(e) => error!("error publishing to AMQP: {:?}", e),
      }
    }
    Ok::<(), zkerr::Error>(())
  };

  // use existing file.id, or create new
  let fid = match oid {
    Some(fid) => {
      // note exists too, for this user?
      match conn.query_row_and_then(
        "select id, uuid, title from zknote where file = ?1 and user = ?2",
        params![fid, uid.to_i64()],
        |row| {
          Ok((
            row.get(0)?,
            row.get::<usize, String>(1)?,
            row.get::<usize, String>(2)?,
          ))
        },
      ) {
        Ok((id, uuid, title)) => {
          let zni = ZkNoteId::Zni(Uuid::parse_str(uuid.as_str())?);
          if !existed {
            send_on_make_file_node(zni, title.as_str()).await?;
          }
          return Ok((id, zni, fid));
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => fid,
        Err(e) => Err(e)?,
      }
    }
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

  // make a new note.
  let (id, sn) = save_zknote(
    &conn,
    lapin_info,
    server,
    uid,
    &SaveZkNote {
      id: None,
      title: name.to_string(),
      pubid: None,
      content: "".to_string(),
      editable: false,
      showtitle: false,
      deleted: false,
      what: None,
    },
    None,
  )
  .await?;

  // set the file id in that note.
  set_zknote_file(&conn, id, fid)?;

  send_on_make_file_node(sn.id, name).await?;

  Ok((id, sn.id, fid))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZkDatabase {
  notes: Vec<ZkNote>,
  links: Vec<ZkLink>,
  users: Vec<ExtraLoginData>,
}

pub fn export_db(_dbfile: &Path) -> Result<ZkDatabase, zkerr::Error> {
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
