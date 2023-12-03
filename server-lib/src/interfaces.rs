use crate::config::Config;
use crate::search;
use crate::sqldata;
use crate::sync;
use actix_session::Session;
use actix_web::HttpResponse;
use either::Either::{Left, Right};
use log::info;
use orgauth;
use orgauth::endpoints::{Callbacks, Tokener};
use orgauth::util::now;
// use reqwest;
use futures_util::Stream;
use search::ZkNoteStream;
use std::error::Error;
use std::time::Duration;
use zkprotocol::content::{
  GetArchiveZkLinks, GetArchiveZkNote, GetZkLinksSince, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, Sysids,
  ZkLinks, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkListNoteSearchResult, ZkNoteSearch};

pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<(Option<orgauth::data::LoginData>, Sysids), Box<dyn Error>> {
  let mut conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
  conn.busy_timeout(Duration::from_millis(500))?;

  let sysids = sqldata::read_sysids(&conn)?;

  let mut cb = Callbacks {
    on_new_user: Box::new(sqldata::on_new_user),
    extra_login_data: Box::new(sqldata::extra_login_data_callback),
    on_delete_user: Box::new(sqldata::on_delete_user),
  };
  let ldopt = match session.get("token")? {
    None => None,
    Some(token) => {
      match orgauth::dbfun::read_user_with_token_pageload(
        &mut conn,
        &session,
        token,
        config.orgauth_config.regen_login_tokens,
        config.orgauth_config.login_token_expiration_ms,
      ) {
        Ok(user) => {
          if user.active {
            Some(orgauth::dbfun::login_data_cb(
              &conn,
              user.id,
              &mut cb.extra_login_data,
            )?)
          } else {
            None
          }
        }
        Err(_e) => None, // Err(e.into()),
      }
    }
  };
  Ok((ldopt, sysids))
}

pub fn zknotes_callbacks() -> Callbacks {
  Callbacks {
    on_new_user: Box::new(sqldata::on_new_user),
    extra_login_data: Box::new(sqldata::extra_login_data_callback),
    on_delete_user: Box::new(sqldata::on_delete_user),
  }
}

// Just like orgauth::endpoints::user_interface, except adds in extra user data.
pub async fn user_interface(
  tokener: &mut dyn Tokener,
  config: &Config,
  msg: orgauth::data::WhatMessage,
) -> Result<orgauth::data::WhatMessage, Box<dyn Error>> {
  Ok(
    orgauth::endpoints::user_interface(
      tokener,
      &config.orgauth_config,
      &mut zknotes_callbacks(),
      msg,
    )
    .await?,
  )
}

// Ok(
//   HttpResponse::Ok()
//     .content_type("application/json")
//     .streaming(),
// )

pub async fn zk_interface_loggedin_streaming(
  config: &Config,
  uid: i64,
  msg: &UserMessage,
) -> Result<HttpResponse, Box<dyn Error>> {
  match msg.what.as_str() {
    "searchzknotesstream" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      // let mut znsm = ZnsMaker::init(conn, uid, &search)?;
      // Ok(HttpResponse::Ok().streaming(znsm.into_iter()))
      // {
      //   // borrowed value of znsm doesn't live long enough!  wat do?
      //   let znsstream = &znsm.make_stream(&conn)?;
      // }
      Err("wat".into())
    }
    wat => Err(format!("invalid 'what' code:'{}'", wat).into()),
  }
}

