use crate::config::State;
use crate::email;
use crate::errors;
use crate::icontent::{
  GetZkLinks, GetZkNoteEdit, ImportZkNote, SaveZkNote, SaveZkNotePlusLinks, UserId, ZkLinks,
  ZkNoteAndAccomplices,
};
use crate::indra as I;
use crate::isearch::{TagSearch, ZkNoteSearch};
use crate::util;
use crate::util::{is_token_expired, now};
use crypto_hash::{hex_digest, Algorithm};
use indradb::Datastore;
use log::{debug, error, info, log_enabled, Level};
use serde_derive::{Deserialize, Serialize};
use simple_error;
use std::error::Error;
use std::path::Path;
use uuid::Uuid;
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};

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
  state: &State,
  msg: UserMessage,
) -> Result<ServerResponse, errors::Error> {
  info!("got a user message: {}", msg.what);
  let itr = state.db.transaction()?;
  if msg.what.as_str() == "register" {
    let msgdata = Option::ok_or(msg.data, "malformed registration data")?;
    let rd: RegistrationData = serde_json::from_value(msgdata)?;
    // do the registration thing.
    // user already exists?
    match I::read_user(&itr, &rd.uid) {
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

        let svs = I::get_systemvs(&itr)?;

        // write a user record.
        I::new_user(
          &itr,
          &svs.public,
          rd.uid.clone(),
          hex_digest(
            Algorithm::SHA256,
            (rd.pwd + salt.as_str()).into_bytes().as_slice(),
          ),
          salt,
          rd.email.clone(),
          Some(registration_key.clone().to_string()),
        )?;

        // send a registration email.
        email::send_registration(
          state.config.appname.as_str(),
          state.config.domain.as_str(),
          state.config.mainsite.as_str(),
          rd.email.as_str(),
          rd.uid.as_str(),
          registration_key.as_str(),
        )?;

        // notify the admin.
        email::send_registration_notification(
          state.config.appname.as_str(),
          state.config.domain.as_str(),
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
    let conn = sqldata::connection_open(config.db.as_path())?;
    let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    let login: Login = serde_json::from_value(msgdata.clone())?;

    let mut userdata = sqldata::read_user(Path::new(&config.db), login.uid.as_str(), None)?;
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
          userdata.token = Some(Uuid::new_v4());
          userdata.tokendate = Some(now()?);
          session.set("token", userdata.token)?;
          sqldata::update_user(config.db.as_path(), &userdata)?;
          println!("logged in, userdata: {:?}", userdata);

          Ok(ServerResponse {
            what: "logged in".to_string(),
            content: serde_json::to_value(ld)?,
          })
        }
      }
    }
  } else {
    match session.get::<Uuid>("token")? {
      None => Ok(ServerResponse {
        what: "not logged in".to_string(),
        content: serde_json::Value::Null,
      }),
      Some(token) => {
        match I::read_user_by_token(&conn, token, Some(state.config.token_expiration_ms)) {
          Err(_) => Ok(ServerResponse {
            // TODO: token expired?
            what: "invalid user or pwd".to_string(),
            content: serde_json::Value::Null,
          }),
          Ok(userdata) => {
            // finally!  processing messages as logged in user.
            user_interface_loggedin(itr, &state, userdata.id, &msg)
          }
        }
      }
    }
  }
}

fn se(s: &str) -> simple_error::SimpleError {
  simple_error::SimpleError::new(s)
}

