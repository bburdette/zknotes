use crate::search::{search_zknotes, search_zknotes_stream, sync_users, system_user, SearchResult};
use crate::util::now;
use actix_web::body::None;
use actix_web::error::PayloadError;
use actix_web::web::Payload;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use async_stream::__private::AsyncStream;
use async_stream::try_stream;
use barrel::Table;
use bytes::Bytes;
use core::pin::{pin, Pin};
use futures::future;
use futures::Stream;
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use log::{error, info};
use std::convert::TryFrom;
use std::thread::spawn;
use std::time::Duration;
// use json::JsonResult;
use crate::error as zkerr;
use crate::sqldata::{self, note_id_for_uuid, save_zklink, save_zknote, user_note_id};
use orgauth;
use orgauth::data::{PhantomUser, User, UserResponse, UserResponseMessage};
use orgauth::dbfun::user_id;
use orgauth::endpoints::Callbacks;
use reqwest;
// use reqwest::{Body, cookie};
use awc;
use rusqlite::{params, Connection, Transaction};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{mpsc, Arc};
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;
use uuid::Uuid;
use zkprotocol::constants::{
  PrivateReplies, PrivateRequests, PrivateStreamingRequests, SpecialUuids,
};
use zkprotocol::content::{
  ArchiveZkLink, GetArchiveZkLinks, GetZkLinksSince, SaveZkNote, SyncMessage, SyncSince,
  UuidZkLink, ZkNote, ZkNoteId, ZkPhantomUser,
};
use zkprotocol::messages::{PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage};
use zkprotocol::search::{
  AndOr, OrderDirection, OrderField, Ordering, ResultType, SearchMod, TagSearch, ZkNoteSearch,
};

fn convert_err(err: reqwest::Error) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

fn convert_payloaderr(err: PayloadError) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CompletedSync {
  after: Option<i64>,
  now: i64,
}

pub async fn prev_sync(
  conn: &Connection,
  user: &User,
  usernoteid: &ZkNoteId,
) -> Result<Option<CompletedSync>, zkerr::Error> {
  let sysid = user_id(&conn, "system")?;
  let zns = ZkNoteSearch {
    tagsearch: TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::Tag, SearchMod::ZkNoteId],
        term: SpecialUuids::Sync.str().to_string(),
      }),
      ao: zkprotocol::search::AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::Tag, SearchMod::ZkNoteId],
        term: usernoteid.to_string(),
      }),
    },
    offset: 0,
    limit: Some(1),
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: false,
    created_after: None,
    created_before: None,
    changed_after: None,
    changed_before: None,
    ordering: Some(Ordering {
      field: OrderField::Changed,
      direction: OrderDirection::Descending,
    }),
  };

  if let SearchResult::SrNote(res) = search_zknotes(conn, sysid, &zns)? {
    println!("result noates: {:?}", res.notes);
    match res.notes.first() {
      Some(n) => match serde_json::from_str::<CompletedSync>(n.content.as_str()) {
        Ok(s) => Ok(Some(s)),
        Err(e) => {
          error!("CompletedSync parse error: {:?}", e);
          Ok(None)
        }
      },
      None => Ok(None),
    }
  } else {
    Err("unexpected search result type".into())
  }
}

pub async fn save_sync(
  conn: &Connection,
  uid: i64,
  usernoteid: i64,
  sync: CompletedSync,
) -> Result<i64, Box<dyn std::error::Error>> {
  let syncid = note_id_for_uuid(conn, &Uuid::parse_str(SpecialUuids::Sync.str())?)?;
  let sysid = user_id(&conn, "system")?;

  let (id, _szn) = save_zknote(
    conn,
    sysid, // save under system id.
    &SaveZkNote {
      id: None,
      title: "sync".to_string(),
      pubid: None,
      content: serde_json::to_string_pretty(&serde_json::to_value(sync)?)?,
      editable: false,
      showtitle: false,
      deleted: false,
    },
  )?;

  // link to 'sync' system note.
  save_zklink(conn, id, syncid, sysid, None)?;

  // link to user note for access
  save_zklink(conn, id, usernoteid, sysid, None)?;

  Ok(id)
}

