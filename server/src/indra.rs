use indradb::Datastore;
use indradb::Transaction;
// use std::convert::TryInto;
// use std::error::Error;
use errors;
use indradb::{Edge, Type, Vertex};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::time::Duration;
use std::time::SystemTime;
use user::{LoginData, User, ZkDatabase};
use uuid::Uuid;
use zkprotocol::content::{
  GetZkLinks, GetZkNoteEdit, ImportZkNote, SaveZkNote, SavedZkNote, ZkLink, ZkNote, ZkNoteEdit,
};

pub fn import_db(zd: &ZkDatabase, path: &str) -> Result<(), errors::Error> {
  // compression factor of 5 (default)
  let sc = indradb::SledConfig::with_compression(None);

  // let ids = sc.open(dbpath.as_os_str().to_str().ok_or(bail!("blah"))?)?;
  let ids = sc.open(path)?;

  let itr = ids.transaction()?;

  // make system vertices.
  // necessary?
  // 'public' 'share' 'search'
  {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "name"),
      &serde_json::to_value("public")?,
    )?;
  };
  {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "name"),
      &serde_json::to_value("share")?,
    )?;
  };
  {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "name"),
      &serde_json::to_value("search")?,
    )?;
  };

  // makin users.
  // hashmap of sqlite ids to uuids.
  let mut uids = HashMap::new();

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

    // save id
    uids.insert(u.id, v.id);
  }

  // note ids.
  let mut nids = HashMap::new();

  /*
  pub struct ZkNote {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub user: i64,
    pub username: String,
    pub pubid: Option<String>,
    pub createdate: i64,
    pub changeddate: i64,
  }
  */

  for n in zd.notes.iter() {
    // make vertex.
    let v = indradb::Vertex::new(indradb::Type::new("note")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "title"),
      &serde_json::to_value(n.title.clone())?,
    )?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(
        indradb::SpecificVertexQuery::single(v.id).into(),
        "content",
      ),
      &serde_json::to_value(n.content.clone())?,
    )?;
    // link to user.
    // itr.set_vertex_properties(
    //   indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "user"),
    //   &serde_json::to_value(n.user.clone())?,
    // )?;
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

    // save id
    nids.insert(n.id, v.id);
  }
  /*
  pub struct ZkLink {
    pub from: i64,
    pub to: i64,
    pub user: i64,
    pub linkzknote: Option<i64>,
    pub delete: Option<bool>,
    pub fromname: Option<String>,
    pub toname: Option<String>,
  }
  */
  for l in zd.links.iter() {
    // make link.
    // let e = indradb::Edge::new_with_current_datetime(indradb::EdgeKey::new(
    //   Uuid::default(),
    //   Type::default(),
    //   Uuid::default(),
    // ));
    itr.create_edge(&indradb::EdgeKey::new(
      *nids.get(&l.from).ok_or(SimpleError::new("key not found"))?,
      Type::default(),
      *nids.get(&l.to).ok_or(SimpleError::new("key not found"))?,
    ))?;

    // link to user.

    // save id
    // nids.insert(n.id, v.id);
  }

  Ok(())
}

/*
pub struct User {
  pub id: i64,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
}
*/
