use crate::error as zkerr;
use crate::search::build_sql;
use crate::search::{search_zknotes, search_zknotes_stream, sync_users, system_user, SearchResult};
use crate::sqldata::{
  self, note_id_for_uuid, save_zklink, save_zknote, user_note_id, zknotes_callbacks,
};
use crate::util::now;
use actix_multipart_rfc7578 as multipart;
use actix_web::error::PayloadError;
use async_stream::try_stream;
use awc;
use bytes::Bytes;
use futures::executor::block_on;
use futures::Stream;
use futures::{future, Future};
use futures_util::TryStreamExt;
use futures_util::{StreamExt, TryFutureExt};
use girlboss::JobReturnStatus;
use log::error;
use orgauth;
use orgauth::data::User;
use orgauth::dbfun::user_id;
use orgauth::endpoints::Callbacks;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio_util::io::StreamReader;
use uuid::Uuid;
use zkprotocol::constants::{PrivateReplies, PrivateStreamingRequests, SpecialUuids};
use zkprotocol::content::{FileStatus, SaveZkNote, SyncMessage, SyncSince, ZkNote, ZkNoteId};
use zkprotocol::messages::{PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage};
use zkprotocol::search::{
  AndOr, OrderDirection, OrderField, Ordering, ResultType, SearchMod, TagSearch, ZkNoteSearch,
};

fn convert_payloaderr(err: PayloadError) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CompletedSync {
  after: Option<i64>,
  now: i64,
}

pub async fn prev_sync(
  conn: &Connection,
  file_path: &Path,
  usernoteid: &ZkNoteId,
) -> Result<Option<CompletedSync>, zkerr::Error> {
  let sysid = user_id(&conn, "system")?;
  let zns = ZkNoteSearch {
    tagsearch: TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::Tag, SearchMod::ZkNoteId],
        term: SpecialUuids::Sync.str().to_string(),
      }),
      ao: zkprotocol::search::AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::Tag, SearchMod::ZkNoteId],
        term: usernoteid.to_string(),
      }),
    },
    offset: 0,
    limit: Some(1),
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: false,
    ordering: Some(Ordering {
      field: OrderField::Changed,
      direction: OrderDirection::Descending,
    }),
  };

  if let SearchResult::SrNote(res) = search_zknotes(conn, &file_path, sysid, &zns)? {
    match res.notes.first() {
      Some(n) => match serde_json::from_str::<CompletedSync>(n.content.as_str()) {
        Ok(s) => Ok(Some(s)),
        Err(e) => {
          error!("CompletedSync parse error: {:?}", e);
          Ok(None)
        }
      },
      None => Ok(None),
    }
  } else {
    Err("unexpected search result type".into())
  }
}

pub async fn save_sync(
  conn: &Connection,
  _uid: i64,
  usernoteid: i64,
  sync: CompletedSync,
) -> Result<i64, Box<dyn std::error::Error>> {
  let syncid = note_id_for_uuid(conn, &Uuid::parse_str(SpecialUuids::Sync.str())?)?;
  let sysid = user_id(&conn, "system")?;

  let (id, _szn) = save_zknote(
    conn,
    sysid, // save under system id.
    &SaveZkNote {
      id: None,
      title: "sync".to_string(),
      pubid: None,
      content: serde_json::to_string_pretty(&serde_json::to_value(sync)?)?,
      editable: false,
      showtitle: false,
      deleted: false,
    },
  )?;

  // link to 'sync' system note.
  save_zklink(conn, id, syncid, sysid, None)?;

  // link to user note for access
  save_zklink(conn, id, usernoteid, sysid, None)?;

  Ok(id)
}

pub struct TempTableNames {
  pub notetemp: String,
  pub linktemp: String,
  pub archivelinktemp: String,
}

pub fn temp_tables(conn: &Connection) -> Result<TempTableNames, zkerr::Error> {
  // // create temporary tables for links and notes we get from the remote.
  // create temporary tables for links and notes we get from the remote.
  let id = sqldata::update_single_value(&conn, "sync_id", |x| match x.parse::<i64>() {
    Ok(n) => (n + 1).to_string(),
    Err(_) => 0.to_string(),
  })?;

  let notetemp = format!("notetemp_{}", id);
  let linktemp = format!("linktemp_{}", id);
  let archivelinktemp = format!("archivelinktemp_{}", id);

  // temporary tables.  should drop when the db connection ends.
  conn.execute(
    format!(
      "create temporary table {} (\"id\" integer primary key not null)",
      notetemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      "create temporary table {} (
      \"fromid\" INTEGER NOT NULL,
      \"toid\" INTEGER NOT NULL,
      \"user\" INTEGER NOT NULL)",
      linktemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      "CREATE UNIQUE INDEX \"{}unq\" ON \"{}\" (\"fromid\", \"toid\", \"user\")",
      linktemp, linktemp
    )
    .as_str(),
    params![],
  )?;

  conn.execute(
    format!(
      "create temporary table {} (\"id\" integer primary key not null)",
      archivelinktemp
    )
    .as_str(),
    params![],
  )?;

  Ok(TempTableNames {
    notetemp,
    linktemp,
    archivelinktemp,
  })
}

