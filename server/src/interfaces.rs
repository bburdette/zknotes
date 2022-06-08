use crate::config::Config;
use crate::search;
use crate::sqldata;
use actix_session::Session;
use actix_web::HttpRequest;
use either::Either::{Left, Right};
use log::info;
use orgauth::endpoints::Callbacks;
use std::error::Error;
use std::path::Path;
use zkprotocol::content::{
  GetZkNoteComments, GetZkNoteEdit, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, ZkLinks,
  ZkNoteEdit,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkNoteSearch};

pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<Option<orgauth::data::LoginData>, Box<dyn Error>> {
  let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
  let mut cb = Callbacks {
    on_new_user: Box::new(sqldata::on_new_user),
    extra_login_data: Box::new(sqldata::extra_login_data_callback),
  };
  match session.get("token")? {
    None => Ok(None),
    Some(token) => {
      match orgauth::dbfun::read_user_by_token(
        &conn,
        token,
        Some(config.orgauth_config.login_token_expiration_ms),
      ) {
        Ok(user) => Ok(Some(orgauth::dbfun::login_data_cb(
          &conn,
          user.id,
          &mut cb.extra_login_data,
          // Box::new(sqldata::extra_login_data_callback),
        )?)),
        Err(_) => Ok(None),
      }
    }
  }
}

// Just like orgauth::endpoints::user_interface, except adds in extra user data.
pub fn user_interface(
  session: &Session,
  config: &Config,
  msg: orgauth::data::WhatMessage,
) -> Result<orgauth::data::WhatMessage, Box<dyn Error>> {
  let mut cb = Callbacks {
    on_new_user: Box::new(sqldata::on_new_user),
    extra_login_data: Box::new(sqldata::extra_login_data_callback),
  };
  orgauth::endpoints::user_interface(&session, &config.orgauth_config, &mut cb, msg)
}

pub fn zk_interface_loggedin(
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
      let gzne: GetZkNoteEdit = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknoteedit(&conn, uid, &gzne)?;
      info!(
        "user#getzknoteedit: {} - {}",
        gzne.zknote, note.zknote.title
      );
      Ok(ServerResponse {
        what: "zknoteedit".to_string(),
        content: serde_json::to_value(note)?,
      })
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
    "powerdelete" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: TagSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let res = search::power_delete_zknotes(&conn, uid, &search)?;
      Ok(ServerResponse {
        what: "powerdeletecomplete".to_string(),
        content: serde_json::to_value(res)?,
      })
    }
    "deletezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      sqldata::delete_zknote(Path::new(&config.orgauth_config.db), uid, id)?;
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
    // read_zklinks no longer returns ZkLinks, its EditLinks now.
    // "getzklinks" => {
    //   let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    //   let gzl: GetZkLinks = serde_json::from_value(msgdata.clone())?;
    //   let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    //   let s = sqldata::read_zklinks(&conn, uid, &gzl)?;
    //   let zklinks = ZkLinks { links: s };
    //   Ok(ServerResponse {
    //     what: "zklinks".to_string(),
    //     content: serde_json::to_value(zklinks)?,
    //   })
    // }
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
  req: HttpRequest,
) -> Result<ServerResponse, Box<dyn Error>> {
  match msg.what.as_str() {
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknote(&conn, None, id)?;
      info!(
        "public#getzknote: {} - {} - {:?}",
        id,
        note.title,
        req.connection_info().realip_remote_addr()
      );
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(ZkNoteEdit {
          links: sqldata::read_public_zklinks(&conn, note.id)?,
          zknote: note,
        })?,
      })
    }
    "getzknotepubid" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let pubid: String = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknotepubid(&conn, None, pubid.as_str())?;
      info!(
        "public#getzknotepubid: {} - {} - {:?}",
        pubid,
        note.title,
        req.connection_info().realip_remote_addr()
      );
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(ZkNoteEdit {
          links: sqldata::read_public_zklinks(&conn, note.id)?,
          zknote: note,
        })?,
      })
    }
    wat => Err(Box::new(simple_error::SimpleError::new(format!(
      "invalid 'what' code:'{}'",
      wat
    )))),
  }
}
