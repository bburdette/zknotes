#[cfg(test)]
mod tests {
  use crate::error as zkerr;
  use crate::init_server;
  use crate::load_config;
  use crate::sqldata;
  use crate::sqldata::*;
  use crate::sync;
  use crate::sync::*;
  use crate::util;
  use futures_util::pin_mut;
  use futures_util::TryStreamExt;
  use orgauth::data::Login;
  use orgauth::data::RegistrationData;
  use orgauth::data::UserRequestMessage;
  use orgauth::dbfun::read_user_by_id;
  use orgauth::dbfun::update_user;
  use orgauth::dbfun::{new_user, user_id};
  use orgauth::endpoints::log_user_in;
  use orgauth::endpoints::Callbacks;
  use orgauth::endpoints::UuidTokener;
  use rusqlite::params;
  use rusqlite::Connection;
  use std::error::Error;
  use std::fs;
  use std::path::Path;
  use std::path::PathBuf;
  use std::sync::Arc;
  use zkprotocol::search::SearchMod;
  use zkprotocol::search::TagSearch;
  use zkprotocol::search::ZkNoteSearch;
  // use std::thread::spawn;
  use tokio_util::io::StreamReader;
  use uuid::Uuid;
  use zkprotocol::constants::SpecialUuids;
  use zkprotocol::content::ExtraLoginData;
  use zkprotocol::content::{GetZkNoteArchives, SaveZkNote, SavedZkNote, UuidZkLink};

  use std::collections::hash_set::HashSet;
  // Note this useful idiom: importing names from outer (for mod tests) scope.
  // use super::*;

  #[actix_web::test]
  async fn test_sync() -> Result<(), Box<dyn Error>> {
    // let _ = System::new();
    err_test().await
  }

  struct TestStuff {
    visible_notes: Vec<(i64, Uuid)>,
    synced_notes: Vec<(i64, Uuid)>,
    unvisible_notes: Vec<(i64, Uuid)>,
    savedlinks: Vec<UuidZkLink>,
    otherusershare: (i64, Uuid),
    otherusersharenote: (i64, Uuid),
    syncuser: i64,
    syncusernote: i64,
    syncusertoken: Uuid,
    otheruser: i64,
    filenote: i64,
    filepath: PathBuf,
  }

  fn idin(id: &Uuid, szns: &Vec<(i64, Uuid)>) -> bool {
    for (_, szn) in szns {
      if *id == *szn {
        return true;
      }
    }

    false
  }

  fn makenote(
    conn: &Connection,
    uid: i64,
    title: String,
  ) -> Result<(i64, SavedZkNote), zkerr::Error> {
    save_zknote(
      &conn,
      uid,
      &SaveZkNote {
        id: None,
        title,
        showtitle: true,
        pubid: None,
        content: "initial content".to_string(),
        editable: false,
        deleted: false,
      },
    )
  }