pub async fn sync(
  dbpath: &Path,
  file_path: &Path,
  uid: i64,
  // callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let conn = Arc::new(sqldata::connection_open(dbpath)?);
  let user = orgauth::dbfun::read_user_by_id(&conn, uid)?; // TODO pass this in from calling ftn?
  let extra_login_data = sqldata::read_extra_login_data(&conn, user.id)?;
  let mut callbacks = &mut zknotes_callbacks();

  // get previous sync.
  let after = prev_sync(&conn, &file_path, &extra_login_data.zknote)
    .await?
    .map(|cs| cs.now);

  let now = now()?;

  let tr = conn.unchecked_transaction()?;

  let ttn = temp_tables(&conn)?;

  // TODO: pass in 'now'?
  let res = sync_from_remote(
    &conn,
    &user,
    after,
    &file_path,
    &ttn.notetemp,
    &ttn.linktemp,
    &ttn.archivelinktemp,
    callbacks,
  )
  .await?;
  if res.what != PrivateReplies::SyncComplete {
    Ok(res)
  } else {
    let remres = sync_to_remote(
      conn.clone(),
      file_path,
      &user,
      Some(ttn.notetemp.clone()),
      Some(ttn.linktemp.clone()),
      Some(ttn.archivelinktemp.clone()),
      after,
      callbacks,
    )
    .await?;

    let unote = user_note_id(&conn, user.id)?;
    save_sync(&conn, user.id, unote, CompletedSync { after, now }).await?;

    tr.commit()?;

    Ok(remres)
  }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum DownloadResult {
  Downloaded,
  AlreadyDownloaded,
  NoSource,
  DownloadFailed,
}

pub async fn download_file(
  conn: &Connection,
  user: &User,
  tmp_files_dir: &Path,
  files_dir: &Path,
  note_id: i64,
) -> Result<DownloadResult, zkerr::Error> {
  // get the note hash, and verify there's a source with this user's id.
  let (uuid, ohash, fs_user_id): (String, Option<String>, Option<i64>) = conn.query_row(
    "select N.uuid, F.hash, FS.user_id from zknote N
      left join file F on N.file = F.id 
      left join file_source FS on F.id == FS.file_id and FS.user_id = ?2
      where N.id = ?1",
    params![note_id, user.id],
    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
  )?;

  let hash = match ohash {
    Some(h) => h,
    None => return Ok(DownloadResult::DownloadFailed),
  };

  if fs_user_id == None {
    return Ok(DownloadResult::NoSource);
  }

  let temphashpath = tmp_files_dir.join(Path::new(hash.as_str()));
  let finalhashpath = files_dir.join(Path::new(hash.as_str()));

  // file exists?
  if finalhashpath.exists() {
    // file already exists.
    return Ok(DownloadResult::AlreadyDownloaded);
  }

  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync - not a remote user".into()),
  };

  let file_uri = awc::http::Uri::try_from(format!("{}/file/{}", url, uuid).as_str())
    .map_err(|x| zkerr::Error::String(x.to_string()))?;

  let res = awc::Client::new()
    .get(file_uri)
    .cookie(cookie::Cookie::parse_encoded(c)?)
    .timeout(Duration::from_secs(60 * 60))
    .send()
    .await
    .map_err(|e| zkerr::Error::String(e.to_string()))?;

  // Write the body to a file.
  let mut file = tokio::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .open(temphashpath.clone()) // temporary filename
    .await
    .map_err(|e| {
      zkerr::annotate_string(
        format!("error saving downloaded file: {:?}", temphashpath),
        e.into(),
      )
    })?;

  res
    .try_for_each(|bytes| match block_on(file.write(&bytes)) {
      Ok(_) => future::ok(()),
      Err(e) => future::err(e.into()),
    })
    .await
    .map_err(|e| zkerr::Error::String(e.to_string()))?;

  let fh = sha256::try_digest(temphashpath.as_path())?;

  // does the hash match?
  if fh != hash {
    std::fs::remove_file(temphashpath)?;
    return Err(zkerr::Error::String(
      "downloaded file hash doesn't match!".to_string(),
    ));
  }

  // again, file exists?
  if finalhashpath.exists() {
    // new file already exists.
    std::fs::remove_file(temphashpath)?;
  } else {
    // move into hashed-files dir.
    std::fs::rename(temphashpath, finalhashpath)?;
  }

  // TODO: remove source records??
  Ok(DownloadResult::Downloaded)
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum UploadResult {
  Uploaded,
  NotAFile,
  FileNotPresent,
  ExistsOnRemote,
  UploadFailed,
}

pub async fn upload_file(
  conn: &Connection,
  user: &User,
  files_dir: &Path,
  note_id: i64,
) -> Result<UploadResult, zkerr::Error> {
  // get the note hash
  let (uuid, ohash): (String, Option<String>) = conn.query_row(
    "select N.uuid, F.hash from zknote N
      left join file F on N.file = F.id 
      where N.id = ?1",
    params![note_id],
    |row| Ok((row.get(0)?, row.get(1)?)),
  )?;

  let hash = match ohash {
    Some(h) => h,
    None => return Ok(UploadResult::NotAFile),
  };

  let hashpath = files_dir.join(Path::new(hash.as_str()));

  // file exists?
  if !hashpath.exists() {
    // file already exists.
    return Ok(UploadResult::FileNotPresent);
  }

  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't upload file - not a remote user".into()),
  };

  let cookie = cookie::Cookie::parse_encoded(c)?;

  // let user_uri = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;

  // is the note on the server, and the file?
  let private_uri = awc::http::Uri::try_from(format!("{}/private", url).as_str())
    .map_err(|x| zkerr::Error::String(x.to_string()))?;

  let getzknote = PrivateMessage {
    what: zkprotocol::constants::PrivateRequests::GetZkNote,
    data: Some(serde_json::to_value(Uuid::from_str(uuid.as_str())?)?),
  };
  let gj = serde_json::to_value(getzknote)?;

  let client = awc::Client::new();

  let mut res = client
    .post(private_uri)
    .cookie(cookie.clone())
    .timeout(Duration::from_secs(60 * 60))
    .send_json(&gj)
    .await
    .map_err(|e| zkerr::Error::String(e.to_string()))?;

  let b: Bytes = res
    .body()
    .map_err(|e| zkerr::Error::String(e.to_string()))
    .await?;

  let j = serde_json::from_slice(b.as_ref())?;

  let msg: PrivateReplyMessage = serde_json::from_value(j)?;
  assert!(msg.what == PrivateReplies::ZkNote);

  let zkn: ZkNote = serde_json::from_value(msg.content)?;

  match zkn.filestatus {
    FileStatus::NotAFile => return Ok(UploadResult::NotAFile),
    FileStatus::FileMissing => (),
    FileStatus::FilePresent => return Ok(UploadResult::ExistsOnRemote),
  }

  let upload_uri = awc::http::Uri::try_from(format!("{}/upload", url).as_str())
    .map_err(|x| zkerr::Error::String(x.to_string()))?;

  let mut form = multipart::client::multipart::Form::default();
  form.add_file_with_mime(zkn.title, hashpath, mime::STAR_STAR)?;

  let mut rq = client
    .post(upload_uri)
    .cookie(cookie)
    .timeout(Duration::from_secs(60 * 60))
    .content_type(form.content_type())
    .send_body(multipart::client::multipart::Body::from(form))
    .await
    .map_err(|e| zkerr::Error::String(e.to_string()))?;

  let prm: PrivateReplyMessage = serde_json::from_slice(
    rq.body()
      .map_err(|e| zkerr::Error::String(e.to_string()))
      .await?
      .as_ref(),
  )?;

  if prm.what == PrivateReplies::FileSyncComplete {
    Ok(UploadResult::Uploaded)
  } else {
    Ok(UploadResult::UploadFailed)
  }
}