pub async fn zk_interface_loggedin(
  config: &Config,
  uid: i64,
  msg: &UserMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  match msg.what.as_str() {
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknote(&conn, Some(uid), id)?;
      info!("user#getzknote: {} - {}", id, note.title);
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "getzknoteedit" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknoteedit(&conn, Some(uid), gzne.zknote)?;
      info!(
        "user#getzknoteedit: {} - {}",
        gzne.zknote, note.zknote.title
      );

      let znew = ZkNoteAndLinksWhat {
        what: gzne.what,
        znl: note,
      };

      Ok(ServerResponse {
        what: "zknoteedit".to_string(),
        content: serde_json::to_value(znew)?,
      })
    }
    "getzneifchanged" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzic: GetZnlIfChanged = serde_json::from_value(msgdata.clone())?;
      info!(
        "user#getzneifchanged: {} - {}",
        gzic.zknote, gzic.changeddate
      );
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, Some(uid), &gzic)?;

      match ozkne {
        Some(zkne) => Ok(ServerResponse {
          what: "zknoteedit".to_string(),
          content: serde_json::to_value(ZkNoteAndLinksWhat {
            what: gzic.what,
            znl: zkne,
          })?,
        }),
        None => Ok(ServerResponse {
          what: "noop".to_string(),
          content: serde_json::Value::Null,
        }),
      }
    }
    "getzknotecomments" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteComments = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotecomments(&conn, uid, &gzne)?;
      Ok(ServerResponse {
        what: "zknotecomments".to_string(),
        content: serde_json::to_value(notes)?,
      })
    }
    "getzknotearchives" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteArchives = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotearchives(&conn, uid, &gzne)?;
      let zlnsr = ZkListNoteSearchResult {
        notes: notes,
        offset: gzne.offset,
        what: "archives".to_string(),
      };
      let zka = ZkNoteArchives {
        zknote: gzne.zknote,
        results: zlnsr,
      };
      Ok(ServerResponse {
        what: "zknotearchives".to_string(),
        content: serde_json::to_value(zka)?,
      })
    }
    "getarchivezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_archivezknote(&conn, uid, &rq)?;
      info!("user#getarchivezknote: {} - {}", note.id, note.title);
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "getarchivezklinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let links = sqldata::read_archivezklinks(&conn, uid, rq.createddate_after)?;
      Ok(ServerResponse {
        what: "archivezklinks".to_string(),
        content: serde_json::to_value(links)?,
      })
    }
    "getzklinkssince" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let links = sqldata::read_zklinks_since(&conn, uid, rq.createddate_after)?;
      Ok(ServerResponse {
        what: "zklinks".to_string(),
        content: serde_json::to_value(links)?,
      })
    }
    "searchzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      // let res = search::search_zknotes_simple(&conn, uid, &search)?;
      let res = search::search_zknotes(&conn, uid, &search)?;
      match res {
        Left(res) => Ok(ServerResponse {
          what: "zklistnotesearchresult".to_string(),
          content: serde_json::to_value(res)?,
        }),
        Right(res) => Ok(ServerResponse {
          what: "zknotesearchresult".to_string(),
          content: serde_json::to_value(res)?,
        }),
      }
    }
    // "searchzknotesstream" => {
    //   let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    //   let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
    //   let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    //   // let res = search::search_zknotes_simple(&conn, uid, &search)?;
    //   let res = search::search_zknotes_stream(&conn, uid, &search)?;
    //   HttpResponse::Ok()
    //     .content_type("application/json")
    //     .streaming(res)?
    //   // match res {
    //   //   Left(res) => Ok(ServerResponse {
    //   //     what: "zklistnotesearchresult".to_string(),
    //   //     content: serde_json::to_value(res)?,
    //   //   }),
    //   //   Right(res) => Ok(ServerResponse {
    //   //     what: "zknotesearchresult".to_string(),
    //   //     content: serde_json::to_value(res)?,
    //   //   }),
    //   // }
    // }
    "powerdelete" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: TagSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let res = search::power_delete_zknotes(&conn, config.file_path.clone(), uid, &search)?;
      Ok(ServerResponse {
        what: "powerdeletecomplete".to_string(),
        content: serde_json::to_value(res)?,
      })
    }
    "deletezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      sqldata::delete_zknote(&conn, config.file_path.clone(), uid, id)?;
      Ok(ServerResponse {
        what: "deletedzknote".to_string(),
        content: serde_json::to_value(id)?,
      })
    }
    "savezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sbe: SaveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let s = sqldata::save_zknote(&conn, uid, &sbe)?;
      Ok(ServerResponse {
        what: "savedzknote".to_string(),
        content: serde_json::to_value(s)?,
      })
    }
    "savezklinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let msg: ZkLinks = serde_json::from_value(msgdata.clone())?;
      let s = sqldata::save_zklinks(&config.orgauth_config.db.as_path(), uid, msg.links)?;
      Ok(ServerResponse {
        what: "savedzklinks".to_string(),
        content: serde_json::to_value(s)?,
      })
    }
    "savezknotepluslinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sznpl: SaveZkNotePlusLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let szkn = sqldata::save_zknote(&conn, uid, &sznpl.note)?;
      let _s = sqldata::save_savezklinks(&conn, uid, szkn.id, sznpl.links)?;
      Ok(ServerResponse {
        what: "savedzknotepluslinks".to_string(),
        content: serde_json::to_value(szkn)?,
      })
    }
    "saveimportzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzl: Vec<ImportZkNote> = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      sqldata::save_importzknotes(&conn, uid, gzl)?;
      Ok(ServerResponse {
        what: "savedimportzknotes".to_string(),
        content: serde_json::to_value(())?,
      })
    }
    "sethomenote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let hn: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let mut user = sqldata::read_user_by_id(&conn, uid)?;
      user.homenoteid = Some(hn);
      sqldata::update_user(&conn, &user)?;
      Ok(ServerResponse {
        what: "homenoteset".to_string(),
        content: serde_json::to_value(hn)?,
      })
    }
    "syncremote" => {
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let user = orgauth::dbfun::read_user_by_id(&conn, uid)?; // TODO pass this in from calling ftn?

      sync::sync(&conn, &user, &mut zknotes_callbacks()).await
    }
    wat => Err(Box::new(simple_error::SimpleError::new(format!(
      "invalid 'what' code:'{}'",
      wat
    )))),
  }
}