pub async fn sync_from_remote_prev(
  conn: &Connection,
  user: &User,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let now = now()?;

  let extra_login_data = sqldata::read_user_by_id(conn, user.id)?;

  // get previous sync.
  let after = prev_sync(&conn, &user, &extra_login_data.zknote)
    .await?
    .map(|cs| cs.now);

  println!("\n\n start sync, prev_sync {:?} \n\n", after);

  // if after.is_none() {
  //   return Err("is none".into());
  // }

  // TODO:  get previous sync information!

  // execute any command from here.  search?
  match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => {
      let url = reqwest::Url::parse(url.as_str())?;

      let jar = Arc::new(reqwest::cookie::Jar::default());
      jar.add_cookie_str(c.as_str(), &url);
      let client = reqwest::Client::builder().cookie_provider(jar).build()?;

      let getnotes = true;
      let getlinks = true;
      let getarchivenotes = true;
      let getarchivelinks = true;

      // TODO: get time on remote system, bail if too far out.

      if getnotes {
        let mut userhash = HashMap::<i64, i64>::new();

        // TODO: get recs with sync date newer than.
        // TODO: order by?
        let zns = ZkNoteSearch {
          tagsearch: TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "".to_string(),
          },
          offset: 0,
          limit: None,
          what: "".to_string(),
          resulttype: ResultType::RtNote,
          archives: false,
          deleted: false,
          created_after: None,
          created_before: None,
          changed_after: after,
          changed_before: None,
          ordering: None,
        };

        let l = PrivateMessage {
          what: PrivateRequests::SearchZkNotes,
          data: Some(serde_json::to_value(zns)?),
        };

        let url = reqwest::Url::parse(
          format!("{}/stream", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(url.clone()).json(&l).send().await?;

        // handle streaming response!

        // let resp = res.json::<ServerResponse>().await?;
        let rstream = res.bytes_stream().map_err(convert_err);

        let mut br = StreamReader::new(rstream);

        let mut line = String::new();
        let nc = br.read_line(&mut line).await?;

        if nc == 0 {
          return Err("empty stream!".into());
        }

        println!("line: {}", line);

        let sr = serde_json::from_str::<PrivateReplyMessage>(line.trim())?;

        if sr.what != PrivateReplies::ZkNoteSearchResult {
          return Err(format!("unexpected what {:?}", sr.what).into());
        }

        // map of remote user ids to local user ids.
        let url =
          reqwest::Url::parse(format!("{}/user", url.origin().unicode_serialization(),).as_str())?;

        /*
        // this code streamed 10232 records in about 4 secs.

        let mut count = 0;
        let mut bytes = 0;

        println!("first message! {:?}", sr);
        loop {
          line.clear();
          let nc = br.read_line(&mut line).await?;

          if nc == 0 {
            break;
          }

          bytes = bytes + nc;
          count = count + 1;
          println!("note line: {}", line);

          let zn = serde_json::from_str::<ZkNote>(line.trim())?;

          println!("zklistnote: {:?}", zn);
        }

        println!("synched {} records, with {} bytes!", count, bytes);
        */

        // TODO: speed this up!  WAAAY slower than just downloading the records.
        loop {
          line.clear();
          let nc = br.read_line(&mut line).await?;

          if nc == 0 {
            break;
          }

          // println!("note line: {}", line);

          let note = serde_json::from_str::<ZkNote>(line.trim())?;

          println!("zknote: {:?}", note);
          // got this user already?
          // If not, make a phantom user.
          let user_uid: i64 = match userhash.get(&note.user) {
            Some(u) => *u,
            None => {
              println!("fetching remote user: {:?}", note.user);
              // fetch a remote user record.
              let res = client
                .post(url.clone())
                .json(&orgauth::data::UserRequestMessage {
                  what: orgauth::data::UserRequest::ReadRemoteUser,
                  data: Some(serde_json::to_value(note.user)?),
                })
                .send()
                .await?;
              let wm: UserResponseMessage = serde_json::from_value(res.json().await?)?;
              println!("remote user wm: {:?}", wm);
              let pu: PhantomUser = match wm.what {
                UserResponse::RemoteUser => serde_json::from_value(
                  wm.data
                    .ok_or::<orgauth::error::Error>("missing data".into())?,
                )?, // .map_err(|e| e.into())?,
                _ => Err::<PhantomUser, Box<dyn std::error::Error>>(
                  orgauth::error::Error::String(format!("unexpected message: {:?}", wm)).into(),
                )?,
              };
              println!("phantom user: {:?}", pu);
              let localuserid = match orgauth::dbfun::read_user_by_uuid(&conn, &pu.uuid) {
                Ok(user) => {
                  println!("found local user {} for remote {}", user.id, pu.id);
                  userhash.insert(pu.id, user.id);
                  user.id
                }
                _ => {
                  let localpuid = orgauth::dbfun::phantom_user(
                    &conn,
                    &pu.name,
                    pu.uuid,
                    pu.active,
                    &mut callbacks.on_new_user,
                  )?;
                  println!(
                    "creating phantom user {} for remote user: {:?}",
                    pu.id, localpuid
                  );
                  userhash.insert(pu.id, localpuid);
                  localpuid
                }
              };
              localuserid
            }
          };

          // Syncing a remote note.
          match conn.execute(
              "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate)
               values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
              params![
                note.title,
                note.content,
                user_uid,
                note.pubid,
                note.editable,
                note.showtitle,
                note.deleted,
                note.id.to_string(),
                note.createdate,
                note.changeddate,
              ],
            )
            {
              Ok(x) => Ok(x),
              Err(rusqlite::Error::SqliteFailure(e, s)) =>
                if e.code == rusqlite::ErrorCode::ConstraintViolation {
                  let (nid, n) = sqldata::read_zknote_unchecked(&conn, &note.id)?;
                  // TODO: uuid conflict;  resolve with older one becoming archive note.
                  // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
                  if note.changeddate > n.changeddate {
                    // note is newer.  archive the old and replace.
                    sqldata::save_zknote(&conn,
                                         user_uid,
                                         &SaveZkNote {
                                           id: Some(note.id),
                                           title: note.title,
                                           pubid: note.pubid,
                                           content: note.content,
                                           editable: note.editable,
                                           showtitle: note.showtitle,
                                           deleted: note.deleted,
                                         })?;
                  } else {
                    // note is older.  add as archive note.
                    // may create duplicate archive notes if edited on two systems and then synced.
                    sqldata::archive_zknote(&conn, nid, &note)?;
                  }
                  Ok(1)
                } else {
                  Err(rusqlite::Error::SqliteFailure(e, s))
                }
              Err(e) => Err(e),
            }?;
        }
      }
      if getarchivenotes {
        println!("reading archive notes");
        let zns = ZkNoteSearch {
          tagsearch: TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "".to_string(),
          },
          offset: 0,
          limit: None,
          what: "".to_string(),
          resulttype: ResultType::RtNote,
          archives: true,
          deleted: false,
          created_after: None,
          created_before: None,
          changed_after: after,
          changed_before: None,
          ordering: None,
        };

        let l = PrivateStreamingMessage {
          what: PrivateStreamingRequests::SearchZkNotes,
          data: Some(serde_json::to_value(zns)?),
        };

        let actual_url = reqwest::Url::parse(
          format!("{}/stream", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(actual_url).json(&l).send().await?;
        let rstream = res.bytes_stream().map_err(convert_err);
        let mut br = StreamReader::new(rstream);

        let mut line = String::new();
        let nc = br.read_line(&mut line).await?;

        if nc == 0 {
          return Err("empty stream!".into());
        }

        println!("line: {}", line);

        let sr = serde_json::from_str::<PrivateReplyMessage>(line.trim())?;

        if sr.what != PrivateReplies::ZkNoteSearchResult {
          return Err(format!("unexpected what {:?}", sr.what).into());
        }

        // write the notes!
        let sysid = user_id(&conn, "system")?;

        let mut count = 0;
        let mut bytes = 0;

        loop {
          line.clear();
          let nc = br.read_line(&mut line).await?;

          if nc == 0 {
            break;
          }

          let note = serde_json::from_str::<ZkNote>(line.trim())?;

          println!("archivenote: {:?}", note);

          count = count + 1;
          bytes = bytes + nc;

          match conn.execute(
          "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate)
           values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
              note.title,
              note.content,
              sysid,
              note.pubid,
              note.editable,
              note.showtitle,
              note.deleted,
              note.id.to_string(),
              note.createdate,
              note.changeddate,
              ])
            {
              Ok(_x) => (),
              Err(rusqlite::Error::SqliteFailure(e, s)) =>
                if e.code == rusqlite::ErrorCode::ConstraintViolation {
                  // if duplicate record, just ignore and go on.
                 ()
                } else {
                  return Err(rusqlite::Error::SqliteFailure(e, s).into())
                }
              Err(e) => return Err(e)?,
            }
        }

        println!("synched {} archive notes, with {} bytes!", count, bytes);
      }

      if getarchivelinks {
        let gazl = GetArchiveZkLinks {
          createddate_after: after,
        };
        let l = PrivateStreamingMessage {
          what: PrivateStreamingRequests::GetArchiveZkLinks,
          data: Some(serde_json::to_value(gazl)?),
        };

        let actual_url = reqwest::Url::parse(
          format!("{}/stream", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(actual_url).json(&l).send().await?;

        let rstream = res.bytes_stream().map_err(convert_err);
        let mut br = StreamReader::new(rstream);
        let mut line = String::new();

        let nc = br.read_line(&mut line).await?;
        if nc == 0 {
          return Err("empty stream!".into());
        }

        println!("line: {}", line);

        let sr = serde_json::from_str::<PrivateReplyMessage>(line.trim())?;

        if sr.what != PrivateReplies::ArchiveZkLinks {
          return Err(format!("unexpected what {:?}", sr.what).into());
        }

        let mut count = 0;
        let mut saved = 0;
        let mut bytes = 0;

        loop {
          line.clear();
          let nc = br.read_line(&mut line).await?;

          if nc == 0 {
            break;
          }

          println!("archive link line: {}", line);

          let l = serde_json::from_str::<ArchiveZkLink>(line.trim())?;

          let ins = match conn.execute(
            "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
              select FN.id, TN.id, U.id, LN.id, ?1, ?2
              from zknote FN, zknote TN, orgauth_user U, zknote LN
              where FN.uuid = ?3
                and TN.uuid = ?4
                and U.uuid = ?5
                and LN.uuid = ?6",
            params![
              l.createdate,
              l.deletedate,
              l.fromUuid,
              l.toUuid,
              l.userUuid,
              l.linkUuid
            ],
          ) {
            Ok(_x) => 1,
            Err(rusqlite::Error::SqliteFailure(e, s)) => {
              if e.code == rusqlite::ErrorCode::ConstraintViolation {
                // if duplicate record, just ignore and go on.
                0
              } else {
                return Err(rusqlite::Error::SqliteFailure(e, s).into());
              }
            }
            Err(e) => return Err(e)?,
          };
          count = count + 1;
          saved = saved + ins;
          bytes = bytes + nc;
          println!("archived link count:, {}", count);
        }

        println!(
          "receieved archive links: {}, saved {}, bytes {}",
          count, saved, bytes
        );
      }

      if getlinks {
        let gazl = GetZkLinksSince {
          createddate_after: after,
        };
        let l = PrivateStreamingMessage {
          what: PrivateStreamingRequests::GetZkLinksSince,
          data: Some(serde_json::to_value(gazl)?),
        };

        let actual_url = reqwest::Url::parse(
          format!("{}/stream", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(actual_url).json(&l).send().await?;

        let rstream = res.bytes_stream().map_err(convert_err);
        let mut br = StreamReader::new(rstream);
        let mut line = String::new();

        let nc = br.read_line(&mut line).await?;
        if nc == 0 {
          return Err("empty stream!".into());
        }

        println!("line: {}", line);

        let sr = serde_json::from_str::<PrivateReplyMessage>(line.trim())?;

        if sr.what != PrivateReplies::ZkLinks {
          return Err(format!("unexpected what {:?}", sr.what).into());
        }

        let mut count = 0;
        let mut saved = 0;
        let mut bytes = 0;

        loop {
          line.clear();
          let nc = br.read_line(&mut line).await?;

          if nc == 0 {
            break;
          }

          println!("link line: {}", line);

          let l = serde_json::from_str::<UuidZkLink>(line.trim())?;

          println!("saving link!, {:?}", l);
          let ins = match conn.execute(
            "with vals(a,b,c,d,e) as (
              select FN.id, TN.id, U.id, LN.id, ?1
              from zknote FN, zknote TN, orgauth_user U
                left outer join zknote LN
                 on LN.uuid = ?5
              where FN.uuid = ?2
                and TN.uuid = ?3
                and U.uuid = ?4)
              insert into zklink (fromid, toid, user, linkzknote, createdate)
                select * from vals ",
            params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
          ) {
            Ok(c) => {
              println!("inserted {}", c);
              Ok(c)
            }
            Err(rusqlite::Error::SqliteFailure(e, s)) => {
              if e.code == rusqlite::ErrorCode::ConstraintViolation {
                // do update, since we can't ON CONFLICT without a values () clause.
                let count = conn.execute(
                  "with vals(a,b,c,d,e) as (
                    select FN.id, TN.id, U.id, LN.id, ?1
                    from zknote FN, zknote TN, orgauth_user U
                      left outer join zknote LN
                       on LN.uuid = ?5
                    where FN.uuid = ?2
                      and TN.uuid = ?3
                      and U.uuid = ?4)
                    update zklink set linkzknote = vals.d, createdate = vals.e
                      from vals
                      where fromid = vals.a
                        and toid = vals.b
                        and user = vals.c",
                  params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
                )?;
                println!("updated {}", count);
                // TODO: uuid conflict;  resolve with old one becoming archive note.
                // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
                Ok(count)
              } else {
                Err(rusqlite::Error::SqliteFailure(e, s))
              }
            }
            Err(e) => Err(e),
          }?;
          count = count + 1;
          saved = saved + ins;
          bytes = bytes + nc;
        }
        println!(
          "receieved links: {}, saved {}, bytes {}",
          count, saved, bytes
        );
      }

      println!("dropping deleted links");
      // drop zklinks which have a zklinkarchive with newer deletedate
      let dropped = conn.execute(
        "with dels as (select ZL.fromid, ZL.toid, ZL.user from zklink ZL, zklinkarchive ZLA
          where ZL.fromid = ZLA.fromid
          and ZL.toid = ZLA.toid
          and ZL.user = ZLA.user
          and ZL.createdate < ZLA.deletedate)
          delete from zklink where
            (zklink.fromid, zklink.toid, zklink.user) in dels ",
        params![],
      )?;

      println!("dropped {} links", dropped);

      let unote = user_note_id(conn, user.id)?;

      save_sync(conn, user.id, unote, CompletedSync { after, now }).await?;

      println!("meh");

      // TODO update cookie?
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SyncComplete,
        content: serde_json::Value::Null,
      })
    }
    _ => Err("can't remote sync".into()),
  }
}

