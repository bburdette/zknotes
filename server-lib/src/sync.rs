use crate::config::Config;
use crate::search;
use crate::sqldata;
use crate::util::now;
use actix_session::Session;
use either::Either::{Left, Right};
use log::info;
use orgauth;
use orgauth::data::WhatMessage;
use orgauth::data::{ChangeEmail, ChangePassword, LoginData, PhantomUser, User, UserInvite};
use orgauth::dbfun::user_id;
use orgauth::endpoints::{Callbacks, Tokener};
use reqwest;
use reqwest::cookie;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use uuid;
use zkprotocol::content::{
    GetZkLinksSince , UuidZkLink,  ArchiveZkLink,ZkLink, GetArchiveZkLinks, GetArchiveZkNote, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, Sysids,
  ZkLinks, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult};

pub async fn sync(
  conn: &Connection,
  user: &User,
  callbacks: &mut Callbacks,
) -> Result<ServerResponse, Box<dyn std::error::Error>> {
  let now = now()?;

  // execute any command from here.  search?
  // let jar = reqwest::cookies::Jar;
  match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => {
      let url = reqwest::Url::parse(url.as_str())?;

      let jar = Arc::new(cookie::Jar::default());
      jar.add_cookie_str(c.as_str(), &url);
      let client = reqwest::Client::builder().cookie_provider(jar).build()?;

      let getnotes = false;
      let getlinks = true;
      let getarchivenotes = false;
      let getarchivelinks = false;

      if getnotes {
        let mut offset: i64 = 0;
        let step: i64 = 50;
        let mut userhash = HashMap::<i64, i64>::new();
        loop {
          let zns = ZkNoteSearch {
            tagsearch: TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "".to_string(),
            },
            offset: offset,
            limit: Some(step),
            what: "".to_string(),
            list: false,
            archives: false,
            created_after: None,
            created_before: None,
            changed_after: None,
            changed_before: Some(now),
          };

          let l = UserMessage {
            what: "searchzknotes".to_string(),
            data: Some(serde_json::to_value(zns)?),
          };

          let private_url = reqwest::Url::parse(
            format!("{}/private", url.origin().unicode_serialization(),).as_str(),
          )?;

          let res = client.post(private_url).json(&l).send().await?;

          // should recieve a whatmessage, yes?

          let resp = res.json::<ServerResponse>().await?;
          let searchres: ZkNoteSearchResult = match resp.what.as_str() {
            "zknotesearchresult" => serde_json::from_value(resp.content).map_err(|e| e.into()),
            _ => Err::<ZkNoteSearchResult, Box<dyn std::error::Error>>(
              format!("unexpected message: {:?}", resp).into(),
            ),
          }?;

          println!("searchres: {:?}", searchres);

          // write the notes!
          let sysid = user_id(&conn, "system")?;

          // map of remote user ids to local user ids.
          let user_url = reqwest::Url::parse(
            format!("{}/user", url.origin().unicode_serialization(),).as_str(),
          )?;

          for note in &searchres.notes {
            // got this user already?
            let user_uid: i64 = match userhash.get(&note.user) {
              Some(u) => *u,
              None => {
                println!("fetching remote user: {:?}", note.user);
                // fetch a remote user record.
                let res = client
                  .post(user_url.clone())
                  .json(&UserMessage {
                    what: "read_remote_user".to_string(),
                    data: Some(serde_json::to_value(note.user)?),
                  })
                  .send()
                  .await?;
                let wm: WhatMessage = serde_json::from_value(res.json().await?)?;
                println!("remote user wm: {:?}", wm);
                let pu: PhantomUser = match wm.what.as_str() {
                  "remote_user" => serde_json::from_value(
                    wm.data
                      .ok_or::<orgauth::error::Error>("missing data".into())?,
                  )?, // .map_err(|e| e.into())?,
                  _ => Err::<PhantomUser, Box<dyn std::error::Error>>(
                    orgauth::error::Error::String(format!("unexpected message: {:?}", wm)).into(),
                  )?,
                };
                println!("phantom user: {:?}", pu);
                let localuserid = match orgauth::dbfun::read_user_by_uuid(&conn, pu.uuid.as_str()) {
                  Ok(user) => {
                    println!("found local user {} for remote {}", user.id, pu.id);
                    userhash.insert(pu.id, user.id);
                    user.id
                  }
                  _ => {
                    let localpuid = orgauth::dbfun::phantom_user(
                      &conn,
                      pu.name,
                      uuid::Uuid::parse_str(pu.uuid.as_str())?,
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
                note.uuid,
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
          if searchres.notes.len() < step as usize {
            println!("notes loop complete");
            break;
          } else {
            offset = offset + step;
          }
        }
      }
      if getarchivenotes {
        let mut offset: i64 = 0;
        let step: i64 = 50;

        loop {
          println!("reading notes: offset {}", offset);
          let zns = ZkNoteSearch {
            tagsearch: TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "".to_string(),
            },
            offset: offset,
            limit: Some(step),
            what: "".to_string(),
            list: false,
            archives: true,
            created_after: None,
            created_before: None,
            changed_after: None,
            changed_before: Some(now),
          };

          let l = UserMessage {
            what: "searchzknotes".to_string(),
            data: Some(serde_json::to_value(zns)?),
          };

          let actual_url = reqwest::Url::parse(
            format!("{}/private", url.origin().unicode_serialization(),).as_str(),
          )?;

          let res = client.post(actual_url).json(&l).send().await?;

          // should recieve a whatmessage, yes?

          let resp = res.json::<ServerResponse>().await?;
          let searchres: ZkNoteSearchResult = match resp.what.as_str() {
            "zknotesearchresult" => serde_json::from_value(resp.content).map_err(|e| e.into()),
            _ => Err::<ZkNoteSearchResult, Box<dyn std::error::Error>>(
              format!("unexpected message: {:?}", resp).into(),
            ),
          }?;

          println!("searchres: {:?}", searchres);

          // write the notes!
          let sysid = user_id(&conn, "system")?;

          for note in &searchres.notes {
            // if we had a sha, we'd insert based on that, right?
            // these are archive notes so should be able to insert if they don't exist, otherwise discard.
            // because archive notes shouldn't change.
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
              note.uuid,
              note.createdate,
              note.changeddate,
              ])?;
          }
          if searchres.notes.len() < step as usize {
            println!("notes loop complete");
            break;
          } else {
            offset = offset + step;
          }
        }
      }

      if getarchivelinks {
        let gazl = GetArchiveZkLinks {
          createddate_after: 0,
        };
        let l = UserMessage {
          what: "getarchivezklinks".to_string(),
          data: Some(serde_json::to_value(gazl)?),
        };

        let actual_url = reqwest::Url::parse(
          format!("{}/private", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(actual_url).json(&l).send().await?;

        // Ok now get the archive links.  Special query!
        let resp = res.json::<ServerResponse>().await?;
        let links: Vec<ArchiveZkLink> = match resp.what.as_str() {
          "archivezklinks" => serde_json::from_value(resp.content).map_err(|e| e.into()),
          _ => Err::<Vec<ArchiveZkLink>, Box<dyn std::error::Error>>(
            format!("unexpected messge: {:?}", resp).into(),
          ),
        }?;

        println!("receieved links: {}", links.len());

        for l in links {
          println!("archiving link!, {:?}", l);
          let count = conn.execute(
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
          println!("archived link count:, {}", count);
        }
      }

      if getlinks {
        let gazl = GetZkLinksSince {
          createddate_after: 0,
        };
        let l = UserMessage {
          what: "getzklinkssince".to_string(),
          data: Some(serde_json::to_value(gazl)?),
        };

        let actual_url = reqwest::Url::parse(
          format!("{}/private", url.origin().unicode_serialization(),).as_str(),
        )?;

        let res = client.post(actual_url).json(&l).send().await?;

        // Ok now get the archive links.  Special query!
        let resp = res.json::<ServerResponse>().await?;
        let links: Vec<UuidZkLink> = match resp.what.as_str() {
          "zklinks" => serde_json::from_value(resp.content).map_err(|e| e.into()),
          _ => Err::<Vec<UuidZkLink>, Box<dyn std::error::Error>>(
            format!("unexpected messge: {:?}", resp).into(),
          ),
        }?;

        println!("receieved links: {}", links.len());

        for l in links {
          println!("saving link!, {:?}", l);
          let count = conn.execute(
            "insert into zklink (fromid, toid, user, linkzknote, createdate)
              select FN.id, TN.id, U.id, LN.id, ?1
              from zknote FN, zknote TN, orgauth_user U, zknote LN
              where FN.uuid = ?2
                and TN.uuid = ?3
                and U.uuid = ?4
                and LN.uuid = ?5",
            params![
              l.createdate,
              l.fromUuid,
              l.toUuid,
              l.userUuid,
              l.linkUuid
            ],
          )?;
          println!("archived link count:, {}", count);
        }
      }

      // TODO update cookie?
      Ok(ServerResponse {
        what: "synccomplete".to_string(),
        content: serde_json::Value::Null,
      })
    }
    _ => Err("can't remote sync".into()),
  }

  // let jar = reqwest::cookies::Jar;
  // match (user.cookie, user.remote_url) {}
}
