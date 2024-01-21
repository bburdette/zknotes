use crate::search::{search_zknotes, search_zknotes_stream, sync_users, SearchResult};
use crate::util::now;
use actix_web::body::None;
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use async_stream::__private::AsyncStream;
use async_stream::try_stream;
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
use rusqlite::{params, Connection};
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
  ArchiveZkLink, GetArchiveZkLinks, GetZkLinksSince, SaveZkNote, SyncMessage, UuidZkLink, ZkNote,
  ZkNoteId,
};
use zkprotocol::messages::{PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage};
use zkprotocol::search::{
  OrderDirection, OrderField, Ordering, ResultType, SearchMod, TagSearch, ZkNoteSearch,
};

fn convert_err(err: reqwest::Error) -> std::io::Error {
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
    created_after: None,
    created_before: None,
    changed_after: None,
    changed_before: None,
    synced_after: None,
    synced_before: None,
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

pub async fn sync_from_remote(
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
  // let jar = reqwest::cookies::Jar;
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
          created_after: after,
          created_before: None,
          changed_after: after,
          changed_before: None,
          synced_after: after,
          synced_before: None,
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
                    sqldata::archive_zknote(&conn, nid, now, &note)?;
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
          created_after: after,
          created_before: None,
          changed_after: after,
          changed_before: None,
          synced_after: after,
          synced_before: None,
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

  // let jar = reqwest::cookies::Jar;
  // match (user.cookie, user.remote_url) {}
}

fn convert_bodyerr(err: actix_web::error::PayloadError) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

pub async fn sync_from_stream(
  conn: &Connection,
  user: &User,
  callbacks: &mut Callbacks,
  body: actix_web::web::Payload,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  // pull in line by line and println
  let rstream = body.map_err(convert_bodyerr);

  let mut br = StreamReader::new(rstream);

  // TODO: pass in now instead of compute here?
  let now = now()?;
  let sysid = user_id(&conn, "system")?;

  let mut line = String::new();
  let nc = br.read_line(&mut line).await?;

  if nc == 0 {
    return Err("empty stream!".into());
  }

  let mut sm: SyncMessage = serde_json::from_str(line.as_str())?;

  match sm {
    SyncMessage::PhantomUserHeader => (),
    _ => return Err(format!("unexpected syncmessage: {:?}", sm).into()),
  }

  if br.read_line(&mut line).await? == 0 {
    return Err("empty stream!".into());
  }
  sm = serde_json::from_str(line.as_str())?;
  let mut userhash = HashMap::<i64, i64>::new();

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
    if br.read_line(&mut line).await? == 0 {
      return Err("empty stream!".into());
    }
    sm = serde_json::from_str(line.as_str())?;
  }

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

  if br.read_line(&mut line).await? == 0 {
    return Err("empty stream!".into());
  }
  sm = serde_json::from_str(line.as_str())?;

  while let SyncMessage::ZkNote(ref note) = sm {
    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;

    match conn.execute(
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
                                   *uid,
                                   &SaveZkNote {
                                     id: Some(note.id),
                                     title: note.title.clone(),
                                     pubid: note.pubid.clone(),
                                     content: note.content.clone(),
                                     editable: note.editable,
                                     showtitle: note.showtitle,
                                     deleted: note.deleted,
                                   })?;
            } else {
              // note is older.  add as archive note.
              // may create duplicate archive notes if edited on two systems and then synced.
              sqldata::archive_zknote(&conn, nid, now, &note)?;
            }
            Ok(1)
          } else {
            Err(rusqlite::Error::SqliteFailure(e, s))
          }
        Err(e) => Err(e),
      }?;
    if br.read_line(&mut line).await? == 0 {
      return Err("empty stream!".into());
    }
    sm = serde_json::from_str(line.as_str())?;
  }

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

  if br.read_line(&mut line).await? == 0 {
    return Err("empty stream!".into());
  }
  sm = serde_json::from_str(line.as_str())?;

  while let SyncMessage::ZkNote(ref note) = sm {
    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;
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
    if br.read_line(&mut line).await? == 0 {
      return Err("empty stream!".into());
    }
    sm = serde_json::from_str(line.as_str())?;
  }

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

  if br.read_line(&mut line).await? == 0 {
    return Err("empty stream!".into());
  }
  sm = serde_json::from_str(line.as_str())?;

  let mut count = 0;
  let mut saved = 0;
  let mut bytes = 0;

  while let SyncMessage::ArchiveZkLink(ref l) = sm {
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
    if br.read_line(&mut line).await? == 0 {
      return Err("empty stream!".into());
    }
    sm = serde_json::from_str(line.as_str())?;
  }

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
  bytes = 0;

  if br.read_line(&mut line).await? == 0 {
    return Err("empty stream!".into());
  }
  sm = serde_json::from_str(line.as_str())?;

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
    if br.read_line(&mut line).await? == 0 {
      return Err("empty stream!".into());
    }
    sm = serde_json::from_str(line.as_str())?;
  }
  println!(
    "receieved links: {}, saved {}, bytes {}",
    count, saved, bytes
  );

  // // let jar = reqwest::cookies::Jar;
  // // match (user.cookie, user.remote_url) {}
  Ok(PrivateReplyMessage {
    what: PrivateReplies::SyncComplete,
    content: serde_json::Value::Null,
  })
}

