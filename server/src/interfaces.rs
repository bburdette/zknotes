// extern crate crypto_hash;
// // extern crate bail! extern crate lettre;
// extern crate lettre_email;
// extern crate rand;
// extern crate serde_json;
// extern crate time;
// extern crate uuid;

use crypto_hash::{hex_digest, Algorithm};
use email;
// use bail!;
// use meta_tag_base;
// use meta_tag_base::{CopyTagBase, MetaTagBase, PubTagBase};
use serde_json::Value;
use simple_error;
use sqldata::User;
use std::error::Error;
use std::sync::{Arc, RwLock};
use util;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

#[derive(Clone)]
pub struct AppState {
  pub htmlstring: String,
  // pub publicmtb: Arc<RwLock<MetaTagBase>>,
}

#[derive(Deserialize, Debug)]
pub struct Message {
  pub uid: String,
  pwd: String,
  what: String,
  data: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct PublicMessage {
  what: String,
  data: Option<serde_json::Value>,
}

/*
#[derive(Deserialize, Debug)]
pub struct PublicTbReq {
  // tagbase id, and what we want it for.
  tbid: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicTagBaseMessage {
  tbid: i64,
  tbname: String,
  tagbase: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ImportedTb {
  usertbid: i64,
  publictbid: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TagBaseMessage {
  // whole elm db is here, but all we care about is the saveid.
  saveid: i64,
  // all we care about is the saveid, and the tbid.  Just those!
  tbid: i64,
  tbfor: String,
  tagbase: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MtbData {
  // with saving MTB, only want the saveid.
  saveid: i64,
  tagbases: serde_json::Value,
  tags: serde_json::Value,
}

#[derive(Deserialize, Debug)]
pub struct TbReq {
  // tagbase id, and what we want it for.
  tbid: i64,
  tbfor: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NoData {
  // requested a db, but got this.
  tbid: i64,
}
*/

#[derive(Deserialize, Debug)]
pub struct RegistrationData {
  email: String,
}

/*
pub fn user_interface(
  // publicmtb: &RwLock<MetaTagBase>,
  pdfdb: &str,
  msg: Message,
) -> Result<ServerResponse, Box<Error>> {
  info!("got a message: {}", msg.what);
  //  let msg: Message = serde_json::from_value(v)?;
  if msg.what.as_str() == "register"
  // do the registration thing.
  {
    // user already exists?
    match util::load_string(format!("users/{}.txt", msg.uid.as_str()).as_str()) {
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
        let msgdata = Option::ok_or(msg.data, bail!("malformed registration data"))?;
        let rd: RegistrationData = serde_json::from_value(msgdata)?;
        // TODO: make a real registration key
        let registration_key = Uuid::new_v4().to_string();
        // write a user record.
        let salt = util::salt_string();
        let user = User {
          id: msg.uid,
          hashwd: hex_digest(
            Algorithm::SHA256,
            (msg.pwd + salt.as_str()).into_bytes().as_slice(),
          ),
          salt: salt,
          email: rd.email,
          registration_key: Some(registration_key.clone().to_string()),
          current_tb: None,
        };
        // save the user record.
        util::write_string(
          serde_json::to_value(&user)?.to_string().as_str(),
          format!("users/{}.txt", user.uid).as_str(),
        )?;
        // send a registration email.
        email::send_registration(
          user.email.as_str(),
          user.uid.as_str(),
          registration_key.as_str(),
        )?;
        email::send_registration_notification(
          "bburdettte@protonmail.com",
          user.email.as_str(),
          user.uid.as_str(),
          registration_key.as_str(),
        )?;
        Ok(ServerResponse {
          what: "registration sent".to_string(),
          content: serde_json::Value::Null,
        })
      }
    }
  } else {
    match util::load_string(format!("users/{}.txt", msg.uid.as_str()).as_str()) {
      Err(_) => Ok(ServerResponse {
        what: "invalid user or pwd".to_string(),
        content: serde_json::Value::Null,
      }),
      Ok(udata) => {
        let userdata: User = serde_json::from_value(serde_json::from_str(udata.as_str())?)?;
        match userdata.registration_key {
          Some(_reg_key) => Ok(ServerResponse {
            what: "unregistered user".to_string(),
            content: serde_json::Value::Null,
          }),
          None => {
            if hex_digest(
              Algorithm::SHA256,
              (msg.pwd + userdata.salt.as_str()).into_bytes().as_slice(),
            ) != userdata.hashwd
            {
              // don't distinguish between bad user id and bad pwd!
              Ok(ServerResponse {
                what: "invalid user or pwd".to_string(),
                content: serde_json::Value::Null,
              })
            } else {
              info!("password match!");
              match msg.what.as_str() {
                /*                "gettagbase" => match msg.data {
                  None => Ok(ServerResponse {
                    what: "no tagbase specified!".to_string(),
                    content: serde_json::Value::Null,
                  }),
                  Some(data) => {
                    let tbr: TbReq = serde_json::from_value(data)?;
                    let mbsaveid = match util::load_string(
                      format!("data/{}-tb-{}.saveid", msg.uid.as_str(), tbr.tbid).as_str(),
                    ) {
                      Ok(s) => match s.parse::<i64>() {
                        Ok(i) => Some(i),
                        Err(_) => None,
                      },
                      Err(_) => Some(0),
                    };
                    info!("gettagbase:{} for {}", tbr.tbid, tbr.tbfor);
                    match util::load_string(
                      format!("data/{}-tb-{}.txt", msg.uid.as_str(), tbr.tbid).as_str(),
                    ) {
                      Ok(data) => match mbsaveid {
                        None => Ok(ServerResponse {
                          what: "saveid error".to_string(),
                          content: serde_json::Value::Null,
                        }),
                        Some(saveid) => Ok(ServerResponse {
                          what: "tagbase".to_string(),
                          content: serde_json::to_value(TagBaseMessage {
                            saveid: saveid,
                            tbid: tbr.tbid,
                            tbfor: tbr.tbfor,
                            tagbase: serde_json::from_str(data.as_str())?,
                          })?,
                        }),
                      },
                      Err(e) => {
                        error!("load error {:?}", e);
                        let nd = NoData { tbid: tbr.tbid };
                        Ok(ServerResponse {
                          what: "nodata".to_string(),
                          content: serde_json::to_value(nd)?,
                        })
                      }
                    }
                  }
                }*/
                wat => Err(bail!(format!("invalid 'what' code:'{}'", wat))),
              }
            }
          }
        }
      }
    }
  }
}
*/