fn convert_bodyerr(err: actix_web::error::PayloadError) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

pub struct TempTableNames {
  pub notetemp: String,
  pub linktemp: String,
  pub archivelinktemp: String,
}

pub fn temp_tables(conn: &Connection) -> Result<TempTableNames, zkerr::Error> {
  // // create temporary tables for links and notes we get from the remote.
  // create temporary tables for links and notes we get from the remote.
  let id = sqldata::update_single_value(&conn, "sync_id", |x| match x.parse::<i64>() {
    Ok(n) => (n + 1).to_string(),
    Err(_) => 0.to_string(),
  })?;

  let notetemp = format!("notetemp_{}", id);
  let linktemp = format!("linktemp_{}", id);
  let archivelinktemp = format!("archivelinktemp_{}", id);

  // temporary tables.  should drop when the db connection ends.
  conn.execute(
    format!(
      //     "create temporary table {} (\"id\" integer primary key not null)",
      "create table {} (\"id\" integer primary key not null)",
      notetemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      //     "create temporary table {} (
      "create table {} (
      \"fromid\" INTEGER NOT NULL,
      \"toid\" INTEGER NOT NULL,
      \"user\" INTEGER NOT NULL)",
      linktemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      "CREATE UNIQUE INDEX \"{}unq\" ON \"{}\" (\"fromid\", \"toid\", \"user\")",
      linktemp, linktemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      //     "create temporary table {} (\"id\" integer primary key not null)",
      "create table {} (\"id\" integer primary key not null)",
      archivelinktemp
    )
    .as_str(),
    params![],
  )?;

  Ok(TempTableNames {
    notetemp,
    linktemp,
    archivelinktemp,
  })
}

