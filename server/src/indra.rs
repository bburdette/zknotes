use indradb::Datastore;
use indradb::Transaction;
// use std::convert::TryInto;
// use std::error::Error;
use errors;
use indradb::{Edge, EdgeKey, EdgeQueryExt, Type, Vertex, VertexQueryExt};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::time::Duration;
use icontent::{
  GetZkLinks, GetZkNoteEdit, ImportZkNote, SaveZkNote, SavedZkNote, ZkLink, ZkNote, ZkNoteEdit,
};
use std::time::SystemTime;
use user::{LoginData, User, ZkDatabase};
use util::now;
use uuid::Uuid;
use zkprotocol::content as C;

#[derive(Debug, Clone)]
pub struct SystemVs {
  public: Uuid,
  search: Uuid,
  share: Uuid,
}

pub fn import_db(zd: &ZkDatabase, path: &str) -> Result<(), errors::Error> {
  // compression factor of 5 (default)
  let sc = indradb::SledConfig::with_compression(None);

  // let ids = sc.open(dbpath.as_os_str().to_str().ok_or(bail!("blah"))?)?;
  let ids = sc.open(path)?;

  let itr = ids.transaction()?;

  // make system vertices.
  // necessary?
  // 'public' 'share' 'search'
  // make these into notes?
  let pubid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "title"),
      &serde_json::to_value("public")?,
    )?;
    v.id
  };
  let shareid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "title"),
      &serde_json::to_value("share")?,
    )?;
    v.id
  };
  let searchid = {
    let v = indradb::Vertex::new(indradb::Type::new("system")?);
    itr.create_vertex(&v)?;
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "title"),
      &serde_json::to_value("search")?,
    )?;
    v.id
  };

  // makin users.
  // hashmap of sqlite ids to uuids.
  let mut uids = HashMap::new();

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

    // save id
    uids.insert(u.id, v.id);
    unotes.insert(u.zknote, v.id);
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

  // TODO replace user notes with users.
  for n in zd.notes.iter() {
    match n.title.as_str() {
      "public" => nids.insert(n.id, pubid),
      "search" => nids.insert(n.id, searchid),
      "share" => nids.insert(n.id, shareid),
      _ => {
        // is this a user note?
        match unotes.get(&n.id) {
          // yes - link to the user vertex instead of making a note.
          Some(id) => nids.insert(n.id, *id),
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
            save_zklink(&itr, &v.id, &uid, &uid, &Some("owner"))?;

            // save id
            nids.insert(n.id, v.id)
          }
        }
      }
    };
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
pub fn new_user<T: indradb::Transaction>(
  itr: &T,
  public: &Uuid,
  name: String,
  hashwd: String,
  salt: String,
  email: String,
  registration_key: Option<String>,
) -> Result<Uuid, errors::Error> {
  let v = indradb::Vertex::new(indradb::Type::new("user")?);
  itr.create_vertex(&v)?;

  // make vertex.
  let v = indradb::Vertex::new(indradb::Type::new("user")?);
  itr.create_vertex(&v)?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "name"),
    &serde_json::to_value(name.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "hashwd"),
    &serde_json::to_value(hashwd.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "salt"),
    &serde_json::to_value(salt.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(v.id).into(), "email"),
    &serde_json::to_value(email.clone())?,
  )?;
  match registration_key {
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
  // itr.set_vertex_properties(
  //   indradb::VertexPropertyQuery::new(
  //     indradb::SpecificVertexQuery::single(v.id).into(),
  //     "registration_key",
  //   ),
  //   &serde_json::to_value(registration_key.clone())?,
  // )?;

  // TODO add link to 'public'
  save_zklink(itr, &v.id, &public, &v.id, &None)?;

  Ok(v.id)
}