fn user_interface_loggedin<T: indradb::Transaction>(
  itr: &T,
  state: &State,
  uid: UserId,
  msg: &UserMessage,
) -> Result<ServerResponse, errors::Error> {
  match msg.what.as_str() {
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let id: Uuid = serde_json::from_value(msgdata.clone())?;
      let note = I::read_zknote(itr, &state.svs, Some(uid), id)?;
      Ok(ServerResponse {
        what: "zknote".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "getzknoteedit" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let id: Uuid = serde_json::from_value(msgdata.clone())?;
      let note = I::read_zknoteedit(itr, &state.svs, uid, id)?;
      Ok(ServerResponse {
        what: "zknoteedit".to_string(),
        content: serde_json::to_value(note)?,
      })
    }
    "searchzknotes" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let res = I::search_zknotes(itr, &state.svs, uid, &search)?;
      Ok(ServerResponse {
        what: "zknotesearchresult".to_string(),
        content: serde_json::to_value(res)?,
      })
    }
    // "powerdelete" => {
    //   let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
    //   let search: TagSearch = serde_json::from_value(msgdata.clone())?;
    //   let res = search::power_delete_zknotes(itr, uid, &search)?;
    //   Ok(ServerResponse {
    //     what: "powerdeletecomplete".to_string(),
    //     content: serde_json::to_value(res)?,
    //   })
    // }
    "deletezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let id: Uuid = serde_json::from_value(msgdata.clone())?;
      I::delete_zknote(itr, uid, id)?;
      Ok(ServerResponse {
        what: "deletedzknote".to_string(),
        content: serde_json::to_value(id)?,
      })
    }
    "savezknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let sbe: SaveZkNote = serde_json::from_value(msgdata.clone())?;
      let s = I::save_zknote(itr, uid, &sbe)?;
      Ok(ServerResponse {
        what: "savedzknote".to_string(),
        content: serde_json::to_value(s)?,
      })
    }
    // "savezklinks" => {
    //   let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    //   let msg: ZkLinks = serde_json::from_value(msgdata.clone())?;
    //   let s = I::save_zklinks(itr, uid, msg.links)?;
    //   Ok(ServerResponse {
    //     what: "savedzklinks".to_string(),
    //     content: serde_json::to_value(s)?,
    //   })
    // }
    "savezknotepluslinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let sznpl: SaveZkNotePlusLinks = serde_json::from_value(msgdata.clone())?;
      let szkn = I::save_zknote(itr, uid, &sznpl.note)?;
      let s = I::save_savezklinks(itr, uid, szkn.id, sznpl.links)?;
      Ok(ServerResponse {
        what: "savedzknotepluslinks".to_string(),
        content: serde_json::to_value(szkn)?,
      })
    }
    "getzklinks" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), se("malformed json data"))?;
      let noteid: Uuid = serde_json::from_value(msgdata.clone())?;
      let s = I::read_zklinks(itr, &state.svs, Some(uid), noteid)?;
      println!("zklinks: {:?}", s.len());
      let zklinks = ZkLinks { links: s };
      Ok(ServerResponse {
        what: "zklinks".to_string(),
        content: serde_json::to_value(zklinks)?,
      })
    }
    // "saveimportzknotes" => {
    //   let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    //   let gzl: Vec<ImportZkNote> = serde_json::from_value(msgdata.clone())?;
    //   I::save_importzknotes(itr, uid, gzl)?;
    //   Ok(ServerResponse {
    //     what: "savedimportzknotes".to_string(),
    //     content: serde_json::to_value(())?,
    //   })
    // }
    wat => Err(se(format!("invalid 'what' code:'{}'", wat).as_str()).into()),
  }
}

// public json msgs don't require login.
pub fn public_interface(
  state: &State,
  msg: PublicMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  info!("process_public_json, what={}", msg.what.as_str());
  match msg.what.as_str() {
    /*
    "getzknote" => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: i64 = serde_json::from_value(msgdata.clone())?;
      let itr = I::getTransaction(state.indradb.as_path())?;
      let note = I::read_zknote(&itr, None, id)?;
      if I::is_zknote_public(&itr, note.id)? {
        Ok(ServerResponse {
          what: "zknote".to_string(),
          content: serde_json::to_value(ZkNoteAndAccomplices {
            links: I::read_public_zklinks(&itr, note.id)?,
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
      let itr = I::getTransaction(state.indradb.as_path())?;
      let note = I::read_zknotepubid(&itr, None, pubid.as_str())?;
      if I::is_zknote_public(&itr, note.id)? {
        Ok(ServerResponse {
          what: "zknote".to_string(),
          content: serde_json::to_value(ZkNoteAndAccomplices {
            links: I::read_public_zklinks(&itr, note.id)?,
            zknote: note,
          })?,
        })
      } else {
        Ok(ServerResponse {
          what: "privatezknote".to_string(),
          content: serde_json::to_value(note.id)?,
        })
      }
    } */
    wat => Err(Box::new(simple_error::SimpleError::new(format!(
      "invalid 'what' code:'{}'",
      wat
    )))),
  }
}
