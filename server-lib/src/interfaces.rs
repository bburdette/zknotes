use crate::config::Config;
use crate::jobs::GirlbossMonitor;
use crate::jobs::JobId;
use crate::jobs::JobMonitor;
use crate::jobs::LogMonitor;
use crate::search;
use crate::sqldata;
use crate::sqldata::zknotes_callbacks;
use crate::state::new_jobid;
use crate::state::State;
use crate::sync;
use actix_session::Session;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use log::info;
use orgauth;
use orgauth::endpoints::Tokener;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;
use zkprotocol::constants::PrivateReplies;
use zkprotocol::constants::PublicReplies;
use zkprotocol::constants::{PrivateRequests, PrivateStreamingRequests, PublicRequests};
use zkprotocol::content::JobState;
use zkprotocol::content::JobStatus;
use zkprotocol::content::{
  GetArchiveZkLinks, GetArchiveZkNote, GetZkLinksSince, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNoteAndLinks, SyncSince,
  ZkLinks, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives, ZkNoteId,
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

  let mut cb = sqldata::zknotes_callbacks();
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
      &mut sqldata::zknotes_callbacks(),
      Some("user".to_string()),
      msg,
    )
    .await?,
  )
}

pub async fn zk_interface_loggedin_streaming(
  config: &Config,
  uid: i64,
  msg: &PrivateStreamingMessage,
) -> Result<HttpResponse, Box<dyn Error>> {
  let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
  match msg.what {
    PrivateStreamingRequests::SearchZkNotes => {
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let znsstream =
        search::search_zknotes_stream(conn, config.file_path.clone(), uid, search, None)
          .map(sync::bytesify);
      Ok(HttpResponse::Ok().streaming(znsstream))
    }
    PrivateStreamingRequests::GetArchiveZkLinks => {
      let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let bstream = sqldata::read_archivezklinks_stream(conn, uid, rq.createddate_after, None)
        .map(sync::bytesify);
      Ok(HttpResponse::Ok().streaming(bstream))
    }
    PrivateStreamingRequests::GetZkLinksSince => {
      let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let bstream = sqldata::read_zklinks_since_stream(conn, uid, rq.createddate_after, None)
        .map(sync::bytesify);
      Ok(HttpResponse::Ok().streaming(bstream))
    }
    PrivateStreamingRequests::Sync => {
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);
      let rq: SyncSince = serde_json::from_value(msgdata.clone())?;
      let jl = LogMonitor {};
      let ss = sync::sync_stream(
        conn,
        PathBuf::from(&config.file_path),
        uid,
        None,
        None,
        None,
        rq.after,
        &mut zknotes_callbacks(),
        &jl,
      );
      Ok(HttpResponse::Ok().streaming(ss))
    }
  }
}