pub fn save_zklink<T: indradb::Transaction>(
  itr: &T,
  fromid: &Uuid,
  toid: &Uuid,
  user: &Uuid,
  ltype: &Option<&str>,
) -> Result<bool, errors::Error> {
  // link owner.
  // Uuid into type?
  //  then multiple links between things, per owner.
  //  can query on owners easily (find all with type).
  //  delete user, can delete links.
  //  can't use type for something else, like 'isa' or whatever.
  //  seems hacky.
  //
  // Uuid into properties.
  //  on query, have to pull properties for filtering.
  //  to find all user links, must query properties on all links.
  //  only single links between things, unless there are types to the links.
  //  when user deletes a link, actually deleting their Uuid from properties.
  //
  // Just try one and see how it works out.  Don't really know enough yet.
  //
  let ek = indradb::EdgeKey::new(
    *fromid,
    match ltype {
      Some(t) => Type::new(t.to_string())?,
      None => Type::default(),
    },
    *toid,
  );

  let useruuid = user.to_string();

  let ret = itr.create_edge(&ek)?;
  itr.set_edge_properties(
    indradb::EdgePropertyQuery::new(
      indradb::EdgeQuery::Specific(indradb::SpecificEdgeQuery::single(ek)),
      useruuid,
    ),
    &serde_json::to_value(true)?,
  )?;

  Ok(ret)
}

/*
pub fn get_vertex_property<T: indradb::Transaction>(
  itr: &T,
vid: &Uuid, name: &str) -> Result<JsonValue, errors::Error> {

  indradb::get_ver
}
*/

pub fn getprop<T: indradb::Transaction, R>(
  itr: &T,
  vq: &indradb::VertexQuery,
  x: &str,
) -> Result<R, errors::Error>
where
  R: serde::de::DeserializeOwned,
{
  Ok(serde_json::from_value::<R>(
    itr
      .get_vertex_properties(indradb::VertexPropertyQuery::new(vq.clone(), x))?
      .first()
      .ok_or(SimpleError::new(
        format!("property not found: {}", x).as_str(),
      ))?
      .value
      .clone(),
  )?)
}

pub fn getoptprop<T: indradb::Transaction, R>(
  itr: &T,
  vq: &indradb::VertexQuery,
  x: &str,
) -> Result<Option<R>, errors::Error>
where
  R: serde::de::DeserializeOwned,
{
  let r = match itr
    .get_vertex_properties(indradb::VertexPropertyQuery::new(vq.clone(), x))?
    .first()
  {
    Some(vp) => Some(serde_json::from_value::<R>(vp.value.clone())?),
    None => None,
  };
  Ok(r)
}

pub fn get_systemvs<T: indradb::Transaction>(itr: &T) -> Result<SystemVs, errors::Error> {
  let vq = indradb::VertexQuery::Range(indradb::RangeVertexQuery::new(100).t(Type::new("system")?));

  let mut public = None;
  let mut search = None;
  let mut share = None;

  for v in itr
    .get_vertex_properties(indradb::VertexPropertyQuery::new(vq, "title"))?
    .iter()
  {
    match serde_json::from_value::<String>(v.value.clone())?.as_str() {
      "public" => public = Some(v.id),
      "search" => search = Some(v.id),
      "share" => share = Some(v.id),
      wat => (),
    }
  }

  match (public, search, share) {
    (Some(p), Some(se), Some(sh)) => Ok(SystemVs {
      public: p,
      search: se,
      share: sh,
    }),
    _ => Err(errors::Error::from(SimpleError::new(
      "unable to find system vertices",
    ))),
  }
}

pub fn link_exists<T: indradb::Transaction>(
  itr: &T,
  from: Uuid,
  to: Uuid,
) -> Result<bool, errors::Error> {
  let v = itr.get_edges(indradb::EdgeQuery::Specific(
    indradb::SpecificEdgeQuery::single(indradb::EdgeKey::new(from, Type::default(), to)),
  ))?;

  Ok(!v.is_empty())
}

pub fn is_note_public<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  id: Uuid,
) -> Result<bool, errors::Error> {
  link_exists(itr, id, svs.public)
}

pub fn is_note_mine<T: indradb::Transaction>(
  itr: &T,
  id: Uuid,
  uid: Uuid,
) -> Result<bool, errors::Error> {
  link_exists(itr, id, uid)
}