pub async fn sync_files_down(
  conn: &Connection,
  temp_file_path: &Path,
  file_path: &Path,
  uid: i64,
  search: &ZkNoteSearch,
) -> Result<Vec<DownloadResult>, Box<dyn std::error::Error>> {
  // TODO pass this in from calling ftn?
  let user = orgauth::dbfun::read_user_by_id(&conn, uid)?;

  let (sql, args) = build_sql(&conn, uid, &search, None)?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let rec_iter = pstmt.query_and_then(rusqlite::params_from_iter(args.iter()), |row| {
    Ok::<i64, rusqlite::Error>(row.get(0)?)
  })?;

  let mut resvec = Vec::new();
  for rec in rec_iter {
    match rec {
      Ok(id) => {
        match download_file(&conn, &user, temp_file_path, file_path, id).await? {
          DownloadResult::Downloaded => resvec.push(DownloadResult::Downloaded),
          DownloadResult::AlreadyDownloaded => (), // resvec.push(DownloadResult::AlreadyDownloaded),
          DownloadResult::NoSource => resvec.push(DownloadResult::NoSource),
          DownloadResult::DownloadFailed => resvec.push(DownloadResult::DownloadFailed),
        };
      }
      Err(_) => (),
    }
  }

  Ok(resvec)
}
pub async fn sync_files_up(
  conn: &Connection,
  file_path: &Path,
  uid: i64,
  search: &ZkNoteSearch,
) -> Result<Vec<UploadResult>, Box<dyn std::error::Error>> {
  // TODO pass this in from calling ftn?
  let user = orgauth::dbfun::read_user_by_id(&conn, uid)?;

  let (sql, args) = build_sql(&conn, uid, &search, None)?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let rec_iter = pstmt.query_and_then(rusqlite::params_from_iter(args.iter()), |row| {
    Ok::<i64, rusqlite::Error>(row.get(0)?)
  })?;

  let mut resvec = Vec::new();

  for rec in rec_iter {
    match rec {
      Ok(id) => {
        match upload_file(&conn, &user, file_path, id).await? {
          UploadResult::Uploaded => resvec.push(UploadResult::Uploaded),
          UploadResult::NotAFile => resvec.push(UploadResult::NotAFile),
          UploadResult::ExistsOnRemote => resvec.push(UploadResult::ExistsOnRemote),
          UploadResult::FileNotPresent => resvec.push(UploadResult::FileNotPresent),
          UploadResult::UploadFailed => resvec.push(UploadResult::UploadFailed),
        };
      }
      Err(_) => (),
    }
  }

  Ok(resvec)
}

