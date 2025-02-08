use crate::config::Config;
use crate::error as zkerr;
use crate::jobs::GirlbossMonitor;
use crate::jobs::JobId;
use crate::jobs::JobMonitor;
use crate::jobs::LogMonitor;
use crate::jobs::ReportFileMonitor;
use crate::search;
use crate::sqldata;
use crate::sqldata::make_file_entry;
use crate::sqldata::make_file_note;
use crate::sqldata::set_zknote_file;
use crate::sqldata::zknotes_callbacks;
use crate::state::new_jobid;
use crate::state::State;
use crate::sync;
use actix_session::Session;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use log::{error, info};
use orgauth;
use orgauth::data::UserId;
use orgauth::endpoints::Tokener;
use rusqlite::Connection;
use std::env;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::constants::PrivateStreamingRequests;
use zkprotocol::content::JobState;
use zkprotocol::content::JobStatus;
use zkprotocol::content::{
  GetArchiveZkLinks, GetZkLinksSince, SyncSince, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives,
};
use zkprotocol::messages::PrivateStreamingMessage;
use zkprotocol::private::PrivateReply;
use zkprotocol::private::PrivateRequest;
use zkprotocol::public::{PublicReply, PublicRequest};
use zkprotocol::search::{ZkListNoteSearchResult, ZkNoteSearch};
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
          _ => return Err(e.into()),
        },
      }
    }
  };
  Ok(ldopt)
}