  fn setup_db(
    conn: &Connection,
    cb: &mut Callbacks,
    remote_url: String,
    syncuser_token: Option<String>,
    syncuser: Option<(Uuid, ExtraLoginData)>,
    basename: &str,
  ) -> Result<TestStuff, Box<dyn Error>> {
    // system note ids
    let publicid = note_id(&conn, "system", "public")?;
    let shareid = note_id(&conn, "system", "share")?;

    // test users.
    let otheruser = new_user(
      &conn,
      &RegistrationData {
        uid: format!("{}-otheruser", basename),
        pwd: "".to_string(),
        email: "".to_string(),
        remote_url: "".to_string(),
      },
      None,
      None,
      false,
      None,
      None,
      None,
      None,
      None,
      &mut cb.on_new_user,
    )?;

    let (syncuuid, synced) = match syncuser {
      Some((l, r)) => (Some(l), Some(serde_json::to_value(r)?)),
      None => (None, None),
    };

    let syncuser = new_user(
      &conn,
      &RegistrationData {
        uid: format!("{}-syncuser", basename),
        pwd: "".to_string(),
        email: "".to_string(),
        remote_url: remote_url.clone(),
      },
      None,
      None,
      false,
      syncuuid,
      None,
      if remote_url == "" {
        None
      } else {
        Some(remote_url)
      },
      synced,
      syncuser_token,
      &mut cb.on_new_user,
    )?;

    let mut ut = UuidTokener { uuid: None };
    let _usr = log_user_in(&mut ut, cb, conn, syncuser)?;
    let syncusertoken = ut
      .uuid
      .ok_or(zkerr::Error::String("no uuid token!".to_string()))?;

    // returning this.
    let mut visible_notes: Vec<(i64, Uuid)> = Vec::new();
    let mut synced_notes: Vec<(i64, Uuid)> = Vec::new();
    let mut unvisible_notes: Vec<(i64, Uuid)> = Vec::new();
    let mut savedlinks: Vec<UuidZkLink> = Vec::new();

    let savelink = |from, to, user, savedlinks: &mut Vec<UuidZkLink>| -> Result<(), zkerr::Error> {
      save_zklink(&conn, from, to, user, None)?;
      savedlinks.push(read_uuidzklink(&conn, from, to, user)?);
      Ok(())
    };

    // visible notes
    // private note for sync user
    // 'share' note visible to user
    // note linked to share note.
    // public note not otherwise shared with user goes
    // public note owned by user
    // user-link-shared note

    // NOT visible notes
    // otheruser private note NOT visible to user
    // share note NOT visible to user
    // note linked to share note NOT visible to user

    let otheruser_note = user_note_id(&conn, otheruser)?;
    let syncuser_note = user_note_id(&conn, syncuser)?;
    {
      // visible but not synced.
      let ozkn = read_zknote_i64(conn, None, otheruser_note)?;
      let szkn = read_zknote_i64(conn, None, syncuser_note)?;
      visible_notes.push((otheruser_note, ozkn.id));
      visible_notes.push((syncuser_note, szkn.id));
    }

    // private note for sync user
    let (private_note_id, private_notesd) =
      makenote(&conn, syncuser, format!("{} syncuser private", basename))?;
    visible_notes.push((private_note_id, private_notesd.id));

    // share note, syncuser and otheruser both connected.
    let (share_note_id, share_notesd) = makenote(
      &conn,
      otheruser,
      format!("{} otheruser, syncuser share", basename),
    )?;
    visible_notes.push((share_note_id, share_notesd.id));
    synced_notes.push((share_note_id, share_notesd.id));
    savelink(share_note_id, shareid, otheruser, &mut savedlinks)?;
    savelink(syncuser_note, share_note_id, otheruser, &mut savedlinks)?;
    savelink(otheruser_note, share_note_id, otheruser, &mut savedlinks)?;

    // ------------------------------------------------------------------
    // notes visible to syncuser.

    // note owned by otheruser, shared with syncuser through share_note
    let (shared_note_id, shared_notesd) = makenote(
      &conn,
      otheruser,
      format!(
        "{} otheruser shared through otheruser, syncuser share",
        basename
      ),
    )?;
    visible_notes.push((shared_note_id, shared_notesd.id));
    synced_notes.push((shared_note_id, shared_notesd.id));
    savelink(shared_note_id, share_note_id, otheruser, &mut savedlinks)?;

    // public note owned by otheruser.
    let (publid_note_id, publid_notesd) = makenote(
      &conn,
      otheruser,
      format!("{} otheruser public note", basename),
    )?;
    visible_notes.push((publid_note_id, publid_notesd.id));
    synced_notes.push((publid_note_id, publid_notesd.id));
    savelink(publid_note_id, publicid, otheruser, &mut savedlinks)?;

    // public note owned by syncuser, with a public id
    // public id the same on client and server, so should conflict.
    let (public_note_id, public_notesd) = save_zknote(
      &conn,
      syncuser,
      &SaveZkNote {
        id: None,
        title: format!("{} syncuser public note", basename),
        showtitle: true,
        pubid: Some("public-note".to_string()), // test duplicate public notes!
        content: "initial content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    visible_notes.push((public_note_id, public_notesd.id));
    synced_notes.push((public_note_id, public_notesd.id));
    savelink(public_note_id, publicid, syncuser, &mut savedlinks)?; // should be visible, and get synced!
    println!(
      "visible link: {:?} ",
      read_uuidzklink(&conn, public_note_id, publicid, syncuser)?
    );

    // user-link-shared note
    let (user_linked_note_id, user_linked_notesd) = makenote(
      &conn,
      otheruser,
      format!("{} otheruser note linked to syncuser", basename),
    )?;
    visible_notes.push((user_linked_note_id, user_linked_notesd.id));
    savelink(
      user_linked_note_id,
      syncuser_note,
      otheruser,
      &mut savedlinks,
    )?;

    // file note.
    let filesdir = format!("{}_files", basename);
    let fdpath = Path::new(filesdir.as_str());
    match std::fs::remove_dir_all(fdpath) {
      Ok(_) => (),
      Err(e) => println!("remove_dir_all error: {:?}", e),
    };
    std::fs::create_dir(fdpath)
      .map_err(|e| zkerr::annotate_string("create_dir".to_string(), e.into()))?;
    let fname = format!("{}_test_file.txt", basename);
    orgauth::util::write_string(fname.as_str(), format!("{} tesssst", basename).as_str())
      .map_err(|e| zkerr::annotate_string("write_string error".to_string(), e.into()))?;
    let fpath = Path::new(&fname);
    let (filenote, _noteid, _fid) =
      sqldata::make_file_note(&conn, Path::new(filesdir.as_str()), syncuser, &fname, fpath)
        .map_err(|e| {
          zkerr::annotate_string(
            format!("make_file_note error: {}", fpath.display()),
            e.into(),
          )
        })?;

    // ------------------------------------------------------------------
    // notes not visible to syncuser.

    // otheruser private note NOT visible to user
    let (otheruser_private_note_id, otheruser_private_notesd) =
      makenote(&conn, otheruser, format!("{} otheruser private", basename))?;
    unvisible_notes.push((otheruser_private_note_id, otheruser_private_notesd.id));

    // share note, only otheruser connected.
    let (othershare_note_id, othershare_notesd) = makenote(
      &conn,
      otheruser,
      format!("{} otheruser othershare, not visible to syncuser", basename),
    )?;
    unvisible_notes.push((othershare_note_id, othershare_notesd.id));
    savelink(othershare_note_id, shareid, otheruser, &mut savedlinks)?;
    savelink(
      otheruser_note,
      othershare_note_id,
      otheruser,
      &mut savedlinks,
    )?;

    // note linked to share note NOT visible to user
    let (othershared_note_id, othershared_notesd) = makenote(
      &conn,
      otheruser,
      format!(
        "{} otheruser note shared with othershare, not visible to syncuser",
        basename
      ),
    )?;
    unvisible_notes.push((othershared_note_id, othershared_notesd.id));
    savelink(
      othershared_note_id,
      othershare_note_id,
      otheruser,
      &mut savedlinks,
    )?;

    println!("setup_db end");

    let ts = TestStuff {
      synced_notes,
      visible_notes,
      unvisible_notes,
      savedlinks,
      otherusershare: (othershare_note_id, othershare_notesd.id),
      otherusersharenote: (othershared_note_id, othershared_notesd.id),
      syncuser,
      syncusernote: syncuser_note,
      syncusertoken,
      otheruser,
      filenote,
      filepath: Path::new(&filesdir).to_path_buf(),
    };

    Ok(ts)
  }

  // ---------------------------------------------------------------
  // testing
  // ---------------------------------------------------------------
  async fn err_test() -> Result<(), Box<dyn Error>> {
    let dbp_client = Path::new("client.db");
    let dbp_server = Path::new("server.db");

    let systemnotes = HashSet::from([
      Uuid::parse_str(SpecialUuids::Public.str())?,
      Uuid::parse_str(SpecialUuids::Comment.str())?,
      Uuid::parse_str(SpecialUuids::Share.str())?,
      Uuid::parse_str(SpecialUuids::Search.str())?,
      Uuid::parse_str(SpecialUuids::User.str())?,
      Uuid::parse_str(SpecialUuids::Archive.str())?,
      Uuid::parse_str(SpecialUuids::System.str())?,
      Uuid::parse_str(SpecialUuids::Sync.str())?,
    ]);

    let mut cb = zknotes_callbacks();
    println!("0");

    // Set up dbs.
    // for now, copying saved databases that are pre-sync, to speed up
    // dev of these tests.

    // set up server first.
    match fs::copy(Path::new("server.db.saved"), dbp_server) {
      Ok(_) => (), // already have a saved initted db.
      Err(_) => {
        // init and save in *.saved
        match fs::remove_file(dbp_server) {
          Ok(_) => (),
          Err(e) => {
            println!(
              "error removing db: '{}'; {}",
              dbp_server.to_str().unwrap_or(""),
              e
            );
          }
        }
        dbinit(dbp_server, None)?;
        fs::copy(dbp_server, Path::new("server.db.saved"))?;
      }
    }
    let server_conn = connection_open(dbp_server)?;
    let server_ts = setup_db(&server_conn, &mut cb, "".to_string(), None, None, "server")?;

    println!("0.1");
    let ssyncuser = user_id(&server_conn, "server-syncuser")?;
    let ssyncuserld = sqldata::read_extra_login_data(&server_conn, ssyncuser)?;
    let ssu = read_user_by_id(&server_conn, ssyncuser)?;

    match fs::copy(Path::new("client.db.saved"), dbp_client) {
      Ok(_) => (), // already have a saved initted db.
      Err(_) => {
        // init and save in *.saved
        match fs::remove_file(dbp_client) {
          Ok(_) => (),
          Err(e) => {
            println!(
              "error removing db '{}': {}",
              dbp_client.to_str().unwrap_or(""),
              e
            );
          }
        }
        dbinit(dbp_client, None)?;
        fs::copy(dbp_client, Path::new("client.db.saved"))?;
      }
    }
    let client_conn = connection_open(dbp_client)?;
    let client_ts = setup_db(
      &client_conn,
      &mut cb,
      "http://localhost:8010".to_string(),
      Some(server_ts.syncusertoken.to_string()),
      Some((ssu.uuid, ssyncuserld)),
      "client",
    )?;

    println!("0.2");

    // ---------------------------------------------------------------
    // test searches.
    // ---------------------------------------------------------------

    // use user2, since user2 has limited visibility to user1 notes.
    let cotheruser = user_id(&client_conn, "client-otheruser")?;
    let csyncuser = user_id(&client_conn, "client-syncuser")?;
    let ssyncuser = user_id(&server_conn, "server-syncuser")?;
    let caconn = Arc::new(client_conn);
    let saconn = Arc::new(server_conn);

    // initial sync, testing duplicate public ids.
    {
      let server_stream = sync_stream(saconn.clone(), ssyncuser, None, None, None, None, &mut cb);

      let ctr = caconn.unchecked_transaction()?;

      let ttn = temp_tables(&caconn)?;
      fn convert_err(err: Box<dyn Error>) -> std::io::Error {
        println!("convert_err {:?}", err);
        todo!()
      }

      let ss = server_stream.map_err(convert_err);
      pin_mut!(ss);
      let mut br = StreamReader::new(ss);

      match sync_from_stream(
        &caconn,
        &client_ts.filepath,
        Some(&ttn.notetemp),
        Some(&ttn.linktemp),
        Some(&ttn.archivelinktemp),
        &mut cb,
        &mut br,
      )
      .await
      {
        Ok(_) => Err("expected error from public id conflict!")?,
        Err(e) => {
          if e.to_string() == "note exists with duplicate public id: public-note" {
            ()
          } else {
            Err(e)?
          }
        }
      }

      ctr.rollback()?;
    }
    println!("1");

    let _cpubid: i64 = caconn.query_row(
      "select id from zknote where pubid = ?1",
      params!["public-note"],
      |row| Ok(row.get(0)?),
    )?;

    let cpub2 = read_zknotepubid(&caconn, None, "public-note")?;

    println!("2");

    let (_spc2_id, spc2) = save_zknote(
      &caconn,
      csyncuser,
      &SaveZkNote {
        id: Some(cpub2.id),
        title: "public-note 2".to_string(),
        showtitle: true,
        pubid: Some("public-note-client".to_string()), // test duplicate public notes!
        content: cpub2.content,
        editable: false,
        deleted: false,
      },
    )?;
    assert!(cpub2.id == spc2.id);

    {
      let server_stream = sync_stream(saconn.clone(), ssyncuser, None, None, None, None, &mut cb);
      // -----------------------------------
      // Writing the stream to a file
      // -----------------------------------
      use futures::executor::block_on;
      use futures::future;
      use tokio::io::AsyncWriteExt;
      let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("server_stream.txt")
        .await
        .unwrap();

      server_stream
        .try_for_each(|bytes| {
          future::ready(
            block_on(file.write(&bytes))
              .map(|_| ())
              .map_err(|e| e.into()),
          )
        })
        .await?;
    }
    {
      let client_stream = sync_stream(
        caconn.clone(),
        csyncuser,
        // Some(ttn.notetemp.clone()),
        None,
        // Some(ttn.linktemp.clone()),
        None,
        // Some(ttn.archivelinktemp.clone()),
        None,
        None,
        &mut cb,
      );
      // -----------------------------------
      // Writing the stream to a file
      // -----------------------------------
      use futures::executor::block_on;
      use futures::future;
      use tokio::io::AsyncWriteExt;
      let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("client_stream.txt")
        .await
        .unwrap();

      client_stream
        .try_for_each(|bytes| {
          future::ready(
            block_on(file.write(&bytes))
              .map(|_| ())
              .map_err(|e| e.into()),
          )
        })
        .await?;
    }
    // ------------------------------------------------------------
    // sync from server to client.
    // ------------------------------------------------------------
    let server_stream = sync_stream(saconn.clone(), ssyncuser, None, None, None, None, &mut cb);

    // let ctr = caconn.unchecked_transaction()?;

    let ttn = temp_tables(&caconn)?;
    fn convert_err(err: Box<dyn Error>) -> std::io::Error {
      println!("convert_err {:?}", err);
      todo!()
    }

    let ss = server_stream.map_err(convert_err);
    pin_mut!(ss);
    let mut br = StreamReader::new(ss);

    sync_from_stream(
      &caconn,
      &client_ts.filepath,
      Some(&ttn.notetemp),
      Some(&ttn.linktemp),
      Some(&ttn.archivelinktemp),
      &mut cb,
      &mut br,
    )
    .await?;

    // ------------------------------------------------------------
    // sync from client to server.
    // ------------------------------------------------------------
    let client_stream = sync_stream(
      caconn.clone(),
      csyncuser,
      Some(ttn.notetemp),
      Some(ttn.linktemp),
      Some(ttn.archivelinktemp),
      None,
      &mut cb,
    );

    let cs = client_stream.map_err(convert_err);
    pin_mut!(cs);
    let mut cbr = StreamReader::new(cs);

    sync_from_stream(
      &saconn,
      &server_ts.filepath,
      None,
      None,
      None,
      &mut cb,
      &mut cbr,
    )
    .await?;

    let postsync_time = util::now()?;

    println!("blah minus one");

    // ------------------------------------------------------------
    // post-sync tests.
    // ------------------------------------------------------------

    // ------------------------------------------------------------
    // phantom user on server?
    let scotheruser = user_id(&saconn, "client-otheruser")?;
    // check that user note ids and user uuids match on client and server.
    {
      let celd = sqldata::read_extra_login_data(&caconn, csyncuser)?;
      let seld = sqldata::read_extra_login_data(&saconn, ssyncuser)?;
      assert!(celd.zknote == seld.zknote);
      let cu = orgauth::dbfun::read_user_by_id(&caconn, csyncuser)?;
      let su = orgauth::dbfun::read_user_by_id(&saconn, ssyncuser)?;
      assert!(cu.uuid == su.uuid);
    }

    {
      let cu = orgauth::dbfun::read_user_by_id(&caconn, cotheruser)?;
      let su = orgauth::dbfun::read_user_by_id(&saconn, scotheruser)?;
      assert!(cu.uuid == su.uuid);
      let celd = sqldata::read_extra_login_data(&caconn, cotheruser)?;
      let seld = sqldata::read_extra_login_data(&saconn, scotheruser)?;
      assert!(celd.zknote == seld.zknote);
    }

    // ------------------------------------------------------------
    println!("3");
    read_zknotepubid(&caconn, None, "public-note")?;
    println!("3.5");
    read_zknotepubid(&caconn, Some(csyncuser), "public-note")?;
    println!("4");
    let cpublic_note_client = read_zknotepubid(&caconn, Some(csyncuser), "public-note-client")?;

    println!("5");
    // client public notes on server.
    let spublic_note_client = read_zknotepubid(&saconn, Some(ssyncuser), "public-note-client")?;

    // ids are the same.
    assert!(spublic_note_client.id == cpublic_note_client.id);

    println!("6");

    {
      println!("cpub2.id: {}", cpub2.id);
      // archives present for revised notes.
      let cpcarchs = read_zknotearchives(
        &caconn,
        csyncuser,
        &GetZkNoteArchives {
          zknote: cpub2.id,
          offset: 0,
          limit: None,
        },
      )?;

      println!("7.2");
      let spcarchs = read_zknotearchives(
        &saconn,
        ssyncuser,
        &GetZkNoteArchives {
          zknote: cpub2.id,
          offset: 0,
          limit: None,
        },
      )?;

      assert!(cpcarchs.len() > 0);

      assert!(cpcarchs == spcarchs);
    }

    // ------------------------------------------------------------
    // Notes get synced, client to server and server to client.
    // archive note for each type of note, is synced both ways.

    println!("visible notes: {:?}", client_ts.visible_notes);
    println!("unvisible notes: {:?}", client_ts.unvisible_notes);

    // Visible notes from client are in server?
    for (_, szn) in &client_ts.synced_notes {
      println!("checking clieint syned: {:?}", szn);
      match sqldata::read_zknote(&saconn, Some(ssyncuser), &szn) {
        Err(zkerr::Error::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => {
          Err(format!("not found: {:?}", szn).into())
        }
        Err(e) => Err(e),
        Ok(x) => Ok(x),
      }?;
    }

    // Visible notes from server are in client?
    for (_, szn) in &server_ts.synced_notes {
      println!("checking server syned: {:?}", szn);
      sqldata::read_zknote(&caconn, Some(csyncuser), &szn)?;
    }

    // non-visible notes from client are not in server?
    for (_, szn) in client_ts.unvisible_notes {
      println!("checking client unvis: {:?}", szn);
      match sqldata::read_zknote(&saconn, Some(ssyncuser), &szn) {
        Ok(_) => Err(format!(
          "client note was not supposed to sync to server: {}",
          szn
        )),
        Err(_) => Ok(()),
      }?
    }

    for (_, szn) in server_ts.unvisible_notes {
      println!("checking server unvis: {:?}", szn);
      match sqldata::read_zknote(&caconn, Some(csyncuser), &szn) {
        Ok(_) => Err(format!(
          "server note was not supposed to sync to client: {}",
          szn
        )),
        Err(_) => Ok(()),
      }?
    }

    // links between two visible notes should sync.
    // links between less than two visible notes should not sync.

    println!("systemnotes: {:?}", systemnotes);

    for uzl in client_ts.savedlinks {
      let fromid = Uuid::parse_str(uzl.fromUuid.as_str())?;
      let toid = Uuid::parse_str(uzl.toUuid.as_str())?;
      println!(
        "from visible? {} {}",
        &fromid,
        idin(&fromid, &client_ts.visible_notes) || systemnotes.contains(&fromid)
      );
      println!(
        "to visible? {} {}",
        &toid,
        idin(&toid, &client_ts.visible_notes) || systemnotes.contains(&toid)
      );
      if (idin(&fromid, &client_ts.visible_notes) || systemnotes.contains(&fromid))
        && (idin(&toid, &client_ts.visible_notes) || systemnotes.contains(&toid))
      {
        // link should be on the server.
        println!("checking link: {:?}", uzl);
        read_uuidzklink_createdate(
          &saconn,
          uzl.fromUuid.as_str(),
          uzl.toUuid.as_str(),
          uzl.userUuid.as_str(),
        )?;
      } else {
        // link should not be on the server.
        println!("checking unsynced link: {:?}", uzl);
        match read_uuidzklink_createdate(
          &saconn,
          uzl.fromUuid.as_str(),
          uzl.toUuid.as_str(),
          uzl.userUuid.as_str(),
        ) {
          Ok(_) => Err(format!("link shouldn't sync: {:?}", uzl)),
          Err(_) => Ok(()),
        }?;
      }
    }

    println!("blah zero");
    // ------------------------------------------------------------
    // add a new share, or link user in to a share, then sync again.
    // the cases:
    //   - add a share link on the server.  document using that share is on the server.
    //   - add a share link on the client.
    // ------------------------------------------------------------

    save_zklink(
      &saconn,
      server_ts.syncusernote,
      server_ts.otherusershare.0,
      server_ts.otheruser,
      None,
    )?;

    // is otherusersharenote on the client now?
    read_zknote(
      &saconn,
      Some(server_ts.otheruser),
      &server_ts.otherusersharenote.1,
    )
    .map_err(|e| {
      zkerr::annotate_string(
        "othersharenote not accessible to otheruser on server".to_string(),
        e,
      )
    })?;

    // can syncuser access it on the server?
    read_zknote(
      &saconn,
      Some(server_ts.syncuser),
      &server_ts.otherusersharenote.1,
    )
    .map_err(|e| {
      zkerr::annotate_string(
        "othersharenote not accessible to syncuser on server".to_string(),
        e,
      )
    })?;

    // new shares, does it work?
    let ns = sync::new_shares(&saconn, server_ts.syncuser, postsync_time)?;
    assert!(ns == vec![server_ts.otherusershare]);

    // TODO: tweak a file on the server, and on the client.
    // check that those files synced.

    // ------------------------------------------------------------
    // sync from server to client.
    let server_stream = sync_stream(saconn.clone(), ssyncuser, None, None, None, None, &mut cb);

    let ttn = temp_tables(&caconn)?;
    let ss = server_stream.map_err(convert_err);
    pin_mut!(ss);
    let mut br = StreamReader::new(ss);

    sync_from_stream(
      &caconn,
      &client_ts.filepath,
      Some(&ttn.notetemp),
      Some(&ttn.linktemp),
      Some(&ttn.archivelinktemp),
      &mut cb,
      &mut br,
    )
    .await?;

    // ------------------------------------------------------------
    // sync from client to server.
    let client_stream = sync_stream(
      caconn.clone(),
      csyncuser,
      Some(ttn.notetemp),
      Some(ttn.linktemp),
      Some(ttn.archivelinktemp),
      None,
      &mut cb,
    );

    let cs = client_stream.map_err(convert_err);
    pin_mut!(cs);
    let mut cbr = StreamReader::new(cs);

    sync_from_stream(
      &saconn,
      &server_ts.filepath,
      None,
      None,
      None,
      &mut cb,
      &mut cbr,
    )
    .await?;

    // can syncuser access it?
    read_zknote(
      &caconn,
      Some(client_ts.syncuser),
      &server_ts.otherusersharenote.1,
    )
    .map_err(|e| {
      zkerr::annotate_string(
        "othersharenote not accessible to syncuser on client".to_string(),
        e,
      )
    })?;

    // ------------------------------------------------------------
    // file sync testing.

    // start a server in another thread.
    let config = load_config("testserver.toml")?;
    let server = init_server(config.clone())?;
    let handle = server.handle();
    let _joinhandle = tokio::task::spawn(async move { server.await });

    // log in the server-syncuser so they get a cookie.
    let client = reqwest::Client::new();
    let l = UserRequestMessage {
      what: orgauth::data::UserRequest::Login,
      data: Some(serde_json::to_value(Login {
        uid: "server-syncuser".to_string(),
        pwd: "".to_string(),
      })?),
    };
    let res = client
      .post(format!("http://{}:{}/user", config.ip, config.port).as_str())
      .json(&l)
      .send()
      .await?;
    let cookie = match res.headers().get(reqwest::header::SET_COOKIE) {
      Some(ck) => Ok(
        ck.to_str()
          .map_err(|_| zkerr::Error::String("invalid cookie".to_string()))?
          .to_string(),
      ),
      None => Err(zkerr::Error::String("no cookie".to_string())),
    }?;
    let mut client_syncuser = read_user_by_id(&caconn, client_ts.syncuser)?;
    client_syncuser.cookie = Some(cookie);
    update_user(&caconn, &client_syncuser)?;

    // Should be set to sync.
    let fsts = TagSearch::SearchTerm {
      mods: vec![SearchMod::File],
      term: "".to_string(),
    };

    let fssearch = ZkNoteSearch {
      tagsearch: fsts,
      offset: 0,
      limit: None,
      what: "".to_string(),
      resulttype: zkprotocol::search::ResultType::RtListNote,
      archives: false,
      deleted: false, // include deleted notes
      ordering: None,
    };
    // use actix_web::actix_rt::{Arbiter, System};

    let reply = sync_files(
      &caconn,
      client_ts.filepath.as_path(),
      client_ts.syncuser,
      &fssearch,
    )
    .await?;

    let fi = read_file_info(&saconn, server_ts.filenote)?;

    // client should have the server file now.
    assert!(Path::new(
      format!("{}/{}", client_ts.filepath.to_str().expect("wat"), fi.hash).as_str()
    )
    .exists());

    println!("sync_files result: {:?}", reply);

    handle.stop(true).await;

    // ------------------------------------------------------------
    // archive link testing.
    //  - archive links transfer.
    //  - after changes
    // ------------------------------------------------------------
    // sync 2:
    // change all notes above and sync.  updated?
    // delete links.  updated?  hmm.  will unshares ever get synced?
    // reinstate links.  updated?  link archives?

    // Verify notes not visible to user1 are not on server.

    // TESTING:
    // user ids on client and server don't match.
    //   should sync fail if user ids don't match?  do user ids match on remote users now?

    Ok(())
  }
}
