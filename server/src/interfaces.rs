use config::Config;
use crypto_hash::{hex_digest, Algorithm};
use email;
use search;
use simple_error;
use sqldata;
use std::error::Error;
use std::path::Path;
use util;
use uuid::Uuid;
use zkprotocol::content::{
  GetZkLinks, GetZkNoteEdit, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, ZkLinks,
  ZkNoteAndAccomplices,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkNoteSearch};

use indra as I;

#[derive(Deserialize, Debug)]
pub struct RegistrationData {
  email: String,
}

pub fn user_interface(config: &Config, msg: UserMessage) -> Result<ServerResponse, Box<dyn Error>> {
  info!("got a user message: {}", msg.what);
  if msg.what.as_str() == "register" {
    // do the registration thing.
    // user already exists?
    match sqldata::read_user(Path::new(&config.db), msg.uid.as_str()) {
      Ok(_) => {
        // err - user exists.
        Ok(ServerResponse {
          what: "user exists".to_string(),
          content: serde_json::Value::Null,
        })
      }
      Err(_) => {
        // user does not exist, which is what we want for a new user.
        // get email from 'data'.
        let msgdata = Option::ok_or(msg.data, "malformed registration data")?;
        let rd: RegistrationData = serde_json::from_value(msgdata)?;
        let registration_key = Uuid::new_v4().to_string();
        let salt = util::salt_string();

        // write a user record.
        sqldata::new_user(
          Path::new(&config.db),
          msg.uid.clone(),
          hex_digest(
            Algorithm::SHA256,
            (msg.pwd + salt.as_str()).into_bytes().as_slice(),
          ),
          salt,
          rd.email.clone(),
          registration_key.clone().to_string(),
        )?;

        // send a registration email.
        email::send_registration(
          config.appname.as_str(),
          config.domain.as_str(),
          config.mainsite.as_str(),
          rd.email.as_str(),
          msg.uid.as_str(),
          registration_key.as_str(),
        )?;

        // notify the admin.
        email::send_registration_notification(
          config.appname.as_str(),
          config.domain.as_str(),
          "bburdettte@protonmail.com",
          rd.email.as_str(),
          msg.uid.as_str(),
          registration_key.as_str(),
        )?;

        Ok(ServerResponse {
          what: "registration sent".to_string(),
          content: serde_json::Value::Null,
        })
      }
    }
  } else {
    match sqldata::read_user(Path::new(&config.db), msg.uid.as_str()) {
      Err(_) => Ok(ServerResponse {
        what: "invalid user or pwd".to_string(),
        content: serde_json::Value::Null,
      }),
      Ok(userdata) => {
        // let userdata: User = serde_json::from_value(serde_json::from_str(udata.as_str())?)?;
        match userdata.registration_key {
          Some(_reg_key) => Ok(ServerResponse {
            what: "unregistered user".to_string(),
            content: serde_json::Value::Null,
          }),
          None => {
            if hex_digest(
              Algorithm::SHA256,
              (msg.pwd.clone() + userdata.salt.as_str())
                .into_bytes()
                .as_slice(),
            ) != userdata.hashwd
            {
              // don't distinguish between bad user id and bad pwd!
              Ok(ServerResponse {
                what: "invalid user or pwd".to_string(),
                content: serde_json::Value::Null,
              })
            } else {
              // finally!  processing messages as logged in user.
              user_interface_loggedin(&config, userdata.id, &msg)
            }
          }
        }
      }
    }
  }
}