// Just like orgauth::endpoints::user_interface, except adds in extra user data.
pub async fn user_interface(
  conn: &Connection,
  tokener: &mut dyn Tokener,
  config: &Config,
  msg: orgauth::data::UserRequest,
) -> Result<orgauth::data::UserResponse, zkerr::Error> {
  Ok(
    orgauth::endpoints::user_interface(
      conn,
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
  uid: UserId,
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
  conn: &Connection,
  uid: UserId,
  msg: &PrivateRequest,
) -> Result<PrivateReply, zkerr::Error> {
  info!("zk_interface_loggedin msg: {:?}", msg);
  match msg {
    PrivateRequest::PvqGetZkNote(id) => {
      let (_nid, note) = sqldata::read_zknote(&conn, &state.config.file_path, Some(uid), &id)?;
      info!("user#getzknote: {:?} - {}", id, note.title);
      Ok(PrivateReply::PvyZkNote(note))
    }
    PrivateRequest::PvqGetZkNoteAndLinks(gzne) => {
      let note =
        sqldata::read_zknoteandlinks(&conn, &state.config.file_path, Some(uid), &gzne.zknote)?;
      info!(
        "user#getzknoteedit: {:?} - {}",
        gzne.zknote, note.zknote.title
      );

      let znew = ZkNoteAndLinksWhat {
        what: gzne.what.clone(),
        znl: note,
      };

      Ok(PrivateReply::PvyZkNoteAndLinksWhat(znew))
    }
    PrivateRequest::PvqGetZnlIfChanged(gzic) => {
      info!(
        "user#getzneifchanged: {:?} - {}",
        gzic.zknote, gzic.changeddate
      );
      let ozkne = sqldata::read_zneifchanged(&conn, &state.config.file_path, Some(uid), &gzic)?;

      match ozkne {
        Some(zkne) => Ok(PrivateReply::PvyZkNoteAndLinksWhat(ZkNoteAndLinksWhat {
          what: gzic.what.clone(),
          znl: zkne,
        })),
        None => Ok(PrivateReply::PvyNoop),
      }
    }
    PrivateRequest::PvqGetZkNoteComments(gzne) => {
      let notes = sqldata::read_zknotecomments(&conn, &state.config.file_path, uid, &gzne)?;
      Ok(PrivateReply::PvyZkNoteComments(notes))
    }
    PrivateRequest::PvqGetZkNoteArchives(gzne) => {
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
      Ok(PrivateReply::PvyZkNoteArchives(zka))
    }
    PrivateRequest::PvqGetArchiveZkNote(rq) => {
      let (_nid, note) = sqldata::read_archivezknote(&conn, &state.config.file_path, uid, &rq)?;
      info!("user#getarchivezknote: {} - {}", note.id, note.title);
      Ok(PrivateReply::PvyZkNote(note))
    }
    PrivateRequest::PvqGetArchiveZklinks(rq) => {
      let links = sqldata::read_archivezklinks(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReply::PvyArchiveZkLinks(links))
    }
    PrivateRequest::PvqGetZkLinksSince(rq) => {
      let links = sqldata::read_zklinks_since(&conn, uid, rq.createddate_after)?;
      Ok(PrivateReply::PvyZkLinks(links))
    }
    PrivateRequest::PvqSearchZkNotes(search) => {
      let res = search::search_zknotes(&conn, &state.config.file_path, uid, &search)?;
      match res {
        search::SearchResult::SrId(res) => Ok(PrivateReply::PvyZkNoteIdSearchResult(res)),
        search::SearchResult::SrListNote(res) => Ok(PrivateReply::PvyZkListNoteSearchResult(res)),
        search::SearchResult::SrNote(res) => Ok(PrivateReply::PvyZkNoteSearchResult(res)),
        search::SearchResult::SrNoteAndLink(res) => {
          Ok(PrivateReply::PvyZkNoteAndLinksSearchResult(res))
        }
      }
    }
    PrivateRequest::PvqPowerDelete(search) => {
      let res = search::power_delete_zknotes(&conn, state.config.file_path.clone(), uid, &search)?;
      Ok(PrivateReply::PvyPowerDeleteComplete(res))
    }
    PrivateRequest::PvqDeleteZkNote(id) => {
      sqldata::delete_zknote(&conn, state.config.file_path.clone(), uid, &id)?;
      Ok(PrivateReply::PvyDeletedZkNote(id.clone()))
    }
    PrivateRequest::PvqSaveZkNote(sbe) => {
      let (_id, s) = sqldata::save_zknote(&conn, uid, &sbe)?;
      Ok(PrivateReply::PvySavedZkNote(s))
    }
    PrivateRequest::PvqSaveZkLinks(msg) => {
      let _ = sqldata::save_zklinks(&state.config.orgauth_config.db.as_path(), uid, &msg.links)?;
      Ok(PrivateReply::PvySavedZkLinks)
    }
    PrivateRequest::PvqSaveZkNoteAndLinks(sznpl) => {
      let (_, szkn) = sqldata::save_zknote(&conn, uid, &sznpl.note)?;
      let _s = sqldata::save_savezklinks(&conn, uid, szkn.id, &sznpl.links)?;
      Ok(PrivateReply::PvySavedZkNoteAndLinks(szkn))
    }
    PrivateRequest::PvqSaveImportZkNotes(gzl) => {
      sqldata::save_importzknotes(&conn, uid, gzl)?;
      Ok(PrivateReply::PvySavedImportZkNotes)
    }
    PrivateRequest::PvqSetHomeNote(hn) => {
      sqldata::set_homenote(&conn, uid, hn)?;
      Ok(PrivateReply::PvyHomeNoteSet(hn.clone()))
    }
    PrivateRequest::PvqSyncRemote => {
      let dbpath: PathBuf = state.config.orgauth_config.db.to_path_buf();
      let file_path: PathBuf = state.config.file_path.to_path_buf();
      let uid: UserId = uid;
      let jid = new_jobid(state, uid);
      let lgb = state.girlboss.clone();

      std::thread::spawn(move || {
        let rt = actix_rt::System::new();

        async fn startit(
          lgb: Arc<std::sync::RwLock<girlboss::Girlboss<JobId, girlboss::Monitor>>>,
          dbpath: PathBuf,
          file_path: PathBuf,
          uid: UserId,
          jid: JobId,
        ) -> () {
          lgb
            .write()
            .map_err(|e| {
              info!("rwlock error: {}", e);
              e
            })
            .unwrap()
            .start(jid, move |mon| async move {
              // job log file.
              let mut lfn = env::temp_dir();
              let uuidname = Uuid::new_v4().to_string();
              lfn.push(uuidname.clone());
              let jlf = match File::create_new(lfn.clone()) {
                Ok(f) => {
                  info!("sync log file: {:?}", lfn);
                  f
                }
                Err(e) => {
                  error!("err creating sync log file: {:?}", e);
                  actix_rt::System::current().stop();
                  return;
                }
              };
              let gbm = ReportFileMonitor {
                monitor: mon,
                outf: Mutex::new(jlf),
              };
              let mut callbacks = &mut zknotes_callbacks();
              write!(gbm, "starting sync");

              let r: Result<(), Box<dyn std::error::Error>> = async {
                let c = sqldata::connection_open(dbpath.as_path())?;
                let conn = Arc::new(c);
                let snid = sync::sync(&conn, &file_path, uid, &mut callbacks, &gbm).await?;
                write!(gbm, "sync completed");
                let fid = make_file_entry(
                  &*conn,
                  file_path.as_path(),
                  uid,
                  &uuidname,
                  lfn.as_path(),
                  false,
                )?;
                // PrivateReply::PvySyncComplete

                set_zknote_file(&*conn, snid, fid)?;

                // conn.commit()?;
                Ok(())
              }
              .await;

              match r {
                Ok(()) => (),
                Err(e) => write!(gbm, "sync err: {:?}", e),
              }
              // add the log to the sync note.

              // match sqldata::connection_open(dbpath.as_path()) {
              //   Err(e) => write!(gbm, "sync err: {:?}", e),
              //   Ok(c) => {
              //     let conn = Arc::new(c);
              //     match sync::sync(&conn, &file_path, uid, &mut callbacks, &gbm).await {
              //       Err(e) => write!(gbm, "sync err: {:?}", e),
              //       Ok(snid) =>
              //       {
              //         write!(gbm, "sync completed");
              //         let fid = make_file_entry(&*conn, file_path.as_path(), uid, nam, lfn.as_path(), false)?;
              //         // PrivateReply::PvySyncComplete

              //         set_zknote_file(&*conn, snid, fid)
              //         // let syncnoteid = save_sync(&conn, user.id, unote, CompletedSync { after, now }).await?;

              //         conn.commit()?;
              //       }
              //     };

              //     // add the log to the sync note.

              actix_rt::System::current().stop();
            })
            .map_err(|e| {
              info!("girlboss start error: {}", e);
              e
            })
            .unwrap();
          ()
        }

        rt.block_on(startit(lgb, dbpath, file_path, uid, jid));
        rt.run()
          .map_err(|e| {
            info!("rt.run error: {}", e);
            e
          })
          .unwrap()
      });

      Ok(PrivateReply::PvyJobStatus(JobStatus {
        jobno: jid.jobno,
        state: JobState::Started,
        message: "".to_string(),
      }))
    }
    PrivateRequest::PvqSyncFiles(znsrq) => {
      let dbpath: PathBuf = state.config.orgauth_config.db.to_path_buf();
      let file_path: PathBuf = state.config.file_path.to_path_buf();
      let file_tmp_path: PathBuf = state.config.file_tmp_path.to_path_buf();
      let uid: UserId = uid;
      let jid = new_jobid(state, uid);
      let lgb = state.girlboss.clone();
      let zns = znsrq.clone();

      std::thread::spawn(move || {
        let rt = actix_rt::System::new();

        async fn startit(
          lgb: Arc<std::sync::RwLock<girlboss::Girlboss<JobId, girlboss::Monitor>>>,
          dbpath: PathBuf,
          file_path: PathBuf,
          file_tmp_path: PathBuf,
          uid: UserId,
          zns: ZkNoteSearch,
          jid: JobId,
        ) -> () {
          lgb
            .write()
            .map_err(|e| {
              info!("rwlock error: {}", e);
              e
            })
            .unwrap()
            .start(jid, move |mon| async move {
              let gbm = GirlbossMonitor { monitor: mon };
              write!(gbm, "starting file sync");

              let r = async {
                let conn = sqldata::connection_open(&dbpath.as_path())?;
                let _dv = sync::sync_files_down(
                  &conn,
                  &file_tmp_path.as_path(),
                  &file_path.as_path(),
                  uid,
                  &zns,
                )
                .await?;
                let _uv = sync::sync_files_up(&conn, &file_path.as_path(), uid, &zns).await?;

                // TODO: send a result with a list of synced files.
                Ok::<(), zkerr::Error>(())
              };
              match r.await {
                Ok(_) => write!(gbm, "file sync completed"),
                Err(e) => write!(gbm, "file sync err: {:?}", e),
              };
              actix_rt::System::current().stop();
            })
            .map_err(|e| {
              info!("girlboss start error: {}", e);
              e
            })
            .unwrap();
          ()
        }

        let znsclone = zns.clone();

        rt.block_on(startit(
          lgb,
          dbpath,
          file_path,
          file_tmp_path,
          uid,
          znsclone,
          jid,
        ));
        rt.run()
          .map_err(|e| {
            info!("rt.run error: {}", e);
            e
          })
          .unwrap()
      });

      Ok(PrivateReply::PvyJobStatus(JobStatus {
        jobno: jid.jobno,
        state: JobState::Started,
        message: "".to_string(),
      }))
    }

    PrivateRequest::PvqGetJobStatus(jobno) => {
      let jid = JobId {
        uid: *uid.to_i64(),
        jobno: jobno.clone(),
      };

      match state.girlboss.read().unwrap().get(&jid) {
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
            jobno: jobno.clone(),
            state,
            message: job.status().message().to_string(),
          };
          info!("job status: {:?}", js);

          Ok(PrivateReply::PvyJobStatus(js))
        }
        None => Ok(PrivateReply::PvyJobNotFound(jobno.clone())),
      }
    }
  }
}