pub async fn sync_from_remote(
  conn: &Connection,
  user: &User,
  after: Option<i64>,
  file_path: &Path,
  notetemp: &String,
  linktemp: &String,
  archivelinktemp: &String,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>> {
  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync - not a remote user".into()),
  };

  let user_url = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;
  let mut parts = awc::http::uri::Parts::default();
  parts.scheme = user_url.scheme().cloned();
  parts.authority = user_url.authority().cloned();
  parts.path_and_query = Some(awc::http::uri::PathAndQuery::from_static("/stream"));
  let uri = awc::http::Uri::from_parts(parts).map_err(|x| zkerr::Error::String(x.to_string()))?;

  let cookie = cookie::Cookie::parse_encoded(c)?;
  let res = awc::Client::new()
    .post(uri)
    .cookie(cookie)
    .timeout(Duration::from_secs(60 * 60))
    .send_json(&serde_json::to_value(PrivateStreamingMessage {
      what: PrivateStreamingRequests::Sync,
      data: Some(serde_json::to_value(SyncSince { after })?),
    })?)
    .await?;

  let mut sr = StreamReader::new(res.map_err(convert_payloaderr));

  let reply = sync_from_stream(
    conn,
    file_path,
    Some(notetemp),
    Some(linktemp),
    Some(archivelinktemp),
    callbacks,
    &mut sr,
  )
  .await?;
  Ok(reply)
}

pub async fn read_sync_message<S>(
  line: &mut String,
  br: &mut StreamReader<S, bytes::Bytes>,
) -> Result<SyncMessage, Box<dyn std::error::Error>>
where
  S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
  line.clear();
  if br.read_line(line).await? == 0 {
    return Err::<SyncMessage, Box<dyn std::error::Error>>(
      zkerr::Error::String("empty stream!".to_string()).into(),
    );
  }
  let sm = serde_json::from_str(line.trim())?;
  Ok(sm)
}