pub async fn sync(
  dbpath: &Path,
  uid: i64,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let conn = Arc::new(sqldata::connection_open(dbpath)?);
  let user = orgauth::dbfun::read_user_by_id(&conn, uid)?; // TODO pass this in from calling ftn?
  let extra_login_data = sqldata::read_user_by_id(&conn, user.id)?;

  // get previous sync.
  let after = prev_sync(&conn, &user, &extra_login_data.zknote)
    .await?
    .map(|cs| cs.now);

  let now = now()?;

  println!("\n\n start sync, prev_sync {:?} \n\n", after);

  let tr = conn.unchecked_transaction()?;

  let ttn = temp_tables(&conn)?;

  // TODO: pass in 'now'?
  let res = sync_from_remote(
    &conn,
    &user,
    after,
    &ttn.notetemp,
    &ttn.linktemp,
    &ttn.archivelinktemp,
    callbacks,
  )
  .await?;
  if res.what != PrivateReplies::SyncComplete {
    Ok(res)
  } else {
    let remres = sync_to_remote(
      conn.clone(),
      &user,
      Some(ttn.notetemp.clone()),
      Some(ttn.linktemp.clone()),
      Some(ttn.archivelinktemp.clone()),
      after,
      callbacks,
    )
    .await?;

    let unote = user_note_id(&conn, user.id)?;
    save_sync(&conn, user.id, unote, CompletedSync { after, now }).await?;

    tr.commit()?;

    Ok(remres)
  }
}

