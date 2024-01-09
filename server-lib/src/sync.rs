use crate::search::{search_zknotes, SearchResult};
use crate::util::now;
use actix_web::body::None;
use actix_web_actors::ws::ProtocolError;
use bytes::Bytes;
use bytestring::ByteString;
use futures_util::TryStreamExt;
use log::{error, info};
use std::str;
use std::{io, thread};
// use json::JsonResult;
use crate::error as zkerr;
use crate::sqldata::{self, note_id_for_uuid, save_zklink, save_zknote, user_note_id};
use awc::ws;
use core::pin::Pin;
use futures_util::{SinkExt as _, StreamExt as _};
use orgauth;
use orgauth::data::{PhantomUser, User, UserResponse, UserResponseMessage};
use orgauth::dbfun::user_id;
use orgauth::endpoints::Callbacks;
use reqwest;
use reqwest::cookie;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::{select, sync::mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::io::StreamReader;
use uuid::Uuid;
use zkprotocol::constants::{
  PrivateReplies, PrivateRequests, PrivateStreamingRequests, SpecialUuids,
};
use zkprotocol::content::{
  ArchiveZkLink, GetArchiveZkLinks, GetZkLinksSince, SaveZkNote, UuidZkLink, ZkNote, ZkNoteId,
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

  // TODO: remove
  return Ok(None);

  // if let SearchResult::SrNote(res) = search_zknotes(conn, sysid, &zns)? {
  //   println!("result noates: {:?}", res.notes);
  //   match res.notes.first() {
  //     Some(n) => match serde_json::from_str::<CompletedSync>(n.content.as_str()) {
  //       Ok(s) => Ok(Some(s)),
  //       Err(e) => {
  //         error!("CompletedSync parse error: {:?}", e);
  //         Ok(None)
  //       }
  //     },
  //     None => Ok(None),
  //   }
  // } else {
  //   Err("unexpected search result type".into())
  // }
}

pub async fn save_sync(
  conn: &Connection,
  uid: i64,
  usernoteid: i64,
  sync: CompletedSync,
) -> Result<i64, zkerr::Error> {
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

pub async fn websocket_sync_from(
  conn: &Connection,
  user: &User,
  callbacks: &mut Callbacks,
  // ) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
) -> Result<PrivateReplyMessage, zkerr::Error> {
  let now = now()?;

  let extra_login_data = sqldata::read_user_by_id(conn, user.id)?;

  // get previous sync.
  let after = prev_sync(&conn, &user, &extra_login_data.zknote)
    .await?
    .map(|cs| cs.now);

  println!("\n\n start sync, prev_sync {:?} \n\n", after);

  // execute any command from here.  search?
  // let jar = reqwest::cookies::Jar;
  match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => {
      let url = reqwest::Url::parse(url.as_str())?;

      let jar = Arc::new(cookie::Jar::default());
      jar.add_cookie_str(c.as_str(), &url);
      let client = reqwest::Client::builder().cookie_provider(jar).build()?;

      let getnotes = true;
      let getlinks = false;
      let getarchivenotes = false;
      let getarchivelinks = false;

      // Request a ws token from remote.
      let private_url =
        reqwest::Url::parse(format!("{}/private", url.origin().unicode_serialization(),).as_str())?;

      let res = client
        .post(private_url.clone())
        .json(&PrivateMessage {
          what: PrivateRequests::GetWsToken,
          data: None,
        })
        .send()
        .await?;

      let pm = serde_json::from_value::<PrivateReplyMessage>(res.json().await?)?;

      let wstoken = match pm.what {
        PrivateReplies::WsToken => {
          println!("token content: {:?}", pm.content);
          serde_json::from_value::<Uuid>(pm.content).map_err(|e| Into::<zkerr::Error>::into(e))
        }
        _ => Err(format!("unexpected what: {:?}", pm.what).as_str().into()),
      }?;

      // naio.  connect to the websocket.
      // std::thread::sleep(std::time::Duration::new(1, 0));

      // maybe use http::url here.
      let wsurl = reqwest::Url::parse(
        format!(
          "{}:{}/ws/{}",
          if url.scheme() == "http" { "ws" } else { "wss" },
          url.authority(), // .unicode_serialization(),
          wstoken.to_string()
        )
        .as_str(),
      )?;

      println!("wsurl {}", wsurl);

      /*
      let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
      let mut cmd_rx = UnboundedReceiverStream::new(cmd_rx);

      // run blocking terminal input reader on separate thread
      let input_thread = thread::spawn(move || loop {
        let mut cmd = String::with_capacity(32);

        if io::stdin().read_line(&mut cmd).is_err() {
          log::error!("error reading line");
          return;
        }

        // decode each message and send.
        cmd_tx.send(cmd).unwrap();
      });
      */

      // let (res, mut ws) = awc::Client::new().ws(wsurl.as_str()).connect().await?;
      let (res, mut ws) = awc::Client::new()
        .ws(wsurl.as_str())
        .connect()
        .await
        .unwrap();

      println!("connect res: {:?}", res);

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

        // send over ws.

        ws.send(ws::Message::Text(ByteString::from(serde_json::to_string(
          &l,
        )?)))
        .await?;

        // Pin::new(&mut ws).write(ws::Message::Text(ByteString::from(serde_json::to_string(
        //   &zns,
        // )?)));

        // match Pin::new(&mut ws).next_item() {}

        // let url = reqwest::Url::parse(
        //   format!("{}/stream", url.origin().unicode_serialization(),).as_str(),
        // )?;

        // let res = client.post(url.clone()).json(&l).send().await?;

        // handle streaming response!

        // let resp = res.json::<ServerResponse>().await?;
        /*
        let rstream = res.bytes_stream().map_err(convert_err);

        let mut br = StreamReader::new(rstream);

        let mut line = String::new();
        let nc = br.read_line(&mut line).await?;

        if nc == 0 {
          return Err("empty stream!".into());
        }
        */

        match ws.next().await {
          Some(msg) => match msg {
            Ok(ws::Frame::Text(txt)) => {
              println!("line: {:?}", txt);
              let sr = serde_json::from_str::<PrivateReplyMessage>(&str::from_utf8(&txt)?)?;
              if sr.what != PrivateReplies::ZkNoteSearchResult {
                return Err(format!("unexpected what {:?}", sr.what).into());
              }
            }
            Ok(_) => {
              return Err(format!("unexpected ws msg: {:?}", msg).into());
            }
            Err(e) => return Err(e.into()),
          },
          None => return Err("no search result".into()),
        }

        // map of remote user ids to local user ids.
        let user_url =
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

        let mut count = 0;
        loop {
          let note = match ws.next().await {
            Some(msg) => match msg {
              Ok(ws::Frame::Text(txt)) => {
                println!("zknote msg {:?}", txt);
                serde_json::from_str::<ZkNote>(&str::from_utf8(&txt)?)?
              }
              Ok(_) => {
                return Err(format!("unexpected ws msg: {:?}", msg).into());
              }
              Err(e) => return Err(e.into()),
            },
            None => {
              println!("breakkk");
              break;
            }
          };

          // line.clear();
          // let nc = br.read_line(&mut line).await?;
          // if nc == 0 {
          //   break;
          // }

          // println!("note line: {}", line);

          // let note = serde_json::from_str::<ZkNote>(line.trim())?;

          println!("zknote: {:?}", count);
          count = count + 1;
          /*
          // got this user already?
          // If not, make a phantom user.
          let user_uid: i64 = match userhash.get(&note.user) {
            Some(u) => *u,
            None => {
              println!("fetching remote user: {:?}", note.user);
              // fetch a remote user record.
              let res = client
                .post(user_url.clone())
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
                )?,
                _ => Err::<PhantomUser, zkerr::Error>(
                  zkerr::Error::String(format!("unexpected message: {:?}", wm)).into(),
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
                    pu.name,
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

          // if we had a sha, we'd insert based on that, right?
          // these are archive notes so should be able to insert if they don't exist, otherwise discard.
          // because archive notes shouldn't change.
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
          */
        }
      }
      println!("note sync complete");

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

pub async fn sync(
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

      let jar = Arc::new(cookie::Jar::default());
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
                    pu.name,
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

          // if we had a sha, we'd insert based on that, right?
          // these are archive notes so should be able to insert if they don't exist, otherwise discard.
          // because archive notes shouldn't change.
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
