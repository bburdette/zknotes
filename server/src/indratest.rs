// use std::convert::TryInto;
// use std::error::Error;
use crate::errors;
use indradb::Datastore;
use indradb::{
  Edge, EdgeKey, EdgeProperty, EdgeQueryExt, SledDatastore, SledTransaction, Transaction, Type,
  Vertex, VertexQueryExt,
};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::time::Duration;
use crate::icontent::{
  Direction, GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkLink, SaveZkNote,
  SavedZkNote, User, UserId, ZkLink, ZkListNote, ZkNote, ZkNoteEdit,
};
use crate::importdb::import_db;
use crate::indra::{
  checknote, delete_zklink, delete_zknote, get_systemvs, is_note_accessible, is_note_mine,
  is_note_public, is_note_shared, link_exists, login_data, mkpropquery, new_user, note_owner,
  read_user, read_zklinks, read_zklistnote, read_zknote, read_zknoteedit, save_savezklinks,
  save_user, save_zklink, save_zknote, search_zknotes, tagsearch,
};
use crate::indra_util::{find_all_q, find_first_q, getoptedgeprop, getoptprop, getprop};
use crate::isearch::{AndOr, SearchMod, TagSearch, ZkNoteSearch, ZkNoteSearchResult};
use crate::user::ZkDatabase;
use crate::util::now;
use std::time::SystemTime;
use uuid::Uuid;
use zkprotocol::content as C; // as U;

#[cfg(test)]
mod test {
  use super::*;
  use std::fs;

  pub fn test_db() -> Result<(), errors::Error> {
    println!("test-db starrt");
    let path = Path::new("indra-test");
    // delete the db if its there.
    fs::remove_dir_all(path);

    {
      // compression factor of 5 (default)
      let sc = indradb::SledConfig::with_compression(None);

      // let ids = sc.open(dbpath.as_os_str().to_str().ok_or(bail!("blah"))?)?;
      // let ids = sc.open(path)?;

      import_db(
        &ZkDatabase {
          notes: Vec::new(),
          links: Vec::new(),
          users: Vec::new(),
        },
        path,
      )?;

      let ids = sc.open(path)?;
      let itr = ids.transaction()?;

      let svs = get_systemvs(&itr)?;

      // println!("read_user: {:?}", read_user(&itr, "ben".to_string())?);

      // make test users.
      let tuid1 = match find_first_q(
        &itr,
        mkpropquery("user".to_string(), "name".to_string()),
        |x| x.value == "test",
      )? {
        Some(vp) => UserId(vp.id),
        None => new_user(
          &itr,
          &svs.public,
          "test".to_string(),
          "".to_string(),
          "".to_string(),
          "test@test.com".to_string(),
          None,
        )?,
      };

      let tuid2 = match find_first_q(
        &itr,
        mkpropquery("user".to_string(), "name".to_string()),
        |x| x.value == "test2",
      )? {
        Some(vp) => UserId(vp.id),
        None => new_user(
          &itr,
          &svs.public,
          "test2".to_string(),
          "".to_string(),
          "".to_string(),
          "test2@test.com".to_string(),
          None,
        )?,
      };

      let szn1 = SaveZkNote {
        id: None,
        title: "test title 1".to_string(),
        pubid: None,
        content: "test content 1".to_string(),
      };
      let sid1 = save_zknote(&itr, tuid1, &szn1)?;

      let szn2 = SaveZkNote {
        id: None,
        title: "test title 2".to_string(),
        pubid: None,
        content: "test content 2".to_string(),
      };
      let sid2 = save_zknote(&itr, tuid1, &szn2)?;

      let szn3 = SaveZkNote {
        id: None,
        title: "test title 3".to_string(),
        pubid: None,
        content: "test content 3".to_string(),
      };
      let sid3 = save_zknote(&itr, tuid2, &szn3)?;

      let szn4 = SaveZkNote {
        id: None,
        title: "test title 4".to_string(),
        pubid: Some("publicccc note".to_string()),
        content: "test content 4".to_string(),
      };
      let sid4 = save_zknote(&itr, tuid2, &szn4)?;
      save_zklink(&itr, &sid4.id, &svs.public, &tuid2, &None)?;

      assert_eq!(true, is_note_public(&itr, &svs, sid4.id)?);
      assert_eq!(false, is_note_public(&itr, &svs, sid3.id)?);
      assert_eq!(false, is_note_public(&itr, &svs, sid2.id)?);
      assert_eq!(false, is_note_public(&itr, &svs, sid1.id)?);

      assert_eq!(true, is_note_mine(&itr, sid1.id, tuid1)?);
      assert_eq!(false, is_note_mine(&itr, sid3.id, tuid1)?);
      assert_eq!(false, is_note_mine(&itr, sid1.id, tuid2)?);
      assert_eq!(false, is_note_mine(&itr, sid2.id, tuid2)?);
      assert_eq!(true, is_note_mine(&itr, sid4.id, tuid2)?);

      // make a share note.
      let szn_share = SaveZkNote {
        id: None,
        title: "test title _share".to_string(),
        pubid: Some("share note".to_string()),
        content: "test content _share".to_string(),
      };
      let sid_share = save_zknote(&itr, tuid2, &szn_share)?;
      save_zklink(&itr, &sid_share.id, &svs.share, &tuid2, &None)?;
      // link szn3 to it.
      save_zklink(&itr, &sid3.id, &sid_share.id, &tuid2, &None)?;

      // hook user tuid1 with the share.
      save_zklink(&itr, &tuid1.0, &sid_share.id, &tuid2, &None)?;

      // now user 1 should be able to see the note in share.
      assert_eq!(false, is_note_shared(&itr, &svs, tuid1, sid4.id)?);
      // user 1 should not be able to see the unshared note.
      assert_eq!(true, is_note_shared(&itr, &svs, tuid1, sid3.id)?);

      assert_eq!(true, is_note_accessible(&itr, &svs, Some(tuid1), sid1.id)?);
      assert_eq!(true, is_note_accessible(&itr, &svs, Some(tuid1), sid3.id)?);
      assert_eq!(false, is_note_accessible(&itr, &svs, None, sid3.id)?);
      assert_eq!(true, is_note_accessible(&itr, &svs, None, sid4.id)?);
      assert_eq!(true, is_note_accessible(&itr, &svs, Some(tuid1), sid4.id)?);
      assert_eq!(false, is_note_accessible(&itr, &svs, Some(tuid2), sid1.id)?);
      assert_eq!(false, is_note_accessible(&itr, &svs, Some(tuid2), sid2.id)?);

      save_zklink(&itr, &sid1.id, &sid2.id, &tuid1, &None)?;
      let zklinks = read_zklinks(&itr, &svs, Some(tuid1), sid1.id)?;
      // println!("zklinkes: {:?}", zklinks);

      let zkn = read_zknote(&itr, &svs, None, sid1.id)?;
      // println!("read_zknote {}", serde_json::to_string_pretty(&zkn)?);

      let zkne1 = read_zknoteedit(&itr, tuid1, &GetZkNoteEdit { zknote: sid1.id })?;
      // println!("read_zknote {}", serde_json::to_string_pretty(&zkne1)?);

      let zkne2 = read_zknoteedit(&itr, tuid2, &GetZkNoteEdit { zknote: sid2.id })?;
      // println!("read_zknote {}", serde_json::to_string_pretty(&zkne2)?);

      let mut tcache = HashMap::new();

      assert_eq!(
        true,
        checknote(
          &itr,
          sid4.id,
          &TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "4".to_string()
          },
          &tuid1,
          &mut tcache,
        )?
      );
      assert_eq!(
        false,
        checknote(
          &itr,
          sid4.id,
          &TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "5".to_string()
          },
          &tuid1,
          &mut tcache,
        )?
      );