// public json msgs don't require login.
pub fn public_interface(
  config: &Config,
  msg: PublicMessage,
  ipaddr: Option<&str>,
) -> Result<ServerResponse, Box<dyn Error>> {
  match msg.what.as_str() {
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      // let note = sqldata::read_zknote(&conn, None, id)?;
      let note = sqldata::read_zknote(&conn, None, gzne.zknote)?;
      info!(
        "public#getzknote: {} - {} - {:?}",
        gzne.zknote, note.title, ipaddr
      );
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(ZkNoteAndLinksWhat {
          what: gzne.what,
          znl: ZkNoteAndLinks {
            links: sqldata::read_public_zklinks(&conn, note.id)?,
            zknote: note,
          },
        })?,
      })
    }
    "getzneifchanged" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzic: GetZnlIfChanged = serde_json::from_value(msgdata.clone())?;
      info!(
        "user#getzneifchanged: {} - {}",
        gzic.zknote, gzic.changeddate
      );
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, None, &gzic)?;

      match ozkne {
        Some(zkne) => Ok(ServerResponse {
          what: "zknoteedit".to_string(),
          content: serde_json::to_value(ZkNoteAndLinksWhat {
            what: gzic.what,
            znl: zkne,
          })?,
        }),
        None => Ok(ServerResponse {
          what: "noop".to_string(),
          content: serde_json::Value::Null,
        }),
      }
    }
    "getzknotepubid" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let pubid: String = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknotepubid(&conn, None, pubid.as_str())?;
      info!(
        "public#getzknotepubid: {} - {} - {:?}",
        pubid, note.title, ipaddr,
      );
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(ZkNoteAndLinksWhat {
          what: "".to_string(),
          znl: ZkNoteAndLinks {
            links: sqldata::read_public_zklinks(&conn, note.id)?,
            zknote: note,
          },
        })?,
      })
    }
    wat => Err(Box::new(simple_error::SimpleError::new(format!(
      "invalid 'what' code:'{}'",
      wat
    )))),
  }
}