// public json msgs don't require login.
pub fn public_interface(
  // publicmtb: &RwLock<MetaTagBase>,
  pdfdb: &str,
  msg: PublicMessage,
) -> Result<ServerResponse, Box<Error>> {
  info!("process_public_json, what={}", msg.what.as_str());
  match msg.what.as_str() {
    // // get Meta Tag Base = getmtb
    // "getmtb" => {
    //   let mtb = publicmtb.read().unwrap();
    //   match serde_json::to_value(&*mtb) {
    //     Ok(val) => Ok(ServerResponse {
    //       what: "publicmtb".to_string(),
    //       content: val,
    //     }),
    //     Err(e) => {
    //       error!("public mtb serialize error {:?}", e);
    //       Ok(ServerResponse {
    //         what: "publicmtbnotfound".to_string(),
    //         content: serde_json::Value::Null,
    //       })
    //     }
    //   }
    // }
    wat => Err(bail!(format!("invalid 'what' code:'{}'", wat))),
  }
}

/*pub fn load_user(uid: &str) -> Result<User, Error> {
    serde_json::from_value(serde_json::from_str(
        util::load_string(format!("users/{}.txt", uid).as_str())?.as_str(),
    )?)
    .map_err(|e| bail!(e.to_string()))
}

pub fn write_user(user: &User) -> Result<usize, Error> {
    util::write_string(
        serde_json::to_value(&user)?.to_string().as_str(),
        format!("users/{}.txt", user.uid).as_str(),
    )
}
*/