pub async fn sync_from_stream<S>(
  conn: &Connection,
  file_path: &Path,
  notetemp: Option<&str>,
  linktemp: Option<&str>,
  archivelinktemp: Option<&str>,
  callbacks: &mut Callbacks,
  br: &mut StreamReader<S, bytes::Bytes>,
) -> Result<PrivateReplyMessage, Box<dyn std::error::Error>>
where
  S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
  // TODO: pass in now instead of compute here?
  let now = now()?;
  let sysid = user_id(&conn, "system")?;

  let mut line = String::new();

  let mut sm;

  sm = read_sync_message(&mut line, br).await?;

  let (_after, remotenow) = match sm {
    SyncMessage::SyncStart(after, rn) => (after, rn),
    _ => return Err(format!("expected SyncStart; unexpected syncmessage: {:?}", sm).into()),
  };

  // milliseconds
  if (now - remotenow).abs() > 10000 {
    return Err(
      format!(
        "remote time too far off! local: {}, remote: {}",
        now, remotenow,
      )
      .into(),
    );
  }

  sm = read_sync_message(&mut line, br).await?;

  match sm {
    SyncMessage::PhantomUserHeader => (),
    _ => {
      return Err(
        format!(
          "expected PhantomUserHeader; unexpected syncmessage: {:?}",
          sm
        )
        .into(),
      )
    }
  }

  sm = read_sync_message(&mut line, br).await?;
  let mut userhash = HashMap::<i64, i64>::new();

  while let SyncMessage::PhantomUser(ref pu) = sm {
    match userhash.get(&pu.id) {
      Some(_) => (),
      None => {
        match orgauth::dbfun::read_user_by_uuid(&conn, &pu.uuid) {
          Ok(user) => {
            userhash.insert(pu.id, user.id);
          }
          _ => {
            let localpuid = orgauth::dbfun::phantom_user(
              &conn,
              &pu.name,
              pu.uuid,
              Some(pu.data.clone()),
              pu.active,
              &mut callbacks.on_new_user,
            )?;
            userhash.insert(pu.id, localpuid);
          }
        };
      }
    };
    sm = read_sync_message(&mut line, br).await?;
  }

  // ----------------------------------------------------------------------------------
  // current zknotes
  // ----------------------------------------------------------------------------------
  // First should be the current notes.
  if let SyncMessage::ZkSearchResultHeader(ref _zsrh) = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ZkSearchResultHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  sm = read_sync_message(&mut line, br).await?;

  while let SyncMessage::ZkNote(ref note, ref mbf) = sm {
    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;

    let file_id: Option<i64> = match mbf {
      None => None,
      Some(fileinfo) =>
      // already have a file for this hash?
      {
        match conn.query_row(
          "select id from file where hash = ?1",
          params![fileinfo.hash],
          |row| Ok(row.get(0)?),
        ) {
          Err(rusqlite::Error::QueryReturnedNoRows) => {
            // insert new file record, and source.
            let now = orgauth::util::now()?;

            // add table entry
            conn.execute(
              "insert into file (hash, createdate, size)
                       values (?1, ?2, ?3)",
              params![fileinfo.hash, now, fileinfo.size],
            )?;
            let file_id = conn.last_insert_rowid();

            // add source record.
            conn.execute(
              "insert into file_source (file_id, user_id) values (?1, ?2)
                 on conflict do nothing",
              params![file_id, uid],
            )?;
            Some(file_id)
          }
          Ok(file_id) => {
            // does the file exist?
            let stpath = file_path.join(fileinfo.hash.clone());
            if Path::exists(stpath.as_path()) {
              Some(file_id)
            } else {
              // add source record.
              conn.execute(
                "insert into file_source (file_id, user_id) values (?1, ?2)
                 on conflict do nothing",
                params![file_id, uid],
              )?;
              Some(file_id)
            }
          }
          Err(e) => return Err(e.into()).into(),
        }
      }
    };

    let ex =  conn.execute(
        "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, file, createdate, changeddate)
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
      params![
        note.title,
        note.content,
     uid,
        note.pubid,
        note.editable,
        note.showtitle,
       note.deleted,
        note.id.to_string(),
        file_id,
        note.createdate,
        note.changeddate,
      ]
      );
    let id: i64 = match ex {
      Ok(_) => Ok(conn.last_insert_rowid()),
      Err(rusqlite::Error::SqliteFailure(e, Some(s))) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          if s.contains("uuid") {
            let (nid, n) = sqldata::read_zknote_unchecked(&conn, file_path, &note.id)?;
            // TODO: uuid conflict;  resolve with older one becoming archive note.
            // SqliteFailure(Error { code: ConstraintViolation, extended_code: 2067 }, Some("UNIQUE constraint failed: zknote.uuid"));
            if note.changeddate > n.changeddate {
              // note is newer.  archive the old and replace.
              sqldata::save_zknote(
                &conn,
                *uid,
                &SaveZkNote {
                  id: Some(note.id),
                  title: note.title.clone(),
                  pubid: note.pubid.clone(),
                  content: note.content.clone(),
                  editable: note.editable,
                  showtitle: note.showtitle,
                  deleted: note.deleted,
                },
              )
              .map(|x| x.0)
            } else if note.changeddate < n.changeddate {
              // note is older.  add as archive note.
              // may create duplicate archive notes if edited on two systems and then synced.
              sqldata::archive_zknote(&conn, nid, &note).map(|x| x.0)
            } else {
              Ok(nid)
            }
          } else if s.contains("pubid") {
            // Error out to alert user to conflict.
            Err(
              zkerr::Error::String(format!(
                "note exists with duplicate public id: {}",
                note.pubid.clone().unwrap_or("".to_string())
              ))
              .into(),
            )
          } else {
            Err(rusqlite::Error::SqliteFailure(e, Some(s)).into())
          }
        } else {
          Err(rusqlite::Error::SqliteFailure(e, Some(s)).into())
        }
      }
      Err(e) => Err(e.into()),
    }?;

    if let Some(ref nt) = notetemp {
      conn.execute(
        format!("insert into {} values (?1)", nt).as_str(),
        params![id],
      )?;
    }

    sm = read_sync_message(&mut line, br).await?;
  }

  // ----------------------------------------------------------------------------------
  // archive notes
  // ----------------------------------------------------------------------------------
  // After the current notes are the archivenotes.
  if let SyncMessage::ZkSearchResultHeader(ref _zsrh) = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ZkSearchResultHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  sm = read_sync_message(&mut line, br).await?;
  while let SyncMessage::ZkNote(ref note, ref _mbf) = sm {
    // TODO: make file source record (?)

    let uid = userhash
      .get(&note.user)
      .ok_or_else(|| zkerr::Error::String("user not found".to_string()))?;
    assert!(*uid == sysid);
    let mbid = match conn.execute(
      "insert into zknote (title, content, user, pubid, editable, showtitle, deleted, uuid, createdate, changeddate)
       values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
          note.title,
          note.content,
          sysid,    // archivenotes all owned by system.
          note.pubid,
          note.editable,
          note.showtitle,
          note.deleted,
          note.id.to_string(),
          note.createdate,
          note.changeddate,
          ])
        {
          Ok(_x) => Ok::<_, zkerr::Error>(Some(conn.last_insert_rowid())),
          Err(rusqlite::Error::SqliteFailure(e, s)) =>
            if e.code == rusqlite::ErrorCode::ConstraintViolation {
              // if duplicate record, just ignore and go on.
             Ok(None)
            } else {
              return Err(rusqlite::Error::SqliteFailure(e, s).into())
            }
          Err(e) => return Err(e)?,
        }?;
    if let (Some(id), Some(nt)) = (mbid, &notetemp) {
      conn.execute(
        format!("insert into {} values (?1)", nt).as_str(),
        params![id],
      )?;
    }
    sm = read_sync_message(&mut line, br).await?;
  }

  if let SyncMessage::ArchiveZkLinkHeader = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected ArchiveZkLinkHeader: {:?}",
        sm
      )
      .into(),
    );
  }

  // ----------------------------------------------------------------------------------
  // archive links
  // ----------------------------------------------------------------------------------
  sm = read_sync_message(&mut line, br).await?;

  let mut count = 0;
  let mut saved = 0;
  // let mut bytes = 0;

  while let SyncMessage::ArchiveZkLink(ref l) = sm {
    let mbid = match conn.execute(
      "insert into zklinkarchive (fromid, toid, user, linkzknote, createdate, deletedate)
              select FN.id, TN.id, U.id, LN.id, ?1, ?2
              from zknote FN, zknote TN, orgauth_user U, zknote LN
              where FN.uuid = ?3
                and TN.uuid = ?4
                and U.uuid = ?5
                and LN.uuid = ?6",
      params![
        l.createdate,
        l.deletedate,
        l.fromUuid,
        l.toUuid,
        l.userUuid,
        l.linkUuid,
      ],
    ) {
      Ok(_x) => Some(conn.last_insert_rowid()),
      Err(rusqlite::Error::SqliteFailure(e, s)) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          // if duplicate record, just ignore and go on.
          None
        } else {
          return Err(rusqlite::Error::SqliteFailure(e, s).into());
        }
      }
      Err(e) => return Err(e)?,
    };
    if let (Some(lt), Some(id)) = (&archivelinktemp, mbid) {
      conn.execute(
        format!("insert into {} values (?1)", lt).as_str(),
        params![id],
      )?;
    }
    count = count + 1;
    saved = saved + mbid.map_or(0, |_| 1);
    // bytes = bytes + nc;
    sm = read_sync_message(&mut line, br).await?;
  }

  // ----------------------------------------------------------------------------------
  // current links
  // ----------------------------------------------------------------------------------
  if let SyncMessage::UuidZkLinkHeader = sm {
  } else {
    return Err(
      format!(
        "unexpected syncmessage, expected UuidZkLinkHeader: {:?}",
        sm
      )
      .into(),
    );
  }
  count = 0;
  saved = 0;

  sm = read_sync_message(&mut line, br).await?;

  while let SyncMessage::UuidZkLink(ref l) = sm {
    let ins = match conn.execute(
      "with vals(a,b,c,d,e) as (
              select FN.id, TN.id, U.id, LN.id, ?1
              from zknote FN, zknote TN, orgauth_user U
                left outer join zknote LN
                 on LN.uuid = ?5
              where FN.uuid = ?2
                and TN.uuid = ?3
                and U.uuid = ?4)
              insert into zklink (fromid, toid, user, linkzknote, createdate)
                select * from vals",
      params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
    ) {
      Ok(c) => Ok(c),
      Err(rusqlite::Error::SqliteFailure(e, s)) => {
        if e.code == rusqlite::ErrorCode::ConstraintViolation {
          // do update, since we can't ON CONFLICT without a values () clause.
          let count = conn.execute(
            "with vals(a,b,c,d,e) as (
                    select FN.id, TN.id, U.id, LN.id, ?1
                    from zknote FN, zknote TN, orgauth_user U
                      left outer join zknote LN
                       on LN.uuid = ?5
                    where FN.uuid = ?2
                      and TN.uuid = ?3
                      and U.uuid = ?4)
                    update zklink set linkzknote = vals.d, createdate = vals.e
                      from vals
                      where fromid = vals.a
                        and toid = vals.b
                        and user = vals.c",
            params![l.createdate, l.fromUuid, l.toUuid, l.userUuid, l.linkUuid],
          )?;
          Ok(count)
        } else {
          Err(rusqlite::Error::SqliteFailure(e, s))
        }
      }
      Err(e) => Err(e),
    }?;

    if let Some(lt) = &linktemp {
      if ins == 1 {
        conn.execute(
          format!(
            "insert into {} (fromid, toid, user)
            select FN.id, TN.id, U.id
              from zknote FN, zknote TN, orgauth_user U
                  where FN.uuid = ?1
                    and TN.uuid = ?2
                    and U.uuid = ?3",
            lt
          )
          .as_str(),
          params![l.fromUuid, l.toUuid, l.userUuid],
        )?;
      }
    }

    count = count + 1;
    saved = saved + ins;
    // bytes = bytes + nc;
    sm = read_sync_message(&mut line, br).await?;
  }
  // drop zklinks which have a zklinkarchive with newer deletedate
  let _dropped = conn.execute(
    "with dels as (select ZL.fromid, ZL.toid, ZL.user from zklink ZL, zklinkarchive ZLA
        where ZL.fromid = ZLA.fromid
        and ZL.toid = ZLA.toid
        and ZL.user = ZLA.user
        and ZL.createdate < ZLA.deletedate)
        delete from zklink where
          (zklink.fromid, zklink.toid, zklink.user) in dels ",
    params![],
  )?;

  Ok(PrivateReplyMessage {
    what: PrivateReplies::SyncComplete,
    content: serde_json::Value::Null,
  })
}

