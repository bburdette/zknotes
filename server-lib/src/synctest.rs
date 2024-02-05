#[cfg(test)]
mod tests {
  use crate::interfaces::*;
  use crate::search::*;
  use crate::sqldata::*;
  use crate::sync::*;
  use either::Either;
  use futures::executor::block_on;
  use futures_util::pin_mut;
  use futures_util::TryStreamExt;
  use orgauth::data::RegistrationData;
  use orgauth::dbfun::{new_user, user_id};
  use orgauth::endpoints::Callbacks;
  use rusqlite::params;
  use rusqlite::Connection;
  use std::error::Error;
  use std::fs;
  use std::path::Path;
  use std::sync::Arc;
  use tokio_util::io::StreamReader;
  use uuid::Uuid;
  use zkprotocol::content::GetArchiveZkNote;
  use zkprotocol::content::GetZkNoteArchives;
  use zkprotocol::content::SaveZkNote;
  use zkprotocol::content::SavedZkNote;
  use zkprotocol::search::*;

  // Note this useful idiom: importing names from outer (for mod tests) scope.
  // use super::*;

  #[tokio::test]
  async fn test_sync() {
    let res = match err_test().await {
      Ok(()) => true,
      Err(e) => {
        println!("test failed with error: {:?}", e);
        false
      }
    };
    assert_eq!(res, true);
  }

  fn setup_db(
    conn: &Connection,
    cb: &mut Callbacks,
    user1uuid: Option<Uuid>,
    basename: &str,
  ) -> Result<((i64, SavedZkNote), (i64, SavedZkNote)), Box<dyn Error>> {
    let publicid = note_id(&conn, "system", "public")?;
    let shareid = note_id(&conn, "system", "share")?;

    let uid1 = new_user(
      &conn,
      &RegistrationData {
        uid: format!("{}-user1", basename),
        pwd: "".to_string(),
        email: "".to_string(),
        remote_url: "".to_string(),
      },
      None,
      None,
      false,
      user1uuid,
      None,
      None,
      None,
      None,
      &mut cb.on_new_user,
    )?;

    let uid2 = new_user(
      &conn,
      &RegistrationData {
        uid: format!("{}-user2", basename),
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

    // let _unid1 = user_note_id(&conn, uid1)?;
    let unid2 = user_note_id(&conn, uid2)?;

    let (szn1_1_share_id, szn1_1_share) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} note1", basename),
        showtitle: true,
        pubid: None,
        content: format!("{} note1 content", basename),
        editable: false,
        deleted: false,
      },
    )?;