fn user_interface_loggedin(
  config: &Config,
  uid: i64,
  msg: &UserMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  match msg.what.as_str() {
    "login" => {
      let conn = sqldata::connection_open(config.db.as_path())?;
      Ok(ServerResponse {
        what: "logged in".to_string(),
        content: serde_json::to_value(sqldata::login_data(&conn, uid)?)?,
      })
    }
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let note = sqldata::read_zknote(&conn, Some(uid), id)?;
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "getzknoteedit" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteEdit = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let note = sqldata::read_zknoteedit(&conn, uid, &gzne)?;
      Ok(ServerResponse {
        what: "zknoteedit".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "searchzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let res = search::search_zknotes(&conn, uid, &search)?;
      Ok(ServerResponse {
        what: "zknotesearchresult".to_string(),
        content: serde_json::to_value(res)?,
      })
    }
    "powerdelete" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: TagSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let res = search::power_delete_zknotes(&conn, uid, &search)?;
      Ok(ServerResponse {
        what: "powerdeletecomplete".to_string(),
        content: serde_json::to_value(res)?,
      })
    }
    "deletezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      sqldata::delete_zknote(Path::new(&config.db), uid, id)?;
      Ok(ServerResponse {
        what: "deletedzknote".to_string(),
        content: serde_json::to_value(id)?,
      })
    }
    "savezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sbe: SaveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let s = sqldata::save_zknote(&conn, uid, &sbe)?;
      Ok(ServerResponse {
        what: "savedzknote".to_string(),
        content: serde_json::to_value(s)?,
      })
    }
    "savezklinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let msg: ZkLinks = serde_json::from_value(msgdata.clone())?;
      let s = sqldata::save_zklinks(&config.db.as_path(), uid, msg.links)?;
      Ok(ServerResponse {
        what: "savedzklinks".to_string(),
        content: serde_json::to_value(s)?,
      })
    }
    "savezknotepluslinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sznpl: SaveZkNotePlusLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let szkn = sqldata::save_zknote(&conn, uid, &sznpl.note)?;
      let s = sqldata::save_savezklinks(&conn, uid, szkn.id, sznpl.links)?;
      Ok(ServerResponse {
        what: "savedzknotepluslinks".to_string(),
        content: serde_json::to_value(szkn)?,
      })
    }
    "getzklinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzl: GetZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let s = sqldata::read_zklinks(&conn, uid, &gzl)?;
      let zklinks = ZkLinks { links: s };
      Ok(ServerResponse {
        what: "zklinks".to_string(),
        content: serde_json::to_value(zklinks)?,
      })
    }
    "saveimportzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzl: Vec<ImportZkNote> = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      sqldata::save_importzknotes(&conn, uid, gzl)?;
      Ok(ServerResponse {
        what: "savedimportzknotes".to_string(),
        content: serde_json::to_value(())?,
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
) -> Result<ServerResponse, Box<dyn Error>> {
  info!("process_public_json, what={}", msg.what.as_str());
  match msg.what.as_str() {
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let note = sqldata::read_zknote(&conn, None, id)?;
      if sqldata::is_zknote_public(&conn, note.id)? {
        Ok(ServerResponse {
          what: "zknote".to_string(),
          content: serde_json::to_value(ZkNoteAndAccomplices {
            links: sqldata::read_public_zklinks(&conn, note.id)?,
            zknote: note,
          })?,
        })
      } else {
        Ok(ServerResponse {
          what: "privatezknote".to_string(),
          content: serde_json::to_value(note.id)?,
        })
      }
    }
    "getzknotepubid" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let pubid: String = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let note = sqldata::read_zknotepubid(&conn, None, pubid.as_str())?;
      if sqldata::is_zknote_public(&conn, note.id)? {
        Ok(ServerResponse {
          what: "zknote".to_string(),
          content: serde_json::to_value(ZkNoteAndAccomplices {
            links: sqldata::read_public_zklinks(&conn, note.id)?,
            zknote: note,
          })?,
        })
      } else {
        Ok(ServerResponse {
          what: "privatezknote".to_string(),
          content: serde_json::to_value(note.id)?,
        })
      }
    }
    wat => Err(Box::new(simple_error::SimpleError::new(format!(
      "invalid 'what' code:'{}'",
      wat
    )))),
  }
}
