#[cfg(test)]
mod tests {
  use crate::search::*;
  use crate::sqldata::*;
  use orgauth::data::RegistrationData;
  use orgauth::dbfun::{new_user, user_id};
  use std::error::Error;
  use std::fs;
  use std::path::Path;
  use zkprotocol::content::SaveZkNote;
  use zkprotocol::search::*;

  // Note this useful idiom: importing names from outer (for mod tests) scope.
  // use super::*;

  #[test]
  fn test_sharing() {
    let res = match err_test() {
      Ok(()) => true,
      Err(e) => {
        println!("error {:?}", e);
        false
      }
    };
    assert_eq!(res, true);
  }

  fn err_test() -> Result<(), Box<dyn Error>> {
    let dbp = Path::new("test.db");
    match fs::remove_file(dbp) {
      Ok(_) => (),
      Err(e) => {
        println!("error removing test.db: {}", e);
      }
    }
    let mut cb = zknotes_callbacks();

    let filesdir = Path::new("");

    dbinit(dbp, None)?;

    let conn = connection_open(dbp)?;

    let server = local_server_id(&conn)?;

    println!("1");

    let uid1 = new_user(
      &conn,
      &RegistrationData {
        uid: "user1".to_string(),
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
        uid: "user2".to_string(),
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

    println!("2");

    let publicid = note_id(&conn, "system", "public")?;
    let shareid = note_id(&conn, "system", "share")?;
    let _searchid = note_id(&conn, "system", "search")?;

    println!("2.5");

    let _unid1 = user_note_id(&conn, uid1)?;
    let unid2 = user_note_id(&conn, uid2)?;

    println!("3");

    let _szn1_1 = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note1".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;

    // user 1 note 2 - share
    let (szn1_2_share_id, szn1_2_share) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note2 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-2 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;

    save_zklink(&conn, szn1_2_share_id, shareid, uid1, None)?;

    println!("4");

    // user 1 note 3 - share
    let (szn1_3_share_id, _szn1_3_share) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note3 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-3 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;

    save_zklink(&conn, szn1_3_share_id, shareid, uid1, None)?;

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
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note4 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-4 content".to_string(),
        editable: true,
        deleted: false,
        what: None,
      },
    )?;
    save_zklink(&conn, szn1_4_id, szn1_2_share_id, uid1, None)?;

    println!("8");

    // user 1 note 5 - on share '3'.
    let (szn1_5_id, szn1_5) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note5 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-5 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;
    save_zklink(&conn, szn1_5_id, szn1_3_share_id, uid1, None)?;

    // user 1 note 6 - shared w user link
    let (szn1_6_id, _szn1_6) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note6 - direct share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-6 content".to_string(),
        editable: true,
        deleted: false,
        what: None,
      },
    )?;
    save_zklink(&conn, szn1_6_id, unid2, uid1, None)?;

    println!("9");

    // user 1 note 7 - shared w reversed user link
    let (szn1_7_id, _szn1_7) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note7 - reversed direct share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-7 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;
    save_zklink(&conn, unid2, szn1_7_id, uid1, None)?;

    println!("10");

    // user 2 can save changes to note 4.
    match save_zknote(
      &conn,
      uid2,
      &server,
      &SaveZkNote {
        id: Some(szn1_4.id),
        title: "u1 note4 - rshare".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-4 content FROM USER 2".to_string(),
        editable: false,
        deleted: false,
        what: None,
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
      &server,
      &SaveZkNote {
        id: Some(szn1_5.id),
        title: "u1 note5 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-5 content FROM USER 2".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    ) {
      Ok(_) => panic!("test failed"),
      Err(_e) => (),
    }

    println!("12");

    let (szn2_1_id, _szn2_1) = save_zknote(
      &conn,
      uid2,
      &server,
      &SaveZkNote {
        id: None,
        title: "u2 note1".to_string(),
        showtitle: true,
        pubid: None,
        content: "note2 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
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

    // TODO test that pubid read works, since that broke in 'production'
    let _pubzn1 = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 public note1".to_string(),
        showtitle: true,
        pubid: Some("publicid1".to_string()),
        content: "note1 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;

    println!("14");

    // despite public id, shouldn't be able to read. because doesn't link to 'public'
    match read_zknotepubid(&conn, filesdir, None, "publicid1") {
      Ok(_) => panic!("wat"),
      Err(_e) => (),
    };

    println!("15");

    let (pubzn2_id, _pubzn2) = save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 public note2".to_string(),
        showtitle: true,
        pubid: Some("publicid2".to_string()),
        content: "note1 content".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;
    println!("15.1");
    save_zklink(&conn, pubzn2_id, publicid, uid1, None)?;
    println!("15.2");
    // should be able to read because links to 'public'.
    read_zknotepubid(&conn, filesdir, None, "publicid2")?;
    println!("15.3");

    // should be able to save changes to a share note without error.
    save_zknote(
      &conn,
      uid1,
      &server,
      &SaveZkNote {
        id: Some(szn1_2_share.id),
        title: "u1 note2 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-2 content changed".to_string(),
        editable: false,
        deleted: false,
        what: None,
      },
    )?;

    println!("16");

    // TODO test that 'public' is not treated as a share.

    // test notes linked with user BY CREATOR are editable.
    let (_szn1_6_id, _szn1_6) = save_zknote(
      &conn,
      uid2,
      &server,
      &SaveZkNote {
        id: None,
        title: "u1 note6 - direct share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-6 content changed by user2".to_string(),
        editable: true,
        deleted: false,
        what: None,
      },
    )?;

    println!("16");

    // ---------------------------------------------------------------
    // test searches.
    // ---------------------------------------------------------------

    // user A can see their own public note.
    let u1pubnote2_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 public note2".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid1, &u1pubnote2_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          println!("length was: {}", zklr.notes.len());
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }

    println!("17");

    // u2 can see the note too.
    match search_zknotes(&conn, filesdir, uid2, &u1pubnote2_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          println!("length was: {}", zklr.notes.len());
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }

    let u1note1_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note1".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    // u2 can't see u1's private note..
    match search_zknotes(&conn, filesdir, uid2, &u1note1_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          // not supposed to see it!
          panic!("test failed")
        } else {
          ()
        }
      }
      _ => panic!("test failed"),
    }
    println!("18");

    // u1 can see their own private note..
    match search_zknotes(&conn, filesdir, uid1, &u1note1_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }

    println!("19");
    let u1note6_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note6 - direct share".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    // u2 can see a note shared directly with them.
    match search_zknotes(&conn, filesdir, uid2, &u1note6_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }

    println!("20");
    let u1note7_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note7 - reversed direct share".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    // u2 can see a note shared directly with them, reversed link.
    match search_zknotes(&conn, filesdir, uid2, &u1note7_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }
    println!("21");

    // u2 can see a note on a share they're a member of.
    let u1note4_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note4 - rshare".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid2, &u1note4_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }
    println!("22");

    // u2 can't see a note on a share they're not a member of.
    let u1note5_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note5 - share".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid2, &u1note5_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          // not supposed to see it!
          panic!("test failed")
        } else {
          ()
        }
      }
      _ => panic!("test failed"),
    }
    println!("23");

    // u2 can't see note4 archive note.
    let u1note4_archive_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch],
        term: "u1 note4 - share".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "u2nran test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid2, &u1note4_archive_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          // not supposed to see it!
          println!("u1note4_srearch {:?}", zklr);
          panic!("test failed")
        } else {
          ()
        }
      }
      _ => panic!("test failed"),
    }
    println!("24");

    // system user can see the archive note
    let systemid = user_id(&conn, "system")?;
    match search_zknotes(&conn, filesdir, systemid, &u1note4_archive_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          println!("u1note4_archive_search w systemid{:?}", zklr);
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }

    // can save changes to a share note, without error.

    // TODO test search modifiers.
    //	 ExactMatch,
    //	 Tag,
    //	 Note,
    //	 User,

    //	 ExactMatch with bad case fails.
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch],
        term: "u1 Public note2".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid1, &u1pubnote2_exact_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 1 {
          panic!("test failed")
        } else {
          ()
        }
      }
      _ => panic!("test failed"),
    }
    println!("25");

    // should be 4 notes tagged with et'user' - 3 users and 1 'system'.
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch, SearchMod::Tag],
        term: "user".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid1, &u1pubnote2_exact_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 4 {
          ()
        } else {
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }
    println!("26");

    // should be 9 notes for 'user1'
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch, SearchMod::Tag, SearchMod::User],
        term: "user1".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid1, &u1pubnote2_exact_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 9 {
          ()
        } else {
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }
    println!("27");

    // should be 1 notes for 'ote1-4'
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: vec![TagSearch::SearchTerm {
        mods: vec![SearchMod::Note],
        term: "ote1-4".to_string(),
      }],
      offset: 0,
      limit: None,
      what: "test".to_string(),
      resulttype: ResultType::RtListNote,
      ordering: None,
      archives: false,
      deleted: false,
    };

    match search_zknotes(&conn, filesdir, uid1, &u1pubnote2_exact_search)? {
      SearchResult::SrListNote(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          panic!("test failed")
        }
      }
      _ => panic!("test failed"),
    }
    //
    Ok(())
  }
}