// THIS ASSUMES SEQUENTIALITY
// which is a bad assumption.  invalid!
pub fn intersect<T: indradb::Transaction>(
  itr: &T,
  vq1: indradb::VertexQuery,
  vq2: indradb::VertexQuery,
) -> Result<Vec<Vertex>, errors::Error> {
  let vec1 = itr.get_vertices(vq1)?;
  let vec2 = itr.get_vertices(vq2)?;
  println!("vec1: {:?}", vec1);
  println!("vec2: {:?}", vec2);
  let i1 = vec1.iter();
  let mut i2 = vec2.iter();

  let mut i2v = i2.next();
  let mut i2vcheck = |v: &mut Vec<Vertex>, i: &Vertex| {
    match i2v {
      None => false,
      Some(i2val) => {
        if i == i2val {
          v.push(i.clone());
          false
        } else if i.id < i2val.id {
          // here we're depending on ordering of the UUIDs, but they aren't ordered!
          // because they aren't ordered, you just have to get all of them to do intersection.
          false
        } else {
          i2v = i2.next();
          true
        }
      }
    }
  };

  let mut rv = Vec::new();

  for i in i1 {
    while i2vcheck(&mut rv, i) {}
  }

  Ok(rv)
}

pub fn save_zknote<T: indradb::Transaction>(
  itr: &T,
  uid: Uuid,
  note: &SaveZkNote,
) -> Result<SavedZkNote, errors::Error> {
  let now = now()?;
  let id = match note.id {
    Some(id) => id,
    None => {
      let v = indradb::Vertex::new(indradb::Type::new("note")?);
      itr.create_vertex(&v)?;
      v.id
    }
  };
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(id).into(), "title"),
    &serde_json::to_value(&note.title)?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(id).into(), "content"),
    &serde_json::to_value(&note.content)?,
  )?;
  if let Some(pubid) = &note.pubid {
    itr.set_vertex_properties(
      indradb::VertexPropertyQuery::new(indradb::SpecificVertexQuery::single(id).into(), "content"),
      &serde_json::to_value(pubid)?,
    )?;
  };

  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(id).into(),
      "createdate",
    ),
    &serde_json::to_value(now.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(id).into(),
      "changeddate",
    ),
    &serde_json::to_value(now.clone())?,
  )?;

  // link to user.
  save_zklink(itr, &id, &uid, &uid, &Some("owner"))?;

  Ok(SavedZkNote {
    id: id,
    changeddate: now,
  })
}

pub fn note_owner<T: indradb::Transaction>(itr: &T, id: Uuid) -> Result<Uuid, errors::Error> {
  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());
  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Outbound, 1)
    .t(Type::new("owner")?);

  let user = itr
    .get_edges(eq)?
    .first()
    .ok_or(SimpleError::new("user not found!"))?
    .key
    .inbound_id;

  Ok(user)
}

pub fn is_note_shared<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Uuid,
  id: Uuid,
) -> Result<bool, errors::Error> {
  // get the edges for this note.
  let sq = indradb::SpecificVertexQuery::single(id).outbound(1000); // outbound edges.
  let es: Vec<Edge> = itr.get_edges(sq)?; // the vertex ids are the outbound ends.

  // any connections between the vertices and svs.share?
  let eks = es
    .iter()
    .map(|x: &Edge| indradb::EdgeKey::new(x.key.outbound_id, Type::default(), svs.share))
    .collect();

  let share_q: indradb::EdgeQuery = indradb::SpecificEdgeQuery::new(eks).into();

  // any of those connect to our user?
  let eq2: Vec<EdgeKey> = itr
    .get_edges(share_q)?
    .iter()
    .map(|x: &Edge| indradb::EdgeKey::new(x.key.inbound_id, Type::default(), uid))
    .collect();

  let shared = !eq2.is_empty();

  Ok(shared)
}

pub fn is_note_accessible<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<Uuid>,
  id: Uuid,
) -> Result<bool, errors::Error> {
  let accessible = is_note_public(itr, svs, id)?
    || match uid {
      Some(uid) => is_note_mine(itr, id, uid)? || is_note_shared(itr, svs, id, uid)?,
      None => false,
    };
  Ok(accessible)
}

