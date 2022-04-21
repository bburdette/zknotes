#[cfg(test)]
mod tests {
  use zkprotocol::search::*;
  //   {
  //   AndOr, SearchMod, TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult,
  // };
  use crate::search::*;
  use crate::sqldata::*;
  use either::Either;
  use std::error::Error;
  use std::fs;
  use std::path::Path;
  use zkprotocol::content::{
    Direction, GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkLink, SaveZkNote,
    SavedZkNote, ZkLink, ZkNote, ZkNoteEdit,
  };

  // Note this useful idiom: importing names from outer (for mod tests) scope.
  use super::*;

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

    dbinit(dbp, 1000000)?;

    let uid1 = new_user(
      dbp,
      "user1".to_string(),
      "".to_string(),
      "".to_string(),
      "".to_string(),
      "".to_string(),
    )?;
    let uid2 = new_user(
      dbp,
      "user2".to_string(),
      "".to_string(),
      "".to_string(),
      "".to_string(),
      "".to_string(),
    )?;

    let conn = connection_open(dbp)?;

    let publicid = note_id(&conn, "system", "public")?;
    let shareid = note_id(&conn, "system", "share")?;
    let searchid = note_id(&conn, "system", "search")?;

    let unid1 = user_note_id(&conn, uid1)?;

    let unid2 = user_note_id(&conn, uid2)?;