// Make a stream of all the records needed to sync the remote.
pub async fn sync_to_remote(
  conn: Arc<Connection>,
  files_dir: &Path,
  user: &User,
  exclude_notes: Option<String>,
  exclude_links: Option<String>,
  exclude_archivelinks: Option<String>,
  after: Option<i64>,
  callbacks: &mut Callbacks,
) -> Result<PrivateReplyMessage, zkerr::Error> {
  // TODO: get time on remote system, bail if too far out.

  let (c, url) = match (user.cookie.clone(), user.remote_url.clone()) {
    (Some(c), Some(url)) => (c, url),
    _ => return Err("can't remote sync - not a remote user".into()),
  };

  let user_url = awc::http::Uri::try_from(url).map_err(|x| zkerr::Error::String(x.to_string()))?;
  let mut parts = awc::http::uri::Parts::default();
  parts.scheme = user_url.scheme().cloned();
  parts.authority = user_url.authority().cloned();
  parts.path_and_query = Some(awc::http::uri::PathAndQuery::from_static("/upstream"));
  let uri = awc::http::Uri::from_parts(parts).map_err(|x| zkerr::Error::String(x.to_string()))?;

  let cookie = cookie::Cookie::parse_encoded(c)?;

  let ss = sync_stream(
    conn,
    PathBuf::from(files_dir),
    user.id,
    exclude_notes,
    exclude_links,
    exclude_archivelinks,
    after,
    callbacks,
  );

  // -----------------------------------
  // Writing the stream to a file
  // -----------------------------------
  // use futures::executor::block_on;
  // use futures::{future, future::FutureExt, stream, stream::StreamExt};
  // use tokio::io::AsyncWriteExt;
  // let mut file = tokio::fs::OpenOptions::new()
  //   .write(true)
  //   .create(true)
  //   .open("some_file")
  //   .await
  //   .unwrap();

  // ss.for_each(|item| match item {
  //   Ok(bytes) => {
  //     block_on(file.write(&bytes));
  //     future::ready(())
  //   }
  //   Err(e) => future::ready(()),
  // })
  // .await;

  // TODO: error?
  awc::Client::new()
    .post(uri)
    .cookie(cookie)
    .timeout(Duration::from_secs(60 * 60))
    .send_body(awc::body::BodyStream::new(ss))
    .map_err(|e| zkerr::Error::String(e.to_string()))
    .await?;

  // TODO: send back Sync results... how many records and etc.

  Ok(PrivateReplyMessage {
    what: PrivateReplies::SyncComplete,
    content: serde_json::Value::Null,
  })
}

