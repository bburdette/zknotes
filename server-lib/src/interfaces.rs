use crate::appdata::AppData;
use crate::appdata::Config;
use crate::appdata::TokenInfo;
use crate::error as zkerr;
use crate::search;
use crate::sqldata;
use crate::sync;
use actix::{ActorContext, StreamHandler};
use actix_session::Session;
use actix_web::HttpResponse;
use actix_web_actors::ws;
use log::info;
use orgauth;
use orgauth::endpoints::{Callbacks, Tokener};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::constants::PrivateReplies;
use zkprotocol::constants::PublicReplies;
use zkprotocol::constants::{PrivateRequests, PrivateStreamingRequests, PublicRequests};
use zkprotocol::content::{
  GetArchiveZkLinks, GetArchiveZkNote, GetZkLinksSince, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNoteAndLinks, ZkLinks,
  ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives, ZkNoteId,
};
use zkprotocol::messages::PublicReplyMessage;
use zkprotocol::messages::{
  PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage, PublicMessage,
};
use zkprotocol::search::{TagSearch, ZkListNoteSearchResult, ZkNoteSearch};

pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<Option<orgauth::data::LoginData>, Box<dyn Error>> {
  let mut conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
  conn.busy_timeout(Duration::from_millis(500))?;

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
        Err(e) => match e {
          orgauth::error::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows) => None,
          _ => return Err(e.into()), // Err(e.into()),
        },
      }
    }
  };
  Ok(ldopt)
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
  msg: orgauth::data::UserRequestMessage,
) -> Result<orgauth::data::UserResponseMessage, Box<dyn Error>> {
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

// // TODO: have the socket close on timeout as in example?
// pub struct StreamingWebSocket {
//   pub config: Config,
//   pub uid: i64,
// }

// impl actix::Actor for StreamingWebSocket {
//   type Context = ws::WebsocketContext<Self>;

//   // /// Method is called on actor start. We start the heartbeat process here.
//   // fn started(&mut self, ctx: &mut Self::Context) {
//   //   self.hb(ctx);
//   // }
// }

// impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for StreamingWebSocket {
//   fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
//     // process websocket messages
//     println!("WS: {msg:?}");
//     match msg {
//       Ok(ws::Message::Ping(msg)) => {
//         // self.hb = Instant::now();
//         ctx.pong(&msg);
//       }
//       Ok(ws::Message::Pong(_)) => {
//         // self.hb = Instant::now();
//       }
//       Ok(ws::Message::Text(text)) => match std::str::from_utf8(text.as_bytes()) {
//         Ok(utf8) => match serde_json::from_str(utf8) {
//           Ok(psm) => match zk_interface_loggedin_wsstreaming(&self.config, self.uid, &psm, ctx) {
//             Ok(_) => {}
//             Err(e) => {}
//           },
//           Err(_) => {}
//         },
//         Err(_) => {}
//       },
//       Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
//       Ok(ws::Message::Close(reason)) => {
//         ctx.close(reason);
//         ctx.stop();
//       }
//       _ => ctx.stop(),
//     }
//   }
// }

// pub fn zk_interface_loggedin_wsstreaming(
//   config: &Config,
//   uid: i64,
//   msg: &PrivateStreamingMessage,
//   ctx: &mut ws::WebsocketContext<StreamingWebSocket>,
// ) -> Result<(), zkerr::Error> {
//   // Ok(HttpResponse::Ok().into())
//   match msg.what {
//     PrivateStreamingRequests::SearchZkNotes => {
//       let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
//       let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
//       let conn = Arc::new(sqldata::connection_open(
//         config.orgauth_config.db.as_path(),
//       )?);
//       let znsstream = search::search_zknotes_wsstream(conn, uid, search, ctx);
//       Ok(())
//     }
//     _ => Ok(()),
//   }
//   //   PrivateStreamingRequests::GetArchiveZkLinks => {
//   //     let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
//   //     let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
//   //     let conn = Arc::new(sqldata::connection_open(
//   //       config.orgauth_config.db.as_path(),
//   //     )?);
//   //     let bstream = sqldata::read_archivezklinks_stream(conn, uid, rq.createddate_after);
//   //     Ok(HttpResponse::Ok().streaming(bstream))
//   //   }
//   //   PrivateStreamingRequests::GetZkLinksSince => {
//   //     let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
//   //     let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
//   //     let conn = Arc::new(sqldata::connection_open(
//   //       config.orgauth_config.db.as_path(),
//   //     )?);
//   //     let bstream = sqldata::read_zklinks_since_stream(conn, uid, rq.createddate_after);
//   //     Ok(HttpResponse::Ok().streaming(bstream))
//   //   } // wat => Err(format!("invalid 'what' code:'{}'", wat).into()),
//   // }
// }

pub async fn zk_interface_loggedin_streaming(
  config: &Config,
  uid: i64,
  msg: &PrivateStreamingMessage,
) -> Result<HttpResponse, Box<dyn Error>> {
  match msg.what {
    PrivateStreamingRequests::SearchZkNotes => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let znsstream = search::search_zknotes_stream(conn, uid, search);
      Ok(HttpResponse::Ok().streaming(znsstream))
    }
    PrivateStreamingRequests::GetArchiveZkLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let bstream = sqldata::read_archivezklinks_stream(conn, uid, rq.createddate_after);
      Ok(HttpResponse::Ok().streaming(bstream))
    }
    PrivateStreamingRequests::GetZkLinksSince => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let bstream = sqldata::read_zklinks_since_stream(conn, uid, rq.createddate_after);
      Ok(HttpResponse::Ok().streaming(bstream))
    } // wat => Err(format!("invalid 'what' code:'{}'", wat).into()),
  }
}

