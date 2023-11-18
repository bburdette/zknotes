use crate::config::Config;
use crate::search;
use crate::sqldata;
use crate::util::now;
use actix_session::Session;
use either::Either::{Left, Right};
use log::info;
use orgauth;
use orgauth::data::{ChangeEmail, ChangePassword, LoginData, User, UserInvite};
use orgauth::dbfun::user_id;
use orgauth::endpoints::{Callbacks, Tokener};
use reqwest;
use reqwest::cookie;
use rusqlite::{params, Connection};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use zkprotocol::content::{
  ArchiveZkLink, GetArchiveZkLinks, GetArchiveZkNote, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, Sysids,
  ZkLinks, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult};

pub async fn sync(
  conn: &Connection,
  user: &User,
) -> Result<ServerResponse, Box<dyn std::error::Error>> {
  let now = now()?;

  // execute any command from here.  search?
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

  // let jar = reqwest::cookies::Jar;
  match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => {
      let url = reqwest::Url::parse(url.as_str())?;

      let jar = Arc::new(cookie::Jar::default());
      jar.add_cookie_str(c.as_str(), &url);
      let client = reqwest::Client::builder().cookie_provider(jar).build()?;

      let getarchivenotes = false;
      let getarchivelinks = true;

      if getarchivenotes {
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

        for note in searchres.notes {
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
          ],
        )?;
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

        for l in links {
          println!("archiving link!");
          conn.execute(
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
