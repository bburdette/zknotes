use crate::util::now;
use futures_util::TryStreamExt;
use orgauth;
use orgauth::data::{PhantomUser, User, UserResponse, UserResponseMessage};
use orgauth::dbfun::user_id;
use orgauth::endpoints::Callbacks;
use reqwest;
use reqwest::cookie;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;
use zkprotocol::constants::{PrivateReplies, PrivateRequests, PrivateStreamingRequests};
use zkprotocol::content::{ArchiveZkLink, GetArchiveZkLinks, GetZkLinksSince, UuidZkLink, ZkNote};
use zkprotocol::messages::{PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage};
use zkprotocol::search::{TagSearch, ZkNoteSearch};

fn convert_err(err: reqwest::Error) -> std::io::Error {
  todo!()
}

pub async fn sync(
  conn: &Connection,
  user: &User,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let now = now()?;

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

      if getnotes {
        let mut userhash = HashMap::<i64, i64>::new();

        let zns = ZkNoteSearch {
          tagsearch: TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "".to_string(),
          },
          offset: 0,
          limit: None,
          what: "".to_string(),
          list: false,
          archives: false,
          created_after: None,
          created_before: None,
          changed_after: None,
          changed_before: Some(now),
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

          println!("note line: {}", line);

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
                    "createing phantom user {} for remote user: {:?}",
                    pu.id, localpuid
                  );
                  userhash.insert(pu.id, localpuid);
                  localpuid
                }
              };
              localuserid
            }
          };

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
            ) {
              Ok(x) => Ok(x),
              Err(rusqlite::Error::SqliteFailure(e, s)) =>
                if e.code == rusqlite::ErrorCode::ConstraintViolation {
                  // TODO: uuid conflict;  resolve with old one becoming archive note.
                  // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
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
          list: false,
          archives: true,
          created_after: None,
          created_before: None,
          changed_after: None,
          changed_before: Some(now),
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

          println!("note line: {}", line);

          let note = serde_json::from_str::<ZkNote>(line.trim())?;

          count = count + 1;
          bytes = bytes + nc;

          conn.execute(
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
              ])?;
        }

        println!("synched {} archive notes, with {} bytes!", count, bytes);
      }

      if getarchivelinks {
        let gazl = GetArchiveZkLinks {
          createddate_after: 0,
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

          println!("note line: {}", line);

          let l = serde_json::from_str::<ArchiveZkLink>(line.trim())?;

          let ins = conn.execute(
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
          )?;

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
          createddate_after: 0,
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

          println!("note line: {}", line);

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