pub fn read_zknote<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<Uuid>,
  id: Uuid,
) -> Result<ZkNote, errors::Error> {
  let accessible = is_note_accessible(itr, svs, uid, id)?;

  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());

  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Outbound, 1)
    .t(Type::new("owner")?);

  let user = itr
    .get_edges(eq)?
    .first()
    .ok_or(SimpleError::new("user not found!"))?
    .key
    .inbound_id;

  // query for getting user name.
  let uq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(user).into());

  Ok(ZkNote {
    id: id,
    title: getprop(itr, &vq, "title")?,
    content: getprop(itr, &vq, "content")?,
    user: user,
    username: getprop(itr, &uq, "name")?,
    pubid: getoptprop(itr, &vq, "pubid")?,
    createdate: getprop(itr, &vq, "createdate")?,
    changeddate: getprop(itr, &vq, "changeddate")?,
  })
}

// todo: repeat until found or none.
pub fn find_first<T: indradb::Transaction, F>(
  itr: &T,
  vpq: indradb::VertexPropertyQuery,
  test: F,
) -> Result<Option<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool,
{
  let vps = itr.get_vertex_properties(vpq)?;
  let ret = vps.iter().find(test);
  match ret {
    Some(d) => Ok(Some(d.clone())),
    None => Ok(None),
  }
}

pub fn find_first_q<T: indradb::Transaction, F, Q>(
  itr: &T,
  vpqf: Q,
  test: F,
) -> Result<Option<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool + Copy,
  Q: Fn(Option<Uuid>) -> Result<indradb::VertexPropertyQuery, errors::Error>,
{
  let mut ret = None;
  let mut uuid = None;
  let mut vpq = vpqf(uuid)?;
  let mut vps = itr.get_vertex_properties(vpq)?;
  while (!vps.is_empty()) {
    match vps.iter().find(test) {
      Some(r) => {
        ret = Some(r.clone());
        break;
      }
      None => match vps.last() {
        Some(l) => {
          vpq = vpqf(Some(l.id))?;
          vps = itr.get_vertex_properties(vpq)?;
        }
        None => break,
      },
    }
  }
  match ret {
    Some(r) => Ok(Some(r.clone())),
    None => Ok(None),
  }
}

pub fn mkpropquery(
  vtype: String,
  prop: String,
) -> Box<dyn Fn(Option<Uuid>) -> Result<indradb::VertexPropertyQuery, errors::Error>> {
  Box::new(move |uuid| match uuid {
    Some(id) => Ok(indradb::VertexPropertyQuery::new(
      indradb::RangeVertexQuery::new(100)
        .t(Type::new(vtype.as_str())?)
        .start_id(id)
        .into(),
      prop.as_str(),
    )),
    None => Ok(indradb::VertexPropertyQuery::new(
      indradb::RangeVertexQuery::new(100)
        .t(Type::new(vtype.as_str())?)
        .into(),
      prop.as_str(),
    )),
  })
}

pub fn test_db(path: &str) -> Result<(), errors::Error> {
  // compression factor of 5 (default)
  let sc = indradb::SledConfig::with_compression(None);

  // let ids = sc.open(dbpath.as_os_str().to_str().ok_or(bail!("blah"))?)?;
  let ids = sc.open(path)?;

  let itr = ids.transaction()?;

  let svs = get_systemvs(&itr)?;

  // user names
  /*
  let vq = indradb::VertexPropertyQuery::new(
    indradb::RangeVertexQuery::new(100)
      .t(Type::new("user")?)
      .into(),
    "name",
  );

  let ffu = find_first(&itr, vq, |x| x.value == "meh")?;
  println!("ffu: {:?}", ffu);
  */

  let tuid = match find_first_q(
    &itr,
    mkpropquery("user".to_string(), "name".to_string()),
    |x| x.value == "test",
  )? {
    Some(vp) => vp.id,
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

  let szn = SaveZkNote {
    id: None,
    title: "test title".to_string(),
    pubid: None,
    content: "test content".to_string(),
  };

  let sid = save_zknote(&itr, tuid, &szn)?;

  let zkn = read_zknote(&itr, &svs, None, sid.id)?;

  println!("{}", serde_json::to_string_pretty(&zkn)?);

  println!("indra test end");

  Ok(())
}