pub fn bytesify(
  x: Result<SyncMessage, Box<dyn std::error::Error>>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
  // map errors to SyncErrors so we'll at least see them on the client.
  // not sure how to log these on the server side.
  let sm = match x {
    Ok(sm) => sm,
    Err(e) => SyncMessage::SyncError(e.to_string()),
  };

  serde_json::to_value(sm)
    .map(|x| {
      Bytes::from({
        let mut z = x.to_string();
        z.push_str("\n");
        z
      })
    })
    .map_err(|e| e.into())
}

pub fn new_shares(
  conn: &Connection,
  uid: i64,
  after: i64,
) -> Result<Vec<(i64, ZkNoteId)>, zkerr::Error> {
  // Are there zklinks created since last sync, that go from our user to a share?
  // get all shares that link to this user and to 'share'
  // use async_stream::__private::AsyncStream;
  // let newshares: AsyncStream<Result<(), Box<dyn std::error::Error + 'static>>> = try_stream! {
  // let newshares = try_stream! {
  let mut pstmt = conn.prepare(
    "select N.uuid, A.toid from zklink A, zklink B, zknote N
        where A.fromid = ?1
        and A.toid = B.fromid
        and B.toid = ?2
        and N.id = A.toid
        and A.createdate >= ?3",
  )?;

  let shareid = sqldata::note_id(&conn, "system", "share")?;
  let usernoteid = user_note_id(&conn, uid)?;

  let rec_iter = pstmt.query_map(params![usernoteid, shareid, after], |row| {
    let uuid: String = row.get(0)?;
    let id = row.get(1)?;
    Ok((id, uuid))
  })?;
  let res: Result<Vec<(i64, Uuid)>, zkerr::Error> = rec_iter
    .map(|i| {
      i.map_err(|e| e.into())
        .and_then(|(id, uuid)| Ok((id, Uuid::parse_str(uuid.as_str())?)))
    })
    .collect();
  res
}