pub async fn sync_from_remote(
  conn: &Connection,
  user: &User,
  after: Option<i64>,
  notetemp: &String,
  linktemp: &String,
  archivelinktemp: &String,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync".into()),
  };

  let user_url = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;
  let mut parts = awc::http::uri::Parts::default();
  parts.scheme = user_url.scheme().cloned();
  parts.authority = user_url.authority().cloned();
  parts.path_and_query = Some(awc::http::uri::PathAndQuery::from_static("/stream"));
  let uri = awc::http::Uri::from_parts(parts).map_err(|x| zkerr::Error::String(x.to_string()))?;

  println!("sync uri {:?}", uri);

  let cookie = cookie::Cookie::parse_encoded(c)?;
  println!("sync to remote 3 - cookie: {:?}", cookie);

  println!("sync to remote 4");
  let res = awc::Client::new()
    .post(uri)
    .cookie(cookie)
    .timeout(Duration::from_secs(60 * 60))
    .send_json(&serde_json::to_value(PrivateStreamingMessage {
      what: PrivateStreamingRequests::Sync,
      data: Some(serde_json::to_value(SyncSince { after })?),
    })?)
    .await?;

  let mut sr = StreamReader::new(res.map_err(convert_payloaderr));

  let reply = sync_from_stream(
    conn,
    Some(notetemp),
    Some(linktemp),
    Some(archivelinktemp),
    callbacks,
    &mut sr,
  )
  .await?;
  Ok(reply)
}

pub async fn read_sync_message<S>(
  line: &mut String,
  br: &mut StreamReader<S, bytes::Bytes>,
) -> Result<SyncMessage, Box<dyn std::error::Error>>
where
  S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
  line.clear();
  if br.read_line(line).await? == 0 {
    return Err::<SyncMessage, Box<dyn std::error::Error>>(
      zkerr::Error::String("empty stream!".to_string()).into(),
    );
  }
  println!("readline: '{}'", line.trim());
  let sm = serde_json::from_str(line.trim())?;
  Ok(sm)
}

