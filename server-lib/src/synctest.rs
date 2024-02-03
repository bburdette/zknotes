#[cfg(test)]
mod tests {
  use crate::interfaces::*;
  use crate::search::*;
  use crate::sqldata::*;
  use crate::sync::*;
  use either::Either;
  use futures::executor::block_on;
  use futures_util::TryStreamExt;
  use orgauth::data::RegistrationData;
  use orgauth::dbfun::{new_user, user_id};
  use orgauth::endpoints::Callbacks;
  use rusqlite::Connection;
  use std::error::Error;
  use std::fs;
  use std::path::Path;
  use std::sync::Arc;
  use tokio_util::io::StreamReader;
  use zkprotocol::content::SaveZkNote;
  use zkprotocol::content::SavedZkNote;
  use zkprotocol::search::*;

  // Note this useful idiom: importing names from outer (for mod tests) scope.
  // use super::*;

  #[test]
  fn test_sync() {
    let res = match block_on(err_test()) {
      Ok(()) => true,
      Err(e) => {
        println!("error {:?}", e);
        false
      }
    };
    assert_eq!(res, true);
  }

  fn setup_db(
    conn: &Connection,
    cb: &mut Callbacks,
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
      None,
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

    let _szn1_1 = save_zknote(
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

    let pn = format!("{}-publicid1", basename);

    // TODO test that pubid read works, since that broke in 'production'
    let (pubzn1_id, pubzn1) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: pn.clone(),
        showtitle: true,
        pubid: Some("publicid1".to_string()), // test duplicate public notes!
        // pubid: Some(format!("{}-publicid1", basename)),
        content: "note1 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    println!("14");

    // despite public id, shouldn't be able to read. because doesn't link to 'public'
    println!("14.1");
    match read_zknotepubid(&conn, None, pn.as_str()) {
      Ok(_) => panic!("wat"),
      Err(_e) => (),
    };
    println!("14.2");

    println!("15");

    let (pubzn2_id, pubzn2) = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: format!("{} - u1 public note2", basename),
        showtitle: true,
        pubid: Some("publicid2".to_string()),
        content: "note1 content".to_string(),
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

  async fn err_test() -> Result<(), Box<dyn Error>> {
    let dbp_client = Path::new("client.db");
    match fs::remove_file(dbp_client) {
      Ok(_) => (),
      Err(e) => {
        println!("error removing test.db: {}", e);
      }
    }
    let dbp_server = Path::new("server.db");
    match fs::remove_file(dbp_server) {
      Ok(_) => (),
      Err(e) => {
        println!("error removing test.db: {}", e);
      }
    }

    dbinit(dbp_client, None)?;

    dbinit(dbp_server, None)?;

    let mut cb = zknotes_callbacks();

    let client_conn = connection_open(dbp_client)?;
    let server_conn = connection_open(dbp_server)?;

    let ((cpubid1, cpub1), (cpubid2, cpub2)) = setup_db(&client_conn, &mut cb, "client")?;
    setup_db(&server_conn, &mut cb, "server")?;

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

    let cuser1 = user_id(&client_conn, "client-user1")?;
    let suser1 = user_id(&server_conn, "server-user1")?;
    let caconn = Arc::new(client_conn);
    let saconn = Arc::new(server_conn);

    // initial sync, testing duplicate public ids.
    {
      let server_stream = sync_stream(saconn.clone(), suser1, None, None, None, None, &mut cb);

      let ctr = caconn.unchecked_transaction()?;

      let ttn = temp_tables(&caconn)?;
      fn convert_err(err: Box<dyn Error>) -> std::io::Error {
        println!("convert_err {:?}", err);
        todo!()
      }
      // use futures::executor::block_on;
      use futures_util::pin_mut;

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
          if e.to_string() == "note exists with duplicate public id: publicid1" {
            ()
          } else {
            Err(e)?
          }
        }
      }

      ctr.rollback()?;
    }

    // rename public ids.
    save_zknote(
      &caconn,
      cuser1,
      &SaveZkNote {
        id: Some(cpub1.id),
        title: "public client 1".to_string(),
        showtitle: true,
        pubid: Some("publicid1-client".to_string()), // test duplicate public notes!
        content: "public note1 content".to_string(),
        editable: false,
        deleted: false,
      },
    )?;

    save_zknote(
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

    let server_stream = sync_stream(saconn, suser1, None, None, None, None, &mut cb);

    // let ctr = caconn.unchecked_transaction()?;

    let ttn = temp_tables(&caconn)?;
    fn convert_err(err: Box<dyn Error>) -> std::io::Error {
      println!("convert_err {:?}", err);
      todo!()
    }
    // use futures::executor::block_on;
    use futures_util::pin_mut;

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

    Ok(())
  }
}