      let zklns = search_zknotes(
        &itr,
        &svs,
        tuid2,
        &ZkNoteSearch {
          tagsearch: TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "test".to_string(),
          },
          offset: None,
          limit: None,
        },
      )?;

      println!("{:?}", zklns);

      assert_ne!(zklns.notes.len(), 0);

      let zklnsNOT = search_zknotes(
        &itr,
        &svs,
        tuid2,
        &ZkNoteSearch {
          tagsearch: TagSearch::Not {
            ts: Box::new(TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "test".to_string(),
            }),
          },
          offset: None,
          limit: None,
        },
      )?;

      assert_eq!(zklnsNOT.notes.len(), 0);

      println!("----------------search AND test -------------------------");

      let zklnsAND = search_zknotes(
        &itr,
        &svs,
        tuid2,
        &ZkNoteSearch {
          tagsearch: TagSearch::Boolex {
            ts1: Box::new(TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "test".to_string(),
            }),
            ao: AndOr::And,
            ts2: Box::new(TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "4".to_string(),
            }),
          },
          offset: None,
          limit: None,
        },
      )?;

      assert_eq!(zklnsAND.notes.len(), 1);

      println!("----------------search OR test -------------------------");

      let zklnsOR = search_zknotes(
        &itr,
        &svs,
        tuid2,
        &ZkNoteSearch {
          tagsearch: TagSearch::Boolex {
            ts1: Box::new(TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "3".to_string(),
            }),
            ao: AndOr::Or,
            ts2: Box::new(TagSearch::SearchTerm {
              mods: Vec::new(),
              term: "4".to_string(),
            }),
          },
          offset: None,
          limit: None,
        },
      )?;

      assert_eq!(zklnsOR.notes.len(), 2);

      println!("indra test end");
    }
    // delete the test db.
    fs::remove_dir_all(path)?;
    Ok(())
  }
  #[test]
  pub fn test_db_runner() {
    println!("test_db_runner");
    match test_db() {
      Ok(()) => (),
      Err(e) => {
        panic!(format!("{:?}", e));
      }
    }
  }
}