    // user 1 note 2 - share
    let (szn1_2_share_id, szn1_2_share) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} note2", basename),
        showtitle: true,
        pubid: None,
        content: format!("{} note2 content", basename),
        editable: false,
        deleted: false,
      },
    )?;
    save_zklink(&conn, szn1_2_share_id, shareid, uid1, None)?;

    // user 1 note 3 - share
    let (szn1_3_share_id, szn1_3_share) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note3 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-3 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;
    save_zklink(&conn, szn1_3_share_id, shareid, uid1, None)?;

    // -----------------------------------------

    println!("5");

    // user1 adds user 2 to user 1 share '2'.
    save_zklink(&conn, unid2, szn1_2_share_id, uid1, None)?;

    println!("6");

    // user2 adds user 2 to user1 share '3'.  should fail
    match save_zklink(&conn, unid2, szn1_3_share_id, uid2, None) {
      Ok(_) => panic!("test failed"),
      // Ok(_) => (),
      Err(_) => (),
    };

    println!("7");

    // user 1 note 4 - on share '2'.
    let (szn1_4_id, szn1_4) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 note4 - share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-4 content".to_string(),
        editable: true,
        deleted: false,
      },
    )?;
    save_zklink(&conn, szn1_4_id, szn1_2_share_id, uid1, None)?;

    println!("8");

    // user 1 note 5 - on share '3'.
    let (szn1_5_id, szn1_5) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 note5 - share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-5 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;
    save_zklink(&conn, szn1_5_id, szn1_3_share_id, uid1, None)?;

    // user 1 note 6 - shared w user link
    let (szn1_6_id, szn1_6) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 note6 - direct share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-6 content".to_string(),
        editable: true,
        deleted: false,
      },
    )?;
    save_zklink(&conn, szn1_6_id, unid2, uid1, None)?;

    println!("9");

    // user 1 note 7 - shared w reversed user link
    let (szn1_7_id, szn1_7) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 note7 - reversed direct share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-7 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;
    save_zklink(&conn, unid2, szn1_7_id, uid1, None)?;

    println!("10");

    // user 2 can save changes to note 4.
    match save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: Some(szn1_4.id),
        title: format!("{} - u1 note4 - rshare", basename),
        showtitle: true,
        pubid: None,
        content: "note1-4 content FROM USER 2".to_string(),
        editable: false,
        deleted: false,
      },
    ) {
      Ok(_) => (),
      Err(e) => {
        println!("error {:?}", e);
        panic!("test failed)")
      }
    };

    println!("11");

    // user 2 can't save changes to note 5.
    match save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: Some(szn1_5.id),
        title: format!("{} - u1 note5 - share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-5 content FROM USER 2".to_string(),
        editable: false,
        deleted: false,
      },
    ) {
      Ok(_) => panic!("test failed"),
      Err(_e) => (),
    }

    println!("12");

    let (szn2_1_id, szn2_1) = save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: None,
        title: format!("{} - u2 note1", basename),
        showtitle: true,
        pubid: None,
        content: "note2 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    println!("13");

    // Ok to link to share 2, because am a member.
    save_zklink(&conn, szn2_1_id, szn1_2_share_id, uid2, None)?;
    // not ok to link to share 3, because not a member.
    // should fail!
    match save_zklink(&conn, szn1_4_id, szn1_3_share_id, uid2, None) {
      Ok(_) => panic!("wat"),
      Err(_e) => (),
    };

    println!("14");

    let pn = format!("{}-not-publicid1", basename);

    // TODO test that pubid read works, since that broke in 'production'

    // note with public id, but not linked to 'public'.
    let (pubzn1_id, pubzn1) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: pn.clone(),
        showtitle: true,
        pubid: Some("not-publicid1".to_string()), // test duplicate public notes!
        // pubid: Some(format!("{}-not-publicid1", basename)),
        content: "note1 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    println!("14");

    // despite public id, shouldn't be able to read. because doesn't link to 'public'
    // println!("14.1");
    // match read_zknotepubid(&conn, None, pn.as_str()) {
    //   Ok(_) => panic!("wat"),
    //   Err(_e) => (),
    // };
    // println!("14.2");

    // println!("15");

    let (pubzn2_id, pubzn2) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 public note2", basename),
        showtitle: true,
        pubid: Some("publicid2".to_string()),
        content: "note2 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;
    println!("15.1");
    save_zklink(&conn, pubzn2_id, publicid, uid1, None)?;
    println!("15.2");
    // should be able to read because links to 'public'.
    read_zknotepubid(&conn, None, "publicid2")?;
    println!("15.3");

    // should be able to save changes to a share note without error.
    save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: Some(szn1_2_share.id),
        title: format!("{} - u1 note2 - share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-2 content changed".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    println!("16");

    // TODO test that 'public' is not treated as a share.

    // test notes linked with user BY CREATOR are editable.
    let (szn1_6_id, szn1_6) = save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 note6 - direct share", basename),
        showtitle: true,
        pubid: None,
        content: "note1-6 content changed by user2".to_string(),
        editable: true,
        deleted: false,
      },
    )?;

    println!("16");
    Ok(((pubzn1_id, pubzn1), (pubzn2_id, pubzn2)))
  }

  // ---------------------------------------------------------------
  // testing
  // ---------------------------------------------------------------
  async fn err_test() -> Result<(), Box<dyn Error>> {
    let dbp_client = Path::new("client.db");
    let dbp_server = Path::new("server.db");

    let mut cb = zknotes_callbacks();

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
            println!("error removing test.db: {}", e);
          }
        }
        dbinit(dbp_server, None)?;
        let server_conn = connection_open(dbp_server)?;
        setup_db(&server_conn, &mut cb, None, "server")?;
        fs::copy(dbp_server, Path::new("server.db.saved"))?;
      }
    }
    let server_conn = connection_open(dbp_server)?;

    let suser1 = user_id(&server_conn, "server-user1")?;
    let suser1ld = orgauth::dbfun::login_data(&server_conn, suser1)?;

    match fs::copy(Path::new("client.db.saved"), dbp_client) {
      Ok(_) => (), // already have a saved initted db.
      Err(_) => {
        // init and save in *.saved
        match fs::remove_file(dbp_client) {
          Ok(_) => (),
          Err(e) => {
            println!("error removing test.db: {}", e);
          }
        }
        dbinit(dbp_client, None)?;
        let client_conn = connection_open(dbp_client)?;
        // passing in the UUID for "server-user1".
        // We'll sync client-user1 and server-user1.
        setup_db(&client_conn, &mut cb, Some(suser1ld.uuid), "client")?;
        fs::copy(dbp_client, Path::new("client.db.saved"))?;
      }
    }
    let client_conn = connection_open(dbp_client)?;

    println!("1");

    println!("2");

    // let publicid = note_id(&conn, "system", "public")?;
    // let shareid = note_id(&conn, "system", "share")?;
    // let _searchid = note_id(&conn, "system", "search")?;

    println!("2.5");

    // let _unid1 = user_note_id(&conn, uid1)?;
    // let unid2 = user_note_id(&conn, uid2)?;

    // ---------------------------------------------------------------
    // test searches.
    // ---------------------------------------------------------------

    // use user2, since user2 has limited visibility to user1 notes.
    let cuser1 = user_id(&client_conn, "client-user1")?;
    let cuser2 = user_id(&client_conn, "client-user2")?;
    let suser2 = user_id(&server_conn, "server-user2")?;
    let caconn = Arc::new(client_conn);
    let saconn = Arc::new(server_conn);

    // initial sync, testing duplicate public ids.
    {
      let server_stream = sync_stream(saconn.clone(), suser2, None, None, None, None, &mut cb);

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
          if e.to_string() == "note exists with duplicate public id: publicid2" {
            ()
          } else {
            Err(e)?
          }
        }
      }

      ctr.rollback()?;
    }
    println!("1");

    let cpubid: i64 = caconn.query_row(
      "select id from zknote where pubid = ?1",
      params!["publicid2"],
      |row| Ok(row.get(0)?),
    )?;

    // let cpub1 = read_zknote_i64(&caconn, Some(cuser2), cpubid)?;
    let cpub2 = read_zknotepubid(&caconn, None, "publicid2")?;

    // rename public ids.
    // let (cpc1_id, cpc1) = save_zknote(
    //   &caconn,
    //   cuser2,
    //   &SaveZkNote {
    //     id: Some(cpub1.id),
    //     title: "public client 1".to_string(),
    //     showtitle: true,
    //     pubid: Some("not-publicid1-client".to_string()), // test duplicate public notes!
    //     content: "public note1 content".to_string(),
    //     editable: false,
    //     deleted: false,
    //   },
    // )?;

    println!("2");
    // assert!(cpub1.id == cpc1.id);

    // alter as user1
    let (spc2_id, spc2) = save_zknote(
      &caconn,
      cuser1,
      &SaveZkNote {
        id: Some(cpub2.id),
        title: "public client 2".to_string(),
        showtitle: true,
        pubid: Some("publicid2-client".to_string()), // test duplicate public notes!
        content: "public note2 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;
    assert!(cpub2.id == spc2.id);

    {
      let server_stream = sync_stream(saconn.clone(), suser2, None, None, None, None, &mut cb);
      // -----------------------------------
      // Writing the stream to a file
      // -----------------------------------
      use futures::executor::block_on;
      use futures::{future, future::FutureExt, stream, stream::StreamExt};
      use tokio::io::AsyncWriteExt;
      let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("server-stream.txt")
        .await
        .unwrap();

      server_stream
        .for_each(|item| match item {
          Ok(bytes) => {
            block_on(file.write(&bytes));
            future::ready(())
          }
          Err(e) => future::ready(()),
        })
        .await;
    }
    {
      let client_stream = sync_stream(
        caconn.clone(),
        cuser2,
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
      use futures::{future, future::FutureExt, stream, stream::StreamExt};
      use tokio::io::AsyncWriteExt;
      let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("client_stream.txt")
        .await
        .unwrap();

      client_stream
        .for_each(|item| match item {
          Ok(bytes) => {
            block_on(file.write(&bytes));
            future::ready(())
          }
          Err(e) => future::ready(()),
        })
        .await;
    }
    // ------------------------------------------------------------
    // sync from server to client.
    // ------------------------------------------------------------
    let server_stream = sync_stream(saconn.clone(), suser2, None, None, None, None, &mut cb);

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
      cuser2,
      Some(ttn.notetemp),
      Some(ttn.linktemp),
      Some(ttn.archivelinktemp),
      None,
      &mut cb,
    );

    let cs = client_stream.map_err(convert_err);
    pin_mut!(cs);
    let mut cbr = StreamReader::new(cs);

    sync_from_stream(&saconn, None, None, None, &mut cb, &mut cbr).await?;

    // ------------------------------------------------------------
    // post-sync tests.
    // ------------------------------------------------------------

    println!("3");
    // read_zknotepubid(&caconn, None, "not-publicid1")?;
    read_zknotepubid(&caconn, None, "publicid2")?;
    println!("3.5");
    // read_zknotepubid(&caconn, Some(cuser1), "not-publicid1")?;
    read_zknotepubid(&caconn, Some(cuser2), "publicid2")?;
    println!("4");
    // let cnot_publicid1_client = read_zknotepubid(&caconn, Some(cuser1), "not-publicid1-client")?;
    let cpublicid2_client = read_zknotepubid(&caconn, Some(cuser2), "publicid2-client")?;

    println!("5");
    // client public notes on server.
    // let snot_publicid1_client = read_zknotepubid(&saconn, Some(suser1), "not-publicid1-client")?;
    let spublicid2_client = read_zknotepubid(&saconn, Some(suser2), "publicid2-client")?;

    // ids are the same.
    // assert!(cnot_publicid1_client.id == snot_publicid1_client.id);
    assert!(spublicid2_client.id == cpublicid2_client.id);

    println!("6");

    {
      println!("cpub2.id: {}", cpub2.id);
      // archives present for revised notes.
      let cpcarchs = read_zknotearchives(
        &caconn,
        cuser2,
        &GetZkNoteArchives {
          zknote: cpub2.id,
          offset: 0,
          limit: None,
        },
      )?;

      println!("7.2");
      let spcarchs = read_zknotearchives(
        &saconn,
        suser2,
        &GetZkNoteArchives {
          zknote: cpub2.id,
          offset: 0,
          limit: None,
        },
      )?;

      assert!(cpcarchs.len() > 0);

      assert!(cpcarchs == spcarchs);
    }

    assert!(false);

    // Verify notes not visible to user1 are not on server.

    // TESTING:
    // user ids on client and server don't match.
    //   should sync fail if user ids don't match?  do user ids match on remote users now?
    // phantom user stuff (like what?)
    //

    // szn1_1_share
    // szn1_2_share
    // szn1_3_share
    // szn1_4
    // szn1_5
    // szn1_6
    // szn1_7
    // szn2_1
    // pubzn1
    // pubzn2
    // szn1_6

    // TEST: give user access to a share, then resync.  do they get the old notes?
    // implement by checking for new links to shares.  when found, sync the whole share.
    // in stream, first find new links to shares and new links from notes to user, and new links to public.
    // or, query for the same thing??  mod date after last sync, OR links to new share, OR new links to new user.

    Ok(())
  }
}