pub async fn sync_from_stream<S>(
  conn: &Connection,
  notetemp: Option<&str>,
  linktemp: Option<&str>,
  archivelinktemp: Option<&str>,
  callbacks: &mut Callbacks,
  br: &mut StreamReader<S, bytes::Bytes>,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>>
where
  S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
  println!(
    "sync_from_stream 1, {:?}, {:?}, {:?}",
    notetemp, linktemp, archivelinktemp
  );
  // pull in line by line and println
  // TODO: pass in now instead of compute here?
  let now = now()?;
  let sysid = user_id(&conn, "system")?;

  let mut line = String::new();

  let mut sm;

  sm = read_sync_message(&mut line, br).await?;

  let (after, remotenow) = match sm {
    SyncMessage::SyncStart(after, rn) => (after, rn),
    _ => return Err(format!("expected SyncStart; unexpected syncmessage: {:?}", sm).into()),
  };

  // milliseconds
  if (now - remotenow).abs() > 10000 {
    return Err(
      format!(
        "remote time too far off! local: {}, remote: {}",
        now, remotenow,
      )
      .into(),
    );
  }

  sm = read_sync_message(&mut line, br).await?;

  match sm {
    SyncMessage::PhantomUserHeader => (),
    _ => {
      return Err(
        format!(
          "expected PhantomUserHeader; unexpected syncmessage: {:?}",
          sm
        )
        .into(),
      )
    }
  }

  sm = read_sync_message(&mut line, br).await?;
  let mut userhash = HashMap::<i64, i64>::new();

  println!("sync_from_stream 5");
  while let SyncMessage::PhantomUser(ref pu) = sm {
    match userhash.get(&pu.id) {
      Some(u) => (),
      None => {
        println!("phantom user: {:?}", pu);
        let localuserid = match orgauth::dbfun::read_user_by_uuid(&conn, &pu.uuid) {
          Ok(user) => {
            println!("found local user {} for remote {}", user.id, pu.id);
            userhash.insert(pu.id, user.id);
          }
          _ => {
            let localpuid = orgauth::dbfun::phantom_user(
              &conn,
              &pu.name,
              pu.uuid,
              pu.active,
              &mut callbacks.on_new_user,
            )?;
            println!(
              "creating phantom user {} for remote user: {:?}",
              pu.id, localpuid
            );
            userhash.insert(pu.id, localpuid);
          }
        };
      }
    };
    sm = read_sync_message(&mut line, br).await?;
    println!("sync_from_stream 6 : {}", line);
  }

  // ----------------------------------------------------------------------------------
  // current zknotes
  // ----------------------------------------------------------------------------------
  // First should be the current notes.
  if let SyncMessage::ZkSearchResultHeader(ref zsrh) = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ZkSearchResultHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  sm = read_sync_message(&mut line, br).await?;

  while let SyncMessage::ZkNote(ref note) = sm {
    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;

    println!("syncing note: {} {}", note.id, note.deleted);

    let ex =  conn.execute(
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
        note.id.to_string(),
        note.createdate,
        note.changeddate,
      ]
      );
    println!("ex: {:?}", ex);
    let id: i64 = match ex {
      Ok(x) => Ok(conn.last_insert_rowid()),
      Err(rusqlite::Error::SqliteFailure(e, Some(s))) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          if s.contains("uuid") {
            let (nid, n) = sqldata::read_zknote_unchecked(&conn, &note.id)?;
            // TODO: uuid conflict;  resolve with older one becoming archive note.
            // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
            if note.changeddate > n.changeddate {
              println!("replacing note: {} {}", note.id, note.deleted);
              // note is newer.  archive the old and replace.
              sqldata::save_zknote(
                &conn,
                *uid,
                &SaveZkNote {
                  id: Some(note.id),
                  title: note.title.clone(),
                  pubid: note.pubid.clone(),
                  content: note.content.clone(),
                  editable: note.editable,
                  showtitle: note.showtitle,
                  deleted: note.deleted,
                },
              )
              .map(|x| x.0)
            } else {
              println!("saving as archive: {} {}", note.id, note.deleted);
              // note is older.  add as archive note.
              // may create duplicate archive notes if edited on two systems and then synced.
              sqldata::archive_zknote(&conn, nid, &note).map(|x| x.0)
            }
          } else if s.contains("pubid") {
            // Error out to alert user to conflict.
            Err(
              zkerr::Error::String(format!(
                "note exists with duplicate public id: {}",
                note.pubid.clone().unwrap_or("".to_string())
              ))
              .into(),
            )
          } else {
            println!("sqliatesfailure {:?}", e);
            Err(rusqlite::Error::SqliteFailure(e, Some(s)).into())
          }
        } else {
          Err(rusqlite::Error::SqliteFailure(e, Some(s)).into())
        }
      }
      Err(e) => {
        println!("alskdnflawen: {:?}", e);
        Err(e.into())
      }
    }?;

    println!("saved note");

    if let Some(ref nt) = notetemp {
      println!("insert into {} values (?1)", nt);
      conn.execute(
        format!("insert into {} values (?1)", nt).as_str(),
        params![id],
      )?;
    }

    sm = read_sync_message(&mut line, br).await?;
    println!("sync_from_stream 7");
  }

  // ----------------------------------------------------------------------------------
  // archive notes
  // ----------------------------------------------------------------------------------
  // After the current notes are the archivenotes.
  if let SyncMessage::ZkSearchResultHeader(ref zsrh) = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ZkSearchResultHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  println!("sync_from_stream 8");
  sm = read_sync_message(&mut line, br).await?;
  println!("sync_from_stream 9");
  while let SyncMessage::ZkNote(ref note) = sm {
    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;
    assert!(*uid == sysid);
    let mbid = match conn.execute(
      "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate)
       values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
          note.title,
          note.content,
          sysid,    // archivenotes all owned by system.
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          note.id.to_string(),
          note.createdate,
          note.changeddate,
          ])
        {
          Ok(_x) => Ok::<_, zkerr::Error>(Some(conn.last_insert_rowid())),
          Err(rusqlite::Error::SqliteFailure(e, s)) =>
            if e.code == rusqlite::ErrorCode::ConstraintViolation {
              // if duplicate record, just ignore and go on.
             Ok(None)
            } else {
              return Err(rusqlite::Error::SqliteFailure(e, s).into())
            }
          Err(e) => return Err(e)?,
        }?;
    if let (Some(id), Some(nt)) = (mbid, &notetemp) {
      conn.execute(
        format!("insert into {} values (?1)", nt).as_str(),
        params![id],
      )?;
    }
    sm = read_sync_message(&mut line, br).await?;
  }

  println!("sync_from_stream 11");
  if let SyncMessage::ArchiveZkLinkHeader = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ArchiveZkLinkHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  // ----------------------------------------------------------------------------------
  // archive links
  // ----------------------------------------------------------------------------------
  sm = read_sync_message(&mut line, br).await?;
  println!("sync_from_stream 12");

  let mut count = 0;
  let mut saved = 0;
  // let mut bytes = 0;

  while let SyncMessage::ArchiveZkLink(ref l) = sm {
    let mbid = match conn.execute(
      "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
              select FN.id, TN.id, U.id, LN.id, ?1, ?2
              from zknote FN, zknote TN, orgauth_user U, zknote LN
              where FN.uuid = ?3
                and TN.uuid = ?4
                and U.uuid = ?5
                and LN.uuid = ?6",
      params![
        l.createdate,
        l.deletedate,
        l.fromUuid,
        l.toUuid,
        l.userUuid,
        l.linkUuid,
      ],
    ) {
      Ok(_x) => Some(conn.last_insert_rowid()),
      Err(rusqlite::Error::SqliteFailure(e, s)) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          // if duplicate record, just ignore and go on.
          None
        } else {
          return Err(rusqlite::Error::SqliteFailure(e, s).into());
        }
      }
      Err(e) => return Err(e)?,
    };
    if let (Some(lt), Some(id)) = (&archivelinktemp, mbid) {
      conn.execute(
        format!("insert into {} values (?1)", lt).as_str(),
        params![id],
      )?;
    }
    count = count + 1;
    saved = saved + mbid.map_or(0, |_| 1);
    // bytes = bytes + nc;
    println!("archived link count:, {}", count);
    sm = read_sync_message(&mut line, br).await?;
  }

  // ----------------------------------------------------------------------------------
  // current links
  // ----------------------------------------------------------------------------------
  println!("sync_from_stream 13");
  if let SyncMessage::UuidZkLinkHeader = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected UuidZkLinkHeader: {:?}",
        sm
      )
      .into(),
    );
  }
  count = 0;
  saved = 0;
  // bytes = 0;

  println!("sync_from_stream 14");
  sm = read_sync_message(&mut line, br).await?;

  while let SyncMessage::UuidZkLink(ref l) = sm {
    println!("saving link!, {:?}", l);
    let ins = match conn.execute(
      "with vals(a,b,c,d,e) as (
              select FN.id, TN.id, U.id, LN.id, ?1
              from zknote FN, zknote TN, orgauth_user U
                left outer join zknote LN
                 on LN.uuid = ?5
              where FN.uuid = ?2
                and TN.uuid = ?3
                and U.uuid = ?4)
              insert into zklink (fromid, toid, user, linkzknote, createdate)
                select * from vals",
      params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
    ) {
      Ok(c) => Ok(c),
      Err(rusqlite::Error::SqliteFailure(e, s)) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          // do update, since we can't ON CONFLICT without a values () clause.
          let count = conn.execute(
            "with vals(a,b,c,d,e) as (
                    select FN.id, TN.id, U.id, LN.id, ?1
                    from zknote FN, zknote TN, orgauth_user U
                      left outer join zknote LN
                       on LN.uuid = ?5
                    where FN.uuid = ?2
                      and TN.uuid = ?3
                      and U.uuid = ?4)
                    update zklink set linkzknote = vals.d, createdate = vals.e
                      from vals
                      where fromid = vals.a
                        and toid = vals.b
                        and user = vals.c",
            params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
          )?;
          println!("updated {}", count);
          // TODO: uuid conflict;  resolve with old one becoming archive note.
          // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
          Ok(count)
        } else {
          Err(rusqlite::Error::SqliteFailure(e, s))
        }
      }
      Err(e) => Err(e),
    }?;

    if let Some(lt) = &linktemp {
      if ins == 1 {
        conn.execute(
          format!(
            "insert into {} (fromid, toid, user)
            select FN.id, TN.id, U.id
              from zknote FN, zknote TN, orgauth_user U
                  where FN.uuid = ?1
                    and TN.uuid = ?2
                    and U.uuid = ?3",
            lt
          )
          .as_str(),
          params![l.fromUuid, l.toUuid, l.userUuid],
        )?;
      }
    }

    count = count + 1;
    saved = saved + ins;
    // bytes = bytes + nc;
    sm = read_sync_message(&mut line, br).await?;
  }
  println!("receieved links: {}, saved {}", count, saved);

  println!("dropping deleted links");
  // drop zklinks which have a zklinkarchive with newer deletedate
  let dropped = conn.execute(
    "with dels as (select ZL.fromid, ZL.toid, ZL.user from zklink ZL, zklinkarchive ZLA
        where ZL.fromid = ZLA.fromid
        and ZL.toid = ZLA.toid
        and ZL.user = ZLA.user
        and ZL.createdate < ZLA.deletedate)
        delete from zklink where
          (zklink.fromid, zklink.toid, zklink.user) in dels ",
    params![],
  )?;

  println!("dropped {} links", dropped);

  Ok(PrivateReplyMessage {
    what: PrivateReplies::SyncComplete,
    content: serde_json::Value::Null,
  })
}