    let szn1_1 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note1".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1 content".to_string(),
        editable: false,
      },
    )?;

    // user 1 note 2 - share
    let szn1_2_share = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note2 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-2 content".to_string(),
        editable: false,
      },
    )?;

    save_zklink(&conn, szn1_2_share.id, shareid, uid1, None)?;

    // user 1 note 3 - share
    let szn1_3_share = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note3 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-3 content".to_string(),
        editable: false,
      },
    )?;

    save_zklink(&conn, szn1_3_share.id, shareid, uid1, None)?;

    // user1 adds user 2 to user 1 share '2'.
    save_zklink(&conn, unid2, szn1_2_share.id, uid1, None)?;

    // user2 adds user 2 to user1 share '3'.  should fail
    match save_zklink(&conn, unid2, szn1_3_share.id, uid2, None) {
      Ok(_) => assert_eq!(2, 4),
      // Ok(_) => (),
      Err(_) => (),
    };

    // user 1 note 4 - on share '2'.
    let szn1_4 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note4 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-4 content".to_string(),
        editable: false,
      },
    )?;
    save_zklink(&conn, szn1_4.id, szn1_2_share.id, uid1, None)?;

    // user 1 note 5 - on share '3'.
    let szn1_5 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note5 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-5 content".to_string(),
        editable: false,
      },
    )?;
    save_zklink(&conn, szn1_5.id, szn1_3_share.id, uid1, None)?;

    // user 1 note 6 - shared w user link
    let szn1_6 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note6 - direct share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-6 content".to_string(),
        editable: false,
      },
    )?;
    save_zklink(&conn, szn1_6.id, unid2, uid1, None)?;

    // user 1 note 7 - shared w reversed user link
    let szn1_7 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 note7 - reversed direct share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-7 content".to_string(),
        editable: false,
      },
    )?;
    save_zklink(&conn, unid2, szn1_7.id, uid1, None)?;

    // user 2 can save changes to note 4.
    save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: Some(szn1_4.id),
        title: "u1 note4 - rshare".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-4 content FROM USER 2".to_string(),
        editable: false,
      },
    )?;

    // user 2 can't save changes to note 5.
    match save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: Some(szn1_5.id),
        title: "u1 note5 - share".to_string(),
        showtitle: true,
        pubid: None,
        content: "note1-5 content FROM USER 2".to_string(),
        editable: false,
      },
    ) {
      Ok(_) => assert_eq!(2, 4),
      Err(_e) => (),
    }

    let szn2_1 = save_zknote(
      &conn,
      uid2,
      &SaveZkNote {
        id: None,
        title: "u2 note1".to_string(),
        showtitle: true,
        pubid: None,
        content: "note2 content".to_string(),
        editable: false,
      },
    )?;
    // Ok to link to share 2, because am a member.
    save_zklink(&conn, szn2_1.id, szn1_2_share.id, uid2, None)?;
    // not ok to link to share 3, because not a member.
    // should fail!
    match save_zklink(&conn, szn1_4.id, szn1_3_share.id, uid2, None) {
      Ok(_) => panic!("wat"),
      Err(_e) => (),
    };

    // TODO test that pubid read works, since that broke in 'production'
    let pubzn1 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 public note1".to_string(),
        showtitle: true,
        pubid: Some("publicid1".to_string()),
        content: "note1 content".to_string(),
        editable: false,
      },
    )?;
    // despite public id, shouldn't be able to read. because doesn't link to 'public'
    match read_zknotepubid(&conn, None, "publicid1") {
      Ok(_) => panic!("wat"),
      Err(_e) => (),
    };

    let pubzn2 = save_zknote(
      &conn,
      uid1,
      &SaveZkNote {
        id: None,
        title: "u1 public note2".to_string(),
        showtitle: true,
        pubid: Some("publicid2".to_string()),
        content: "note1 content".to_string(),
        editable: false,
      },
    )?;
    save_zklink(&conn, pubzn2.id, publicid, uid1, None)?;
    // should be able to read because links to 'public'.
    read_zknotepubid(&conn, None, "publicid2")?;

    // TODO test that 'public' is not treated as a share.

    // TODO test notes linked with user BY CREATOR are editable/visible.

    // ---------------------------------------------------------------
    // test searches.
    // ---------------------------------------------------------------

    // user A can their own public note.
    let u1pubnote2_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 public note2".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid1, &u1pubnote2_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          println!("lenth was: {}", zklr.notes.len());
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // u2 can see the note too.
    match search_zknotes(&conn, uid2, &u1pubnote2_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          println!("lenth was: {}", zklr.notes.len());
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    let u1note1_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note1".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    // u2 can't see u1's private note..
    match search_zknotes(&conn, uid2, &u1note1_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          // not supposed to see it!
          assert_eq!(2, 4)
        } else {
          ()
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // u1 can see their own private note..
    match search_zknotes(&conn, uid1, &u1note1_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    let u1note6_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note6 - direct share".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    // u2 can see a note shared directly with them.
    match search_zknotes(&conn, uid2, &u1note6_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // supposed to see it!
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    let u1note7_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note7 - reversed direct share".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    // u2 can see a note shared directly with them, reversed link.
    match search_zknotes(&conn, uid2, &u1note7_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // u2 can see a note on a share they're a member of.
    let u1note4_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note4 - share".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid2, &u1note4_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          ()
        } else {
          // not supposed to see it!
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // u2 can't see a note on a share they're not a member of.
    let u1note5_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: Vec::new(),
        term: "u1 note5 - share".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid2, &u1note5_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() > 0 {
          // not supposed to see it!
          assert_eq!(2, 4)
        } else {
          ()
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // TODO test search modifiers.
    //	 ExactMatch,
    //	 Tag,
    //	 Note,
    //	 User,

    //	 ExactMatch with bad case fails.
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch],
        term: "u1 Public note2".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid1, &u1pubnote2_exact_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 1 {
          assert_eq!(2, 4)
        } else {
          ()
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // should be 4 notes tagged with et'user' - 3 users and 1 'system'.
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch, SearchMod::Tag],
        term: "user".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid1, &u1pubnote2_exact_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 4 {
          ()
        } else {
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }

    // should be 9 notes for 'user1'
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: vec![SearchMod::ExactMatch, SearchMod::Tag, SearchMod::User],
        term: "user1".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid1, &u1pubnote2_exact_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 9 {
          ()
        } else {
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }
    // should be 1 notes for 'ote1-4'
    let u1pubnote2_exact_search = ZkNoteSearch {
      tagsearch: TagSearch::SearchTerm {
        mods: vec![SearchMod::Note],
        term: "ote1-4".to_string(),
      },
      offset: 0,
      limit: None,
      what: "test".to_string(),
      list: true,
    };

    match search_zknotes(&conn, uid1, &u1pubnote2_exact_search)? {
      Either::Left(zklr) => {
        if zklr.notes.len() == 1 {
          ()
        } else {
          assert_eq!(2, 4)
        }
      }
      Either::Right(zknr) => assert_eq!(2, 4),
    }
    //
    Ok(())
  }
}