// Make a stream of all the records needed to sync the remote.
pub async fn sync_to_remote(
  conn: Arc<Connection>,
  user: &User,
  callbacks: &mut Callbacks,
  // ) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
) -> Result<PrivateReplyMessage, zkerr::Error> {
  let extra_login_data = sqldata::read_user_by_id(&conn, user.id)?;

  // TODO: get time on remote system, bail if too far out.

  // get previous sync.
  let after = prev_sync(&conn, &user, &extra_login_data.zknote)
    .await?
    .map(|cs| cs.now);
  // let after = None;

  println!("\n\n start sync, prev_sync {:?} \n\n", after);

  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync".into()),
  };

  println!("sync to remote 2");
  let now = now()?;
  // let user_url  = Into::<awc::http::Uri>::into(url)?;
  let user_url = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;
  let mut parts = awc::http::uri::Parts::default();
  parts.scheme = user_url.scheme().cloned();
  parts.authority = user_url.authority().cloned();
  parts.path_and_query = Some(awc::http::uri::PathAndQuery::from_static("/upstream"));
  let uri = awc::http::Uri::from_parts(parts).map_err(|x| zkerr::Error::String(x.to_string()))?;

  println!("sync uri {:?}", uri);

  let client = awc::Client::new();
  let cookie = cookie::Cookie::parse_encoded(c)?;
  println!("sync to remote 3 - cookie: {:?}", cookie);

  let ss = sync_stream(conn, user.id, after, callbacks);
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

pub fn empty_stream() -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + 'static>>> {
  try_stream! {
    yield Bytes::from("");
  }
}

pub fn bytesify(
  x: Result<SyncMessage, Box<dyn std::error::Error>>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
  x.and_then(|x| {
    serde_json::to_value(x)
      .map(|x| {
        Bytes::from({
          let mut z = x.to_string();
          z.push_str("\n");
          z
        })
      })
      .map_err(|e| e.into())
  })
}

// Make a stream of all the records needed to sync the remote.
pub fn sync_stream(
  conn: Arc<Connection>,
  uid: i64,
  after: Option<i64>,
  callbacks: &mut Callbacks,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + 'static>>> {
  // let mut userhash = HashMap::<i64, i64>::new();

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
    created_after: after,
    created_before: None,
    changed_after: after,
    changed_before: None,
    synced_after: after,
    synced_before: None,
    ordering: None,
  };

  let sync_users = sync_users(conn.clone(), uid, after, &zns).map(bytesify);

  // TODO: sync_users derived from read_zklinks_since_stream ?

  let znstream = search_zknotes_stream(conn.clone(), uid, zns).map(bytesify);

  let ans = ZkNoteSearch {
    tagsearch: TagSearch::SearchTerm {
      mods: Vec::new(),
      term: "".to_string(),
    },
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: true,
    created_after: after,
    created_before: None,
    changed_after: after,
    changed_before: None,
    synced_after: after,
    synced_before: None,
    ordering: None,
  };
  let anstream = search_zknotes_stream(conn.clone(), uid, ans).map(bytesify);

  let als = sqldata::read_archivezklinks_stream(conn.clone(), uid, after).map(bytesify);
  let ls = sqldata::read_zklinks_since_stream(conn, uid, after).map(bytesify);

  sync_users
    .chain(znstream)
    .chain(anstream)
    .chain(als)
    .chain(ls)
}
