use crate::config::Config;
use crate::email;
use crate::search;
use crate::sqldata;
use crate::util;
use actix_session::Session;
use crypto_hash::{hex_digest, Algorithm};
use either::Either::{Left, Right};
use log::info;
use simple_error;
use std::error::Error;
use std::path::Path;
use uuid::Uuid;
use zkprotocol::content::{
  ChangePassword, GetZkNoteComments, GetZkNoteEdit, ImportZkNote, Login, LoginData,
  RegistrationData, SaveZkNote, SaveZkNotePlusLinks, ZkLinks, ZkNoteEdit,
};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use zkprotocol::search::{TagSearch, ZkNoteSearch};

pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<Option<LoginData>, Box<dyn Error>> {
  let conn = sqldata::connection_open(config.db.as_path())?;

  match session.get("token")? {
    None => Ok(None),
    Some(token) => {
      match sqldata::read_user_by_token(&conn, token, Some(config.token_expiration_ms)) {
        Ok(user) => Ok(Some(sqldata::login_data(&conn, user.id)?)),
        Err(_) => Ok(None),
      }
    }
  }
}

pub fn user_interface(
  session: &Session,
  config: &Config,
  msg: UserMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  info!("got a user message: {}", msg.what);
  let conn = sqldata::connection_open(config.db.as_path())?;
  if msg.what.as_str() == "register" {
    let msgdata = Option::ok_or(msg.data, "malformed registration data")?;
    let rd: RegistrationData = serde_json::from_value(msgdata)?;
    // do the registration thing.
    // user already exists?
    match sqldata::read_user_by_name(&conn, rd.uid.as_str()) {
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
        let registration_key = Uuid::new_v4().to_string();
        let salt = util::salt_string();

        // write a user record.
        sqldata::new_user(
          Path::new(&config.db),
          rd.uid.clone(),
          hex_digest(
            Algorithm::SHA256,
            (rd.pwd + salt.as_str()).into_bytes().as_slice(),
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
          rd.uid.as_str(),
          registration_key.as_str(),
        )?;

        // notify the admin.
        email::send_registration_notification(
          config.appname.as_str(),
          config.domain.as_str(),
          config.admin_email.as_str(),
          rd.email.as_str(),
          rd.uid.as_str(),
          registration_key.as_str(),
        )?;

        Ok(ServerResponse {
          what: "registration sent".to_string(),
          content: serde_json::Value::Null,
        })
      }
    }
  } else if msg.what == "login" {
    let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    let login: Login = serde_json::from_value(msgdata.clone())?;

    let userdata = sqldata::read_user_by_name(&conn, login.uid.as_str())?;
    match userdata.registration_key {
      Some(_reg_key) => Ok(ServerResponse {
        what: "unregistered user".to_string(),
        content: serde_json::Value::Null,
      }),
      None => {
        if hex_digest(
          Algorithm::SHA256,
          (login.pwd.clone() + userdata.salt.as_str())
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
          let ld = sqldata::login_data(&conn, userdata.id)?;
          // new token here, and token date.
          let token = Uuid::new_v4();
          sqldata::add_token(&conn, userdata.id, token)?;
          session.set("token", token)?;
          sqldata::update_user(&conn, &userdata)?;
          println!("logged in, user: {:?}", userdata.name);

          Ok(ServerResponse {
            what: "logged in".to_string(),
            content: serde_json::to_value(ld)?,
          })
        }
      }
    }
  } else if msg.what == "logout" {
    session.remove("token");

    Ok(ServerResponse {
      what: "logged out".to_string(),
      content: serde_json::Value::Null,
    })
  } else {
    match session.get::<Uuid>("token")? {
      None => Ok(ServerResponse {
        what: "not logged in".to_string(),
        content: serde_json::Value::Null,
      }),
      Some(token) => {
        match sqldata::read_user_by_token(&conn, token, Some(config.token_expiration_ms)) {
          Err(e) => {
            println!("rubt error: {:?}", e);

            Ok(ServerResponse {
              what: "invalid user or pwd".to_string(),
              content: serde_json::Value::Null,
            })
          }
          Ok(userdata) => {
            // finally!  processing messages as logged in user.
            user_interface_loggedin(&config, userdata.id, &msg)
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
    "ChangePassword" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let cp: ChangePassword = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      sqldata::change_password(&conn, uid, cp)?;
      Ok(ServerResponse {
        what: "changed password".to_string(),
        content: serde_json::Value::Null,
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
    "getzknotecomments" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteComments = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
      let notes = sqldata::read_zknotecomments(&conn, uid, &gzne)?;
      Ok(ServerResponse {
        what: "zknotecomments".to_string(),
        content: serde_json::to_value(notes)?,
      })
    }
    "searchzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.db.as_path())?;
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
    //   let conn = sqldata::connection_open(config.db.as_path())?;
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
          content: serde_json::to_value(ZkNoteEdit {
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
          content: serde_json::to_value(ZkNoteEdit {
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
