use crate::config::Config;
use crate::error as zkerr;
use crate::jobs::GirlbossMonitor;
use crate::jobs::JobId;
use crate::jobs::JobMonitor;
use crate::jobs::LogMonitor;
use crate::search;
use crate::sqldata;
use crate::sqldata::local_server_id;
use crate::sqldata::zknotes_callbacks;
use crate::sqldata::LapinInfo;
use crate::state::new_jobid;
use crate::state::State;
use crate::sync;
use actix_session::Session;
use actix_web::HttpResponse;
use futures_util::StreamExt;
use lapin::ConnectionState;
use log::{error, info};
use orgauth;
use orgauth::data::UserId;
use orgauth::endpoints::Tokener;
use orgauth::util::now;
use rusqlite::Connection;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::constants::PrivateStreamingRequests;
use zkprotocol::content::JobState;
use zkprotocol::content::JobStatus;
use zkprotocol::content::Server;
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

pub async fn connect_and_make_lapin_info(
  state: &State,
  token: Option<String>,
) -> Option<LapinInfo> {
  // TODO: maybe attempt reconnect only every X seconds, so as not to
  // slow processing during rmq outage.

  // if no uri configured, will never be a connection.
  let uri = match &state.config.aqmp_uri {
    Some(uri) => uri,
    None => {
      return None;
    }
  };

  // use existing connection if there is one.
  let reconnect = match &state.lapin_conn.read() {
    Ok(wut) => match &**wut {
      Some(lc) => {
        let reconnect = match lc.status().state() {
          ConnectionState::Initial => true,
          ConnectionState::Connecting => false,
          ConnectionState::Connected => false,
          ConnectionState::Closing => true,
          ConnectionState::Closed => true,
          // ConnectionState::Reconnecting => false,
          ConnectionState::Error => true,
        };
        if reconnect {
          true
        } else {
          return sqldata::make_lapin_info(Some(&lc), token).await;
        }
      }
      None => true, // connection missing!
    },
    Err(e) => {
      error!("lapin_conn poisoned: {e:?}");
      false
    }
  };

  // existing connection has bad state. try reconnect.
  if reconnect {
    // have to do the write() after the read() is out of scope.
    match state.lapin_conn.write() {
      Ok(mut lcmod) => {
        match lapin::Connection::connect(uri.as_str(), lapin::ConnectionProperties::default()).await
        {
          Err(e) => {
            error!("amqp connection error: {e:?}");
            None
          }
          Ok(conn) => {
            info!("amqp reconnected!");
            let ret = sqldata::make_lapin_info(Some(&conn), token).await;
            *lcmod = Some(conn);
            ret
          }
        }
      }
      _ => None,
    }
  } else {
    None
  }
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
      let ls = local_server_id(&conn)?;
      let luuid = Uuid::from_str(ls.uuid.as_str())?;
      let now = now()?;
      let syncstart = zkprotocol::sync_data::SyncStart {
        after: rq.after,
        before: now,
        server: luuid,
      };
      let ss = sync::sync_stream(
        conn,
        PathBuf::from(&config.file_path),
        uid,
        None,
        None,
        None,
        None,
        syncstart,
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
  token: Option<String>,
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
        edittab: gzne.edittab.clone(),
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
        Some(zkne) => Ok(PrivateReply::PvyZkNoteAndLinksWhat(zkne)),
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
      let li = connect_and_make_lapin_info(state, token).await;
      let (_id, s) = sqldata::save_zknote(&conn, &li, &state.server, uid, &sbe, None).await?;
      Ok(PrivateReply::PvySavedZkNote(s))
    }
    PrivateRequest::PvqSaveZkLinks(msg) => {
      let _ = sqldata::save_zklinks(&state.config.orgauth_config.db.as_path(), uid, &msg.links)?;
      Ok(PrivateReply::PvySavedZkLinks)
    }
    PrivateRequest::PvqSaveZkNoteAndLinks(sznpl) => {
      let li = connect_and_make_lapin_info(state, token).await;
      let (_, szkn) =
        sqldata::save_zknote(&conn, &li, &state.server, uid, &sznpl.note, None).await?;
      let _s = sqldata::save_savezklinks(&conn, uid, szkn.id, &sznpl.links)?;
      Ok(PrivateReply::PvySavedZkNoteAndLinks(szkn))
    }
    PrivateRequest::PvqSaveImportZkNotes(gzl) => {
      let li = connect_and_make_lapin_info(state, token).await;
      sqldata::save_importzknotes(&conn, &li, &state.server, uid, gzl).await?;
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
      let server = state.server.clone();
      let li = connect_and_make_lapin_info(state, token.clone()).await;
      let lapin_channelx = li.map(|li| li.channel).clone();

      std::thread::spawn(move || {
        let rt = actix_rt::System::new();

        async fn startit(
          lgb: Arc<std::sync::RwLock<girlboss::Girlboss<JobId, girlboss::Monitor>>>,
          dbpath: PathBuf,
          file_path: PathBuf,
          uid: UserId,
          jid: JobId,
          server: Server,
          lapin_channel: Option<lapin::Channel>,
          token: Option<String>,
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
              let mut callbacks = &mut zknotes_callbacks();
              let server = server.clone();
              // let lapin_channel = lapin_channelx.clone();
              write!(gbm, "starting sync");

              let li = match (lapin_channel, token) {
                (Some(channel), Some(token)) => Some(LapinInfo { channel, token }),
                _ => None,
              };
              // None for now!
              let r =
                sync::sync(&dbpath, &file_path, &li, uid, &server, &mut callbacks, &gbm).await;
              match r {
                Ok(_) => write!(gbm, "sync completed"),
                Err(e) => write!(gbm, "sync err: {:?}", e),
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

        rt.block_on(startit(
          lgb,
          dbpath,
          file_path,
          uid,
          jid,
          server,
          lapin_channelx,
          token,
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
    PrivateRequest::PvqSyncFiles(znsrq) => {
      let dbpath: PathBuf = state.config.orgauth_config.db.to_path_buf();
      let file_path: PathBuf = state.config.file_path.to_path_buf();
      let file_tmp_path: PathBuf = state.config.file_tmp_path.to_path_buf();
      let uid: UserId = uid;
      let jid = new_jobid(state, uid);
      let lgb = state.girlboss.clone();
      let zns = znsrq.clone();
      let li = connect_and_make_lapin_info(state, token).await;

      std::thread::spawn(move || {
        let rt = actix_rt::System::new();

        async fn startit(
          lgb: Arc<std::sync::RwLock<girlboss::Girlboss<JobId, girlboss::Monitor>>>,
          lapin_info: Option<LapinInfo>,
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
                  &lapin_info,
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
          li,
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
        edittab: gzne.edittab.clone(),
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
        Some(zkne) => Ok(PublicReply::PbyZkNoteAndLinksWhat(zkne)),
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