pub async fn zk_interface_loggedin(
  appdata: &AppData,
  uid: i64,
  msg: &PrivateMessage,
) -> Result<PrivateReplyMessage, Box<dyn Error>> {
  {
    let tokes = appdata.wstokens.lock().unwrap();
    println!("zk_interface_loggedin tokes {:?}", tokes);
  }
  // match PrivateRequests::fromstr(msg.what.as_str()) {
  match msg.what {
    PrivateRequests::GetZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let (_nid, note) = sqldata::read_zknote(&conn, Some(uid), &id)?;
      info!("user#getzknote: {:?} - {}", id, note.title);
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNote,
        content: serde_json::to_value(note)?,
      })
    }
    PrivateRequests::GetZkNoteAndLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknoteandlinks(&conn, Some(uid), &gzne.zknote)?;
      info!(
        "user#getzknoteedit: {:?} - {}",
        gzne.zknote, note.zknote.title
      );

      let znew = ZkNoteAndLinksWhat {
        what: gzne.what,
        znl: note,
      };

      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNoteAndLinksWhat,
        content: serde_json::to_value(znew)?,
      })
    }
    PrivateRequests::GetZnlIfChanged => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzic: GetZnlIfChanged = serde_json::from_value(msgdata.clone())?;
      info!(
        "user#getzneifchanged: {:?} - {}",
        gzic.zknote, gzic.changeddate
      );
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, Some(uid), &gzic)?;

      match ozkne {
        Some(zkne) => Ok(PrivateReplyMessage {
          what: PrivateReplies::ZkNoteAndLinksWhat,
          content: serde_json::to_value(ZkNoteAndLinksWhat {
            what: gzic.what,
            znl: zkne,
          })?,
        }),
        None => Ok(PrivateReplyMessage {
          what: PrivateReplies::Noop,
          content: serde_json::Value::Null,
        }),
      }
    }
    PrivateRequests::GetZkNoteComments => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteComments = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotecomments(&conn, uid, &gzne)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNoteComments,
        content: serde_json::to_value(notes)?,
      })
    }
    PrivateRequests::GetZkNoteArchives => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteArchives = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotearchives(&conn, uid, &gzne)?;
      let zlnsr = ZkListNoteSearchResult {
        notes,
        offset: gzne.offset,
        what: "archives".to_string(),
      };
      // let (id, uuid) = sqldata::id_uuid_for_zknoteid(&conn, &gzne.zknote)?;
      let zka = ZkNoteArchives {
        zknote: gzne.zknote,
        results: zlnsr,
      };
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNoteArchives,
        content: serde_json::to_value(zka)?,
      })
    }
    PrivateRequests::GetArchiveZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let (_nid, note) = sqldata::read_archivezknote(&conn, uid, &rq)?;
      info!("user#getarchivezknote: {} - {}", note.id, note.title);
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNote,
        content: serde_json::to_value(note)?,
      })
    }
    PrivateRequests::GetArchiveZklinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let links = sqldata::read_archivezklinks(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ArchiveZkLinks,
        content: serde_json::to_value(links)?,
      })
    }
    PrivateRequests::GetZkLinksSince => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let links = sqldata::read_zklinks_since(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkLinks,
        content: serde_json::to_value(links)?,
      })
    }
    PrivateRequests::SearchZkNotes => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      // let res = search::search_zknotes_simple(&conn, uid, &search)?;
      let res = search::search_zknotes(&conn, uid, &search)?;
      match res {
        search::SearchResult::SrId(res) => Ok(PrivateReplyMessage {
          what: PrivateReplies::ZkNoteIdSearchResult,
          content: serde_json::to_value(res)?,
        }),
        search::SearchResult::SrListNote(res) => Ok(PrivateReplyMessage {
          what: PrivateReplies::ZkListNoteSearchResult,
          content: serde_json::to_value(res)?,
        }),
        search::SearchResult::SrNote(res) => Ok(PrivateReplyMessage {
          what: PrivateReplies::ZkNoteSearchResult,
          content: serde_json::to_value(res)?,
        }),
        search::SearchResult::SrNoteAndLink(res) => Ok(PrivateReplyMessage {
          what: PrivateReplies::ZkNoteAndLinksSearchResult,
          content: serde_json::to_value(res)?,
        }),
      }
    }
    PrivateRequests::PowerDelete => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: TagSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let res =
        search::power_delete_zknotes(&conn, appdata.config.file_path.clone(), uid, &search)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::PowerDeleteComplete,
        content: serde_json::to_value(res)?,
      })
    }
    PrivateRequests::DeleteZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      sqldata::delete_zknote(&conn, appdata.config.file_path.clone(), uid, &id)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::DeletedZkNote,
        content: serde_json::to_value(id)?,
      })
    }
    PrivateRequests::SaveZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sbe: SaveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let s = sqldata::save_zknote(&conn, uid, &sbe)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedZkNote,
        content: serde_json::to_value(s)?,
      })
    }
    PrivateRequests::SaveZkLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let msg: ZkLinks = serde_json::from_value(msgdata.clone())?;
      let s = sqldata::save_zklinks(&appdata.config.orgauth_config.db.as_path(), uid, msg.links)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedZkLinks,
        content: serde_json::to_value(s)?,
      })
    }
    PrivateRequests::SaveZkNoteAndLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sznpl: SaveZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let (_, szkn) = sqldata::save_zknote(&conn, uid, &sznpl.note)?;
      let _s = sqldata::save_savezklinks(&conn, uid, szkn.id, sznpl.links)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedZkNoteAndLinks,
        content: serde_json::to_value(szkn)?,
      })
    }
    PrivateRequests::SaveImportZkNotes => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzl: Vec<ImportZkNote> = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      sqldata::save_importzknotes(&conn, uid, gzl)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedImportZkNotes,
        content: serde_json::to_value(())?,
      })
    }
    PrivateRequests::SetHomeNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let hn: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      // let mut user = sqldata::read_user_by_id(&conn, uid)?;
      // user.homenoteid = Some(hn);
      sqldata::set_homenote(&conn, uid, hn)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::HomeNoteSet,
        content: serde_json::to_value(hn)?,
      })
    }
    PrivateRequests::SyncRemote => {
      let conn = sqldata::connection_open(appdata.config.orgauth_config.db.as_path())?;
      let user = orgauth::dbfun::read_user_by_id(&conn, uid)?; // TODO pass this in from calling ftn?

      // sync::sync(&conn, &user, &mut zknotes_callbacks()).await
      sync::websocket_sync_from(&conn, &user, &mut zknotes_callbacks())
        .await
        .map_err(|e| e.into())
    }
    PrivateRequests::GetWsToken => {
      println!("PrivateRequests::GetWsToken");
      let uuid = Uuid::new_v4();
      let now = orgauth::util::now()?;

      {
        let mut wst = appdata.wstokens.lock().unwrap();
        wst.insert(
          uuid,
          TokenInfo {
            create_time: now,
            uid,
          },
        );
        println!("wst: {:?}", wst);
      }

      Ok(PrivateReplyMessage {
        what: PrivateReplies::WsToken,
        content: serde_json::to_value(uuid)?,
      })
    }
  }
}

