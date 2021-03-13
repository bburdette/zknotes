use crate::errors;
use crate::icontent::{
  Direction, GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkLink, SaveZkNote,
  SavedZkNote, User, UserId, ZkLink, ZkListNote, ZkNote, ZkNoteEdit,
};
use crate::indra::{new_user, save_zklink, save_zknote};
use crate::user::ZkDatabase;
use indradb::Datastore;
use indradb::Transaction;
use indradb::{Edge, EdgeKey, EdgeProperty, EdgeQueryExt, Type, Vertex, VertexQueryExt};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::Path; // as U;

pub fn import_db(zd: &ZkDatabase, path: &Path) -> Result<(), errors::Error> {
  // compression factor of 5 (default)
  let sc = indradb::SledConfig::with_compression(None);

  let ids = sc.open(path)?;

  let itr = ids.transaction()?;

  // make system vertices.
  // necessary?
  // 'public' 'share' 'search'
  // make these into notes?
  let pubid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;

    v.id
  };

  // make a system user.
  let sysuid = new_user(
    &itr,
    &pubid,
    "system".to_string(),
    "wat".to_string(),
    "wat".to_string(),
    "wat".to_string(),
    Some("wat".to_string()),
  )?;

  // add note fields to 'public', so it can be retrieved like a note in queries.
  let szn = SaveZkNote {
    id: Some(pubid),
    title: "public".to_string(),
    pubid: None,
    content: "".to_string(),
  };
  save_zknote(&itr, sysuid, &szn)?;

  let shareid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    // give it note fields, so it can be retrieved like a note in queries.
    let szn = SaveZkNote {
      id: Some(v.id),
      title: "share".to_string(),
      pubid: None,
      content: "".to_string(),
    };
    save_zknote(&itr, sysuid, &szn)?;
    v.id
  };

  let searchid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    let szn = SaveZkNote {
      id: Some(v.id),
      title: "search".to_string(),
      pubid: None,
      content: "".to_string(),
    };
    save_zknote(&itr, sysuid, &szn)?;
    v.id
  };

  // makin users.
  // hashmap of sqlite ids to uuids.
  let mut uids: HashMap<i64, UserId> = HashMap::new();

  let mut unotes = HashMap::new();

  for u in zd.users.iter() {
    // make vertex.
    let v = indradb::Vertex::new(indradb::Type::new("user")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "name"),
      &serde_json::to_value(u.name.clone())?,
    )?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(
        indradb::SpecificVertexQuery::single(v.id).into(),
        "hashwd",
      ),
      &serde_json::to_value(u.hashwd.clone())?,
    )?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "salt"),
      &serde_json::to_value(u.salt.clone())?,
    )?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "email"),
      &serde_json::to_value(u.email.clone())?,
    )?;
    match &u.registration_key {
      Some(k) => {
        itr.set_vertex_properties(
          indradb::VertexPropertyQuery::new(
            indradb::SpecificVertexQuery::single(v.id).into(),
            "registration_key",
          ),
          &serde_json::to_value(k.clone())?,
        )?;
      }
      None => (),
    }

    let uid = UserId(v.id);

    // users should be notes too.
    save_zknote(
      &itr,
      sysuid,
      &SaveZkNote {
        id: Some(v.id),
        title: u.name.clone(),
        pubid: None,
        content: "".to_string(),
      },
    )?;

    // save id
    uids.insert(u.id, uid);
    unotes.insert(u.zknote, uid);
  }

  // note ids.
  let mut nids = HashMap::new();

  for n in zd.notes.iter() {
    match n.title.as_str() {
      "public" => nids.insert(n.id, pubid),
      "search" => nids.insert(n.id, searchid),
      "share" => nids.insert(n.id, shareid),
      _ => {
        // is this a user note?
        match unotes.get(&n.id) {
          // yes - link to the user vertex instead of making a note.
          Some(id) => nids.insert(n.id, id.0),
          // no - make a note.
          None => {
            // make vertex.
            let v = indradb::Vertex::new(indradb::Type::new("note")?);
            itr.create_vertex(&v)?;
            itr.set_vertex_properties(
              indradb::VertexPropertyQuery::new(
                indradb::SpecificVertexQuery::single(v.id).into(),
                "title",
              ),
              &serde_json::to_value(n.title.clone())?,
            )?;
            itr.set_vertex_properties(
              indradb::VertexPropertyQuery::new(
                indradb::SpecificVertexQuery::single(v.id).into(),
                "content",
              ),
              &serde_json::to_value(n.content.clone())?,
            )?;
            match &n.pubid {
              Some(pubid) => {
                itr.set_vertex_properties(
                  indradb::VertexPropertyQuery::new(
                    indradb::SpecificVertexQuery::single(v.id).into(),
                    "pubid",
                  ),
                  &serde_json::to_value(pubid.clone())?,
                )?;
              }
              None => (),
            }
            itr.set_vertex_properties(
              indradb::VertexPropertyQuery::new(
                indradb::SpecificVertexQuery::single(v.id).into(),
                "createdate",
              ),
              &serde_json::to_value(n.createdate.clone())?,
            )?;
            itr.set_vertex_properties(
              indradb::VertexPropertyQuery::new(
                indradb::SpecificVertexQuery::single(v.id).into(),
                "changeddate",
              ),
              &serde_json::to_value(n.changeddate.clone())?,
            )?;

            // link to user.
            let uid = uids
              .get(&n.user)
              .ok_or(SimpleError::new("user not found"))?;
            save_zklink(&itr, &v.id, &uid.0, &uid, &Some("owner"))?;

            // save id
            nids.insert(n.id, v.id)
          }
        }
      }
    };
  }
  for l in zd.links.iter() {
    save_zklink(
      &itr,
      nids.get(&l.from).ok_or(SimpleError::new("key not found"))?,
      nids.get(&l.to).ok_or(SimpleError::new("key not found"))?,
      uids.get(&l.user).ok_or(SimpleError::new(
        format!("link user id not found {:?}", l.user).as_str(),
      ))?,
      &None,
    )?;
  }

  Ok(())
}