// Make a stream of all the records needed to sync the remote.
pub async fn sync_to_remote(
  conn: Arc<Connection>,
  user: &User,
  exclude_notes: Option<String>,
  exclude_links: Option<String>,
  exclude_archivelinks: Option<String>,
  after: Option<i64>,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, zkerr::Error> {
  // let tr = conn.transaction()?;

  // let extra_login_data = sqldata::read_user_by_id(&conn, user.id)?;

  // TODO: get time on remote system, bail if too far out.

  println!("\n\n start sync, prev_sync {:?} \n\n", after);

  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync".into()),
  };

  println!("sync to remote 2");
  let user_url = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;
  let mut parts = awc::http::uri::Parts::default();
  parts.scheme = user_url.scheme().cloned();
  parts.authority = user_url.authority().cloned();
  parts.path_and_query = Some(awc::http::uri::PathAndQuery::from_static("/upstream"));
  let uri = awc::http::Uri::from_parts(parts).map_err(|x| zkerr::Error::String(x.to_string()))?;

  println!("sync uri {:?}", uri);

  let cookie = cookie::Cookie::parse_encoded(c)?;
  println!("sync to remote 3 - cookie: {:?}", cookie);

  let ss = sync_stream(
    conn,
    user.id,
    exclude_notes,
    exclude_links,
    exclude_archivelinks,
    after,
    callbacks,
  );

  // -----------------------------------
  // Writing the stream to a file
  // -----------------------------------
  // use futures::executor::block_on;
  // use futures::{future, future::FutureExt, stream, stream::StreamExt};
  // use tokio::io::AsyncWriteExt;
  // let mut file = tokio::fs::OpenOptions::new()
  //   .write(true)
  //   .create(true)
  //   .open("some_file")
  //   .await
  //   .unwrap();

  // ss.for_each(|item| match item {
  //   Ok(bytes) => {
  //     block_on(file.write(&bytes));
  //     future::ready(())
  //   }
  //   Err(e) => future::ready(()),
  // })
  // .await;

  println!("sync to remote 4");
  let res = awc::Client::new()
    .post(uri)
    .cookie(cookie)
    .timeout(Duration::from_secs(60 * 60))
    .send_body(awc::body::BodyStream::new(ss))
    .await;
  println!("sync to remote 5 : {:?}", res);

  Ok(PrivateReplyMessage {
    what: PrivateReplies::SyncComplete,
    content: serde_json::Value::Null,
  })
}

pub fn bytesify(
  x: Result<SyncMessage, Box<dyn std::error::Error>>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
  // map errors to SyncErrors so we'll at least see them on the client.
  // not sure how to log these on the server side.
  let sm = match x {
    Ok(sm) => sm,
    Err(e) => SyncMessage::SyncError(e.to_string()),
  };

  serde_json::to_value(sm)
    .map(|x| {
      Bytes::from({
        let mut z = x.to_string();
        println!("bytesify '{}'", z);
        z.push_str("\n");
        z
      })
    })
    .map_err(|e| e.into())
}