// public json msgs don't require login.
pub fn public_interface(
  config: &Config,
  msg: &PublicRequest,
  ipaddr: Option<&str>,
) -> Result<PublicReply, zkerr::Error> {
  match msg {
    PublicRequest::PbrGetZkNoteAndLinks(gzne) => {
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let (_, note) = sqldata::read_zknote(&conn, &config.file_path, None, &gzne.zknote)?;
      info!(
        "public#getzknote: {:?} - {} - {:?}",
        gzne.zknote, note.title, ipaddr
      );
      Ok(PublicReply::PbyZkNoteAndLinksWhat(ZkNoteAndLinksWhat {
        what: gzne.what.clone(),
        znl: ZkNoteAndLinks {
          links: sqldata::read_public_zklinks(&conn, &note.id)?,
          zknote: note,
        },
      }))
    }
    PublicRequest::PbrGetZnlIfChanged(gzic) => {
      info!(
        "user#getzneifchanged: {:?} - {}",
        gzic.zknote, gzic.changeddate
      );
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let ozkne = sqldata::read_zneifchanged(&conn, &config.file_path, None, &gzic)?;

      match ozkne {
        Some(zkne) => Ok(PublicReply::PbyZkNoteAndLinksWhat(ZkNoteAndLinksWhat {
          what: gzic.what.clone(),
          znl: zkne,
        })),
        None => Ok(PublicReply::PbyNoop),
      }
    }
    PublicRequest::PbrGetZkNotePubId(pubid) => {
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      let note = sqldata::read_zknotepubid(&conn, &config.file_path, None, pubid.as_str())?;
      info!(
        "public#getzknotepubid: {} - {} - {:?}",
        pubid, note.title, ipaddr,
      );
      Ok(PublicReply::PbyZkNoteAndLinks(ZkNoteAndLinks {
        links: sqldata::read_public_zklinks(&conn, &note.id)?,
        zknote: note,
      }))
    }
  }
}