// Make a stream of all the records needed to sync the remote.
pub fn sync_stream(
  conn: Arc<Connection>,
  files_dir: PathBuf,
  uid: i64,
  exclude_notes: Option<String>,
  exclude_links: Option<String>,
  exclude_archivelinks: Option<String>,
  after: Option<i64>,
  _callbacks: &mut Callbacks,
) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error + 'static>>> {
  let start = try_stream! { yield SyncMessage::SyncStart(after, now()?); }.map(bytesify);

  // !(et'sync' & u'system')
  // !(et'user' & u'system')
  // and not z'' of the SpecialUuids::
  // (!z'f596bc2c-a882-4c1c-b739-8c4e25f34eb2' &
  // (!z'e82fefee-bcd3-4e2e-b350-9963863e516d' &
  // (!z'466d39ec-2ea7-4d43-b44c-1d3d083f8a9d' &
  // (!z'84f72fd0-8836-43a3-ac66-89e0ab49dd87' &
  // (!z'4fb37d76-6fc8-4869-8ee4-8e05fa5077f7' &
  // (!z'ad6a4ca8-0446-4ecc-b047-46282ced0d84' &
  // (!z'0efcc98f-dffd-40e5-af07-90da26b1d469' &
  // !z'528ccfc2-8488-41e0-a4e1-cbab6406674e')))))))

  // Don't sync 'sync' notes.
  let exclude_sync = TagSearch::Not {
    ts: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId, SearchMod::Tag],
        term: SpecialUuids::Sync.str().to_string(),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::User],
        term: "system".to_string(),
      }),
    }),
  };

  // Don't sync user notes.
  let exclude_user = TagSearch::Not {
    ts: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId, SearchMod::Tag],
        term: SpecialUuids::User.str().to_string(),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::User, SearchMod::Tag],
        term: "system".to_string(),
      }),
    }),
  };

  // Don't sync the special system notes.
  // remind me to write a search parser in rust!  tedious
  let exclude_system = TagSearch::Boolex {
    ts1: Box::new(TagSearch::Not {
      ts: Box::new(TagSearch::SearchTerm {
        mods: vec![SearchMod::ZkNoteId],
        term: SpecialUuids::Public.str().to_string(),
      }),
    }),
    ao: AndOr::And,
    ts2: Box::new(TagSearch::Boolex {
      ts1: Box::new(TagSearch::Not {
        ts: Box::new(TagSearch::SearchTerm {
          mods: vec![SearchMod::ZkNoteId],
          term: SpecialUuids::Comment.str().to_string(),
        }),
      }),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::Boolex {
        ts1: Box::new(TagSearch::Not {
          ts: Box::new(TagSearch::SearchTerm {
            mods: vec![SearchMod::ZkNoteId],
            term: SpecialUuids::Share.str().to_string(),
          }),
        }),
        ao: AndOr::And,
        ts2: Box::new(TagSearch::Boolex {
          ts1: Box::new(TagSearch::Not {
            ts: Box::new(TagSearch::SearchTerm {
              mods: vec![SearchMod::ZkNoteId],
              term: SpecialUuids::Search.str().to_string(),
            }),
          }),
          ao: AndOr::And,
          ts2: Box::new(TagSearch::Boolex {
            ts1: Box::new(TagSearch::Not {
              ts: Box::new(TagSearch::SearchTerm {
                mods: vec![SearchMod::ZkNoteId],
                term: SpecialUuids::User.str().to_string(),
              }),
            }),
            ao: AndOr::And,
            ts2: Box::new(TagSearch::Boolex {
              ts1: Box::new(TagSearch::Not {
                ts: Box::new(TagSearch::SearchTerm {
                  mods: vec![SearchMod::ZkNoteId],
                  term: SpecialUuids::Archive.str().to_string(),
                }),
              }),
              ao: AndOr::And,
              ts2: Box::new(TagSearch::Boolex {
                ts1: Box::new(TagSearch::Not {
                  ts: Box::new(TagSearch::SearchTerm {
                    mods: vec![SearchMod::ZkNoteId],
                    term: SpecialUuids::System.str().to_string(),
                  }),
                }),
                ao: AndOr::And,
                ts2: Box::new(TagSearch::Not {
                  ts: Box::new(TagSearch::SearchTerm {
                    mods: vec![SearchMod::ZkNoteId],
                    term: SpecialUuids::Sync.str().to_string(),
                  }),
                }),
              }),
            }),
          }),
        }),
      }),
    }),
  };

  let ns = match after {
    Some(a) => match new_shares(&conn, uid, a) {
      Ok(ns) => ns,
      Err(_e) => vec![],
    },
    None => vec![],
  };

  let new_shares = !ns.is_empty();

  let ts = match after {
    None => TagSearch::SearchTerm {
      mods: Vec::new(),
      term: "".to_string(),
    },
    Some(a) => {
      let anyafter = TagSearch::SearchTerm {
        mods: vec![SearchMod::After, SearchMod::Mod],
        term: a.to_string(),
      };
      // anyafter, or tagged with any of the share ids.
      ns.into_iter().fold(anyafter, |ts, sh| TagSearch::Boolex {
        ts1: Box::new(TagSearch::Boolex {
          ts1: Box::new(TagSearch::SearchTerm {
            mods: vec![SearchMod::ZkNoteId, SearchMod::Tag],
            term: sh.1.to_string(),
          }),
          ao: AndOr::Or,
          ts2: Box::new(TagSearch::SearchTerm {
            mods: vec![SearchMod::ZkNoteId],
            term: sh.1.to_string(),
          }),
        }),
        ao: AndOr::Or,
        ts2: Box::new(ts),
      })
    }
  };

  // get new notes, and old notes that are attached to shares.
  let zns = ZkNoteSearch {
    tagsearch: ts.clone(),
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: true,
    ordering: None,
  };

  let sync_users = sync_users(conn.clone(), uid, after, &zns).map(bytesify);

  let system_user = system_user(conn.clone()).map(bytesify);

  // TODO: sync_users derived from read_zklinks_since_stream ?

  let full_excl = TagSearch::Boolex {
    ts1: Box::new(ts.clone()),
    ao: AndOr::And,
    ts2: Box::new(TagSearch::Boolex {
      ts1: Box::new(exclude_sync),
      ao: AndOr::And,
      ts2: Box::new(TagSearch::Boolex {
        ts1: Box::new(exclude_user),
        ao: AndOr::And,
        ts2: Box::new(exclude_system),
      }),
    }),
  };

  let zns = ZkNoteSearch {
    tagsearch: full_excl,
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: false,
    deleted: true,
    ordering: None,
  };

  let znstream = search_zknotes_stream(
    conn.clone(),
    files_dir.clone(),
    uid,
    zns,
    exclude_notes.clone(),
  )
  .map(bytesify);

  let ans = ZkNoteSearch {
    tagsearch: ts,
    offset: 0,
    limit: None,
    what: "".to_string(),
    resulttype: ResultType::RtNote,
    archives: true,
    deleted: false,
    ordering: None,
  };

  let anstream =
    search_zknotes_stream(conn.clone(), files_dir, uid, ans, exclude_notes).map(bytesify);

  // if there's a new share, just get all links since the beginning of time.
  let linkafter = if new_shares { after } else { None };

  let als =
    sqldata::read_archivezklinks_stream(conn.clone(), uid, linkafter, exclude_archivelinks.clone())
      .map(bytesify);

  let ls = sqldata::read_zklinks_since_stream(conn, uid, linkafter, exclude_links).map(bytesify);

  let end = try_stream! { yield SyncMessage::SyncEnd; }.map(bytesify);

  start
    .chain(sync_users)
    .chain(system_user)
    .chain(znstream)
    .chain(anstream)
    .chain(als)
    .chain(ls)
    .chain(end)
}