// public json msgs don't require login.
pub fn public_interface(
  config: &Config,
  msg: PublicMessage,
  ipaddr: Option<&str>,
) -> Result<PublicReplyMessage, Box<dyn Error>> {
  match msg.what {
    PublicRequests::GetZkNoteAndLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      // let note = sqldata::read_zknote(&conn, None, id)?;
      let (_, note) = sqldata::read_zknote(&conn, None, &gzne.zknote)?;
      info!(
        "public#getzknote: {:?} - {} - {:?}",
        gzne.zknote, note.title, ipaddr
      );
      Ok(PublicReplyMessage {
        what: PublicReplies::ZkNoteAndLinks,
        content: serde_json::to_value(ZkNoteAndLinksWhat {
          what: gzne.what,
          znl: ZkNoteAndLinks {
            links: sqldata::read_public_zklinks(&conn, &note.id)?,
            zknote: note,
          },
        })?,
      })
    }
    PublicRequests::GetZnlIfChanged => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzic: GetZnlIfChanged = serde_json::from_value(msgdata.clone())?;
      info!(
        "user#getzneifchanged: {:?} - {}",
        gzic.zknote, gzic.changeddate
      );
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, None, &gzic)?;

      match ozkne {
        Some(zkne) => Ok(PublicReplyMessage {
          what: PublicReplies::ZkNoteAndLinks,
          content: serde_json::to_value(ZkNoteAndLinksWhat {
            what: gzic.what,
            znl: zkne,
          })?,
        }),
        None => Ok(PublicReplyMessage {
          what: PublicReplies::Noop,
          content: serde_json::Value::Null,
        }),
      }
    }
    PublicRequests::GetZkNotePubId => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let pubid: String = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknotepubid(&conn, None, pubid.as_str())?;
      info!(
        "public#getzknotepubid: {} - {} - {:?}",
        pubid, note.title, ipaddr,
      );
      Ok(PublicReplyMessage {
        what: PublicReplies::ZkNoteAndLinks,
        content: serde_json::to_value(ZkNoteAndLinksWhat {
          what: "".to_string(),
          znl: ZkNoteAndLinks {
            links: sqldata::read_public_zklinks(&conn, &note.id)?,
            zknote: note,
          },
        })?,
      })
    }
  }
}