pub async fn zk_interface_loggedin(
  state: &State,
  uid: i64,
  msg: &PrivateMessage,
) -> Result<PrivateReplyMessage, Box<dyn Error>> {
  info!("zk_interface_loggedin msg: {:?}", msg);
  match msg.what {
    PrivateRequests::GetZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let (_nid, note) = sqldata::read_zknote(&conn, &state.config.file_path, Some(uid), &id)?;
      info!("user#getzknote: {:?} - {}", id, note.title);
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNote,
        content: serde_json::to_value(note)?,
      })
    }
    PrivateRequests::GetZkNoteAndLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let note =
        sqldata::read_zknoteandlinks(&conn, &state.config.file_path, Some(uid), &gzne.zknote)?;
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
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, &state.config.file_path, Some(uid), &gzic)?;

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
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotecomments(&conn, &state.config.file_path, uid, &gzne)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNoteComments,
        content: serde_json::to_value(notes)?,
      })
    }
    PrivateRequests::GetZkNoteArchives => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let gzne: GetZkNoteArchives = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let notes = sqldata::read_zknotearchives(&conn, &state.config.file_path, uid, &gzne)?;
      let zlnsr = ZkListNoteSearchResult {
        notes,
        offset: gzne.offset,
        what: "archives".to_string(),
      };
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
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let (_nid, note) = sqldata::read_archivezknote(&conn, &state.config.file_path, uid, &rq)?;
      info!("user#getarchivezknote: {} - {}", note.id, note.title);
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkNote,
        content: serde_json::to_value(note)?,
      })
    }
    PrivateRequests::GetArchiveZklinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let links = sqldata::read_archivezklinks(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ArchiveZkLinks,
        content: serde_json::to_value(links)?,
      })
    }
    PrivateRequests::GetZkLinksSince => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let links = sqldata::read_zklinks_since(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::ZkLinks,
        content: serde_json::to_value(links)?,
      })
    }
    PrivateRequests::SearchZkNotes => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let res = search::search_zknotes(&conn, &state.config.file_path, uid, &search)?;
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
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let res = search::power_delete_zknotes(&conn, state.config.file_path.clone(), uid, &search)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::PowerDeleteComplete,
        content: serde_json::to_value(res)?,
      })
    }
    PrivateRequests::DeleteZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let id: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      sqldata::delete_zknote(&conn, state.config.file_path.clone(), uid, &id)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::DeletedZkNote,
        content: serde_json::to_value(id)?,
      })
    }
    PrivateRequests::SaveZkNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sbe: SaveZkNote = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let s = sqldata::save_zknote(&conn, uid, &sbe)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedZkNote,
        content: serde_json::to_value(s)?,
      })
    }
    PrivateRequests::SaveZkLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let msg: ZkLinks = serde_json::from_value(msgdata.clone())?;
      let s = sqldata::save_zklinks(&state.config.orgauth_config.db.as_path(), uid, msg.links)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedZkLinks,
        content: serde_json::to_value(s)?,
      })
    }
    PrivateRequests::SaveZkNoteAndLinks => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let sznpl: SaveZkNoteAndLinks = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
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
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      sqldata::save_importzknotes(&conn, uid, gzl)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::SavedImportZkNotes,
        content: serde_json::to_value(())?,
      })
    }
    PrivateRequests::SetHomeNote => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let hn: ZkNoteId = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      sqldata::set_homenote(&conn, uid, hn)?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::HomeNoteSet,
        content: serde_json::to_value(hn)?,
      })
    }
    PrivateRequests::SyncRemote => {
      info!("PrivateRequests::SyncRemote");
      let dbpath: PathBuf = state.config.orgauth_config.db.to_path_buf();
      let file_path: PathBuf = state.config.file_path.to_path_buf();
      let uid: i64 = uid;

      let jid = new_jobid(state, uid);
      info!("SyncRemote jobid: {:?}", jid);

      let job = state
        .girlboss
        .start(jid, move |mon| async move {
          let gbm = GirlbossMonitor { monitor: mon };
          // spawn thread, local runtime.  success.
          std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            let local = LocalSet::new();
            let mut callbacks = &mut zknotes_callbacks();
            write!(gbm, "starting sync");
            let r = local.block_on(
              &rt,
              sync::sync(&dbpath, &file_path, uid, &mut callbacks, &gbm),
            );
            match r {
              Ok(_) => write!(gbm, "sync completed"),
              Err(e) => write!(gbm, "sync err: {:?}", e),
            }
          });
        })
        .await?;

      // tokio::time::sleep(Duration::from_millis(100)).await;

      Ok(PrivateReplyMessage {
        what: PrivateReplies::JobStatus,
        content: serde_json::to_value(JobStatus {
          jobno: jid.jobno,
          state: JobState::Started,
          message: "".to_string(),
        })?,
      })
    }
    PrivateRequests::SyncFiles => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let zns: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      let dv = sync::sync_files_down(
        &conn,
        &state.config.file_tmp_path.as_path(),
        &state.config.file_path.as_path(),
        uid,
        &zns,
      )
      .await?;
      let uv = sync::sync_files_up(&conn, &state.config.file_path.as_path(), uid, &zns).await?;
      Ok(PrivateReplyMessage {
        what: PrivateReplies::FileSyncComplete,
        content: serde_json::to_value((dv, uv))?,
      })
    }
    PrivateRequests::GetJobStatus => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let jobno: i64 = serde_json::from_value(msgdata.clone())?;

      let jid = JobId { uid, jobno };

      match state.girlboss.get(&jid).await {
        Some(job) => {
          let state = if job.is_finished() {
            if job.succeeded() {
              JobState::Completed
            } else {
              JobState::Failed
            }
          } else {
            JobState::Running
          };

          let js = JobStatus {
            jobno,
            state,
            message: job.status().message().to_string(),
          };
          info!("job status: {:?}", js);

          Ok(PrivateReplyMessage {
            what: PrivateReplies::JobStatus,
            content: serde_json::to_value(js)?,
          })
        }
        None => {
          let js = jobno;
          Ok(PrivateReplyMessage {
            what: PrivateReplies::JobNotFound,
            content: serde_json::to_value(js)?,
          })
        }
      }
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
      let (_, note) = sqldata::read_zknote(&conn, &config.file_path, None, &gzne.zknote)?;
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
      let ozkne = sqldata::read_zneifchanged(&conn, &config.file_path, None, &gzic)?;

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
      let note = sqldata::read_zknotepubid(&conn, &config.file_path, None, pubid.as_str())?;
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