// Make a stream of all the records needed to sync the remote.
pub fn sync_stream(
  conn: Arc<Connection>,
  uid: i64,
  exclude_notes: Option<String>,
  exclude_links: Option<String>,
  exclude_archivelinks: Option<String>,
  after: Option<i64>,
  callbacks: &mut Callbacks,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + 'static>>> {
  let start = try_stream! { yield SyncMessage::SyncStart(after, now()?); }.map(bytesify);

  // !(et'sync' & u'system')
  // !(et'user' & u'system')
  // and not z'' of the SpecialUuids::
  // (!z'f596bc2c-a882-4c1c-b739-8c4e25f34eb2' &
  // (!z'e82fefee-bcd3-4e2e-b350-9963863e516d' &
  // (!z'466d39ec-2ea7-4d43-b44c-1d3d083f8a9d' &
  // (!z'84f72fd0-8836-43a3-ac66-89e0ab49dd87' &
  // (!z'4fb37d76-6fc8-4869-8ee4-8e05fa5077f7' &
  // (!z'ad6a4ca8-0446-4ecc-b047-46282ced0d84' &
  // (!z'0efcc98f-dffd-40e5-af07-90da26b1d469' &
  // !z'528ccfc2-8488-41e0-a4e1-cbab6406674e')))))))

  // Don't sync 'sync' notes.
  let exclude_sync = TagSearch::Not {
    ts: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId, SearchMod::Tag],
        term: SpecialUuids::Sync.str().to_string(),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::User],
        term: "system".to_string(),
      }),
    }),
  };

  // Don't sync user notes.
  let exclude_user = TagSearch::Not {
    ts: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId, SearchMod::Tag],
        term: SpecialUuids::User.str().to_string(),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::User, SearchMod::Tag],
        term: "system".to_string(),
      }),
    }),
  };

  // Don't sync the special system notes.
  // remind me to write a search parser in rust!  tedious
  let exclude_system = TagSearch::Boolex {
    ts1: Box::new(TagSearch::Not {
      ts: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId],
        term: SpecialUuids::Public.str().to_string(),
      }),
    }),
    ao: AndOr::And,
    ts2: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::Not {
        ts: Box::new(TagSearch::SearchTerm {
          mods: vec![SearchMod::ZkNoteId],
          term: SpecialUuids::Comment.str().to_string(),
        }),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::Boolex {
        ts1: Box::new(TagSearch::Not {
          ts: Box::new(TagSearch::SearchTerm {
            mods: vec![SearchMod::ZkNoteId],
            term: SpecialUuids::Share.str().to_string(),
          }),
        }),
        ao: AndOr::And,
        ts2: Box::new(TagSearch::Boolex {
          ts1: Box::new(TagSearch::Not {
            ts: Box::new(TagSearch::SearchTerm {
              mods: vec![SearchMod::ZkNoteId],
              term: SpecialUuids::Search.str().to_string(),
            }),
          }),
          ao: AndOr::And,
          ts2: Box::new(TagSearch::Boolex {
            ts1: Box::new(TagSearch::Not {
              ts: Box::new(TagSearch::SearchTerm {
                mods: vec![SearchMod::ZkNoteId],
                term: SpecialUuids::User.str().to_string(),
              }),
            }),
            ao: AndOr::And,
            ts2: Box::new(TagSearch::Boolex {
              ts1: Box::new(TagSearch::Not {
                ts: Box::new(TagSearch::SearchTerm {
                  mods: vec![SearchMod::ZkNoteId],
                  term: SpecialUuids::Archive.str().to_string(),
                }),
              }),
              ao: AndOr::And,
              ts2: Box::new(TagSearch::Boolex {
                ts1: Box::new(TagSearch::Not {
                  ts: Box::new(TagSearch::SearchTerm {
                    mods: vec![SearchMod::ZkNoteId],
                    term: SpecialUuids::System.str().to_string(),
                  }),
                }),
                ao: AndOr::And,
                ts2: Box::new(TagSearch::Not {
                  ts: Box::new(TagSearch::SearchTerm {
                    mods: vec![SearchMod::ZkNoteId],
                    term: SpecialUuids::Sync.str().to_string(),
                  }),
                }),
              }),
            }),
          }),
        }),
      }),
    }),
  };

  let emptyts = TagSearch::SearchTerm {
    mods: Vec::new(),
    term: "".to_string(),
  };

  let zns = ZkNoteSearch {
    tagsearch: emptyts.clone(),
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: true,
    created_after: None,
    created_before: None,
    changed_after: after,
    changed_before: None,
    ordering: None,
  };

  let sync_users = sync_users(conn.clone(), uid, after, &zns).map(bytesify);

  let system_user = system_user(conn.clone()).map(bytesify);

  // TODO: sync_users derived from read_zklinks_since_stream ?

  let full_excl = TagSearch::Boolex {
    ts1: Box::new(exclude_sync),
    ao: AndOr::And,
    ts2: Box::new(TagSearch::Boolex {
      ts1: Box::new(exclude_user),
      ao: AndOr::And,
      ts2: Box::new(exclude_system),
    }),
  };

  let zns = ZkNoteSearch {
    tagsearch: full_excl,
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: true,
    created_after: None,
    created_before: None,
    changed_after: after,
    changed_before: None,
    ordering: None,
  };

  let znstream = search_zknotes_stream(conn.clone(), uid, zns, exclude_notes.clone()).map(bytesify);

  let ans = ZkNoteSearch {
    tagsearch: emptyts,
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: true,
    deleted: false,
    created_after: None,
    created_before: None,
    changed_after: after,
    changed_before: None,
    ordering: None,
  };

  let anstream = search_zknotes_stream(conn.clone(), uid, ans, exclude_notes).map(bytesify);

  let als =
    sqldata::read_archivezklinks_stream(conn.clone(), uid, after, exclude_archivelinks.clone())
      .map(bytesify);

  let ls = sqldata::read_zklinks_since_stream(conn, uid, after, exclude_links).map(bytesify);

  let end = try_stream! { yield SyncMessage::SyncEnd; }.map(bytesify);

  start
    .chain(sync_users)
    .chain(system_user)
    .chain(znstream)
    .chain(anstream)
    .chain(als)
    .chain(ls)
    .chain(end)
}
