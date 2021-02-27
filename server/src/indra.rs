// use std::convert::TryInto;
// use std::error::Error;
use errors;
use indradb::Datastore;
use indradb::{
  Edge, EdgeKey, EdgeProperty, EdgeQueryExt, SledDatastore, SledTransaction, Transaction, Type,
  Vertex, VertexQueryExt,
};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::time::Duration;
use icontent::{
  Direction, GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkLink, SaveZkNote,
  SavedZkNote, User, UserId, ZkLink, ZkListNote, ZkNote, ZkNoteEdit,
};
use indra_util::{find_all_q, find_first_q, getoptedgeprop, getoptprop, getprop};
use isearch::{AndOr, SearchMod, TagSearch, ZkNoteSearch, ZkNoteSearchResult};
use std::time::SystemTime;
use util::now;
use uuid::Uuid;
use zkprotocol::content as C;

#[derive(Debug, Clone)]
pub struct SystemVs {
  pub public: Uuid,
  pub search: Uuid,
  pub share: Uuid,
}

pub fn new_user<T: indradb::Transaction>(
  itr: &T,
  public: &Uuid,
  name: String,
  hashwd: String,
  salt: String,
  email: String,
  registration_key: Option<String>,
) -> Result<UserId, errors::Error> {
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
  let uid = UserId(v.id);

  // add link to 'public'
  save_zklink(itr, &v.id, &public, &uid, &None)?;

  // give it note fields too, so it can be retrieved like a note in queries.
  let szn = SaveZkNote {
    id: Some(v.id),
    title: name.clone(),
    pubid: None,
    content: "".to_string(),
  };
  save_zknote(itr, uid, &szn);

  Ok(uid)
}

pub fn save_user<T: indradb::Transaction>(itr: &T, user: &User) -> Result<(), errors::Error> {
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(user.id.0).into(),
      "name",
    ),
    &serde_json::to_value(user.name.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(user.id.0).into(),
      "hashwd",
    ),
    &serde_json::to_value(user.hashwd.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(user.id.0).into(),
      "salt",
    ),
    &serde_json::to_value(user.salt.clone())?,
  )?;
  itr.set_vertex_properties(
    indradb::VertexPropertyQuery::new(
      indradb::SpecificVertexQuery::single(user.id.0).into(),
      "email",
    ),
    &serde_json::to_value(user.email.clone())?,
  )?;
  match &user.registration_key {
    Some(k) => {
      itr.set_vertex_properties(
        indradb::VertexPropertyQuery::new(
          indradb::SpecificVertexQuery::single(user.id.0).into(),
          "registration_key",
        ),
        &serde_json::to_value(k.clone())?,
      )?;
    }
    None => (),
  }

  Ok(())
}

pub fn read_user<T: indradb::Transaction>(itr: &T, name: &String) -> Result<User, errors::Error> {
  let jn = serde_json::to_value(name)?;

  let tuid = find_first_q(
    itr,
    mkpropquery("user".to_string(), "name".to_string()),
    |x| x.value == jn,
  )?
  .ok_or(SimpleError::new("user not found"))?
  .id;

  let uq: indradb::VertexQuery = indradb::SpecificVertexQuery::single(tuid).into();

  Ok(User {
    id: UserId(tuid),
    name: name.clone(),
    hashwd: getprop(itr, &uq, "hashwd")?,
    salt: getprop(itr, &uq, "salt")?,
    email: getprop(itr, &uq, "email")?,
    registration_key: getoptprop(itr, &uq, "registration_key")?,
  })
}

pub fn login_data<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
) -> Result<LoginData, errors::Error> {
  let svs = get_systemvs(itr)?;

  // TODO: make login token that expires!
  let uq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(uid.0).into());

  Ok(LoginData {
    userid: uid,
    username: getprop(itr, &uq, "name")?,
    publicid: svs.public,
    shareid: svs.share,
    searchid: svs.search,
  })
}

pub fn save_zklink<T: indradb::Transaction>(
  itr: &T,
  fromid: &Uuid,
  toid: &Uuid,
  user: &UserId,
  ltype: &Option<&str>,
) -> Result<i64, errors::Error> {
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
    *toid,
    match ltype {
      Some(t) => Type::new(t.to_string())?,
      None => Type::default(),
    },
    *fromid,
  );

  let useruuid = user.0.to_string();

  itr.create_edge(&ek)?;

  // add the user.
  itr.set_edge_properties(
    indradb::EdgePropertyQuery::new(
      indradb::EdgeQuery::Specific(indradb::SpecificEdgeQuery::single(ek.clone())),
      useruuid,
    ),
    &serde_json::to_value(true)?,
  )?;

  // increment user count.
  let ucount =
    indradb::EdgePropertyQuery::new(indradb::SpecificEdgeQuery::single(ek).into(), "usercount");

  match itr.get_edge_properties(ucount.clone())?.first() {
    Some(c) => {
      let count = c.value.as_i64().ok_or(simple_error::SimpleError::new(
        format!("edge user count not a number! {}", c.value)
          .to_string()
          .as_str(),
      ))?;
      itr.set_edge_properties(ucount, &serde_json::to_value(count + 1)?)?;
      Ok(count + 1)
    }
    None => {
      itr.set_edge_properties(ucount, &serde_json::to_value(1)?)?;
      Ok(1)
    }
  }
}

pub fn delete_zklink<T: indradb::Transaction>(
  itr: &T,
  fromid: &Uuid,
  toid: &Uuid,
  user: &UserId,
  ltype: &Option<&str>,
) -> Result<i64, errors::Error> {
  let ek = indradb::EdgeKey::new(
    *toid,
    match ltype {
      Some(t) => Type::new(t.to_string())?,
      None => Type::default(),
    },
    *fromid,
  );

  let useruuid = user.0.to_string();

  // is this user on the edge now?
  let ueq = indradb::EdgePropertyQuery::new(
    indradb::SpecificEdgeQuery::single(ek.clone()).into(),
    useruuid,
  );

  match itr.get_edge_properties(ueq.clone())?.first() {
    None => Ok(0),
    Some(_) => {
      // remove the user
      itr.delete_edge_properties(ueq)?;

      // decrement user count.
      let ucount = indradb::EdgePropertyQuery::new(
        indradb::SpecificEdgeQuery::single(ek.clone()).into(),
        "usercount",
      );

      let eps = itr.get_edge_properties(ucount.clone())?;
      let c = eps.first().ok_or(simple_error::SimpleError::new(
        "user count wasn't found on this edge",
      ))?;
      let count = c.value.as_i64().ok_or(simple_error::SimpleError::new(
        format!("edge user count not a number! {}", c.value)
          .to_string()
          .as_str(),
      ))?;

      if count == 1 {
        // last user out, delete the edge
        itr.delete_edges(indradb::SpecificEdgeQuery::single(ek))?;
        Ok(0)
      } else {
        itr.set_edge_properties(ucount, &serde_json::to_value(count - 1)?)?;
        Ok(count - 1)
      }
    }
  }
}

pub fn save_savezklinks<T: indradb::Transaction>(
  itr: &T,
  userid: UserId,
  zknid: Uuid,
  zklinks: Vec<SaveZkLink>,
) -> Result<(), errors::Error> {
  for link in zklinks.iter() {
    let (from, to) = match link.direction {
      Direction::From => (zknid, link.otherid),
      Direction::To => (link.otherid, zknid),
    };
    if link.user.0 == userid.0 {
      if link.delete == Some(true) {
        delete_zklink(itr, &from, &to, &userid, &None)?;
      } else {
        save_zklink(itr, &from, &to, &userid, &None)?;
      }
    }
  }

  Ok(())
}

pub fn get_systemvs<T: indradb::Transaction>(itr: &T) -> Result<SystemVs, errors::Error> {
  let vq = indradb::VertexQuery::Range(indradb::RangeVertexQuery::new().t(Type::new("system")?));

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
  ltype: &indradb::Type,
  from: Uuid,
  to: Uuid,
) -> Result<bool, errors::Error> {
  let v = itr.get_edges(indradb::EdgeQuery::Specific(
    indradb::SpecificEdgeQuery::single(indradb::EdgeKey::new(to, ltype.clone(), from)),
  ))?;

  Ok(!v.is_empty())
}

pub fn is_note_public<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  id: Uuid,
) -> Result<bool, errors::Error> {
  link_exists(itr, &Type::default(), id, svs.public)
}

pub fn is_note_mine<T: indradb::Transaction>(
  itr: &T,
  id: Uuid,
  uid: UserId,
) -> Result<bool, errors::Error> {
  link_exists(itr, &Type::new("owner")?, id, uid.0)
}

pub fn save_zknote<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  note: &SaveZkNote,
) -> Result<SavedZkNote, errors::Error> {
  let now = now()?;

  // If we're creating a new note, create it with note type.
  // otherwise, we'll just keep the existing vertex type.
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
  save_zklink(itr, &id, &uid.0, &uid, &Some("owner"))?;

  Ok(SavedZkNote {
    id: id,
    changeddate: now,
  })
}

pub fn note_owner<T: indradb::Transaction>(itr: &T, id: Uuid) -> Result<Uuid, errors::Error> {
  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());
  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Outbound)
    .t(Type::new("owner")?)
    .limit(1);

  let user = itr
    .get_edges(eq)?
    .first()
    .ok_or(SimpleError::new("user not found1!"))?
    .key
    .inbound_id;

  Ok(user)
}

pub fn is_note_shared<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: UserId,
  id: Uuid,
) -> Result<bool, errors::Error> {
  // get the edges for this note.
  let sq = indradb::SpecificVertexQuery::single(id).inbound(); // edges where inbound = id?
  let es: Vec<Edge> = itr.get_edges(sq)?; // the vertex ids are the outbound ends.

  // any connections between the vertices and svs.share?
  let eks = es
    .iter()
    .map(|x: &Edge| indradb::EdgeKey::new(svs.share, Type::default(), x.key.outbound_id))
    .collect();

  let share_q: indradb::EdgeQuery = indradb::SpecificEdgeQuery::new(eks).into();

  // any of those connect to our user?
  let eq2: Vec<EdgeKey> = itr
    .get_edges(share_q)?
    .iter()
    .map(|x: &Edge| indradb::EdgeKey::new(uid.0, Type::default(), x.key.inbound_id))
    .collect();

  let shared = !eq2.is_empty();

  Ok(shared)
}

pub fn is_note_accessible<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<UserId>,
  id: Uuid,
) -> Result<bool, errors::Error> {
  let accessible = is_note_public(itr, svs, id)?
    || match uid {
      Some(uid) => is_note_mine(itr, id, uid)? || is_note_shared(itr, svs, uid, id)?,
      None => false,
    };
  Ok(accessible)
}

pub fn read_zknote<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<UserId>,
  id: Uuid,
) -> Result<ZkNote, errors::Error> {
  let accessible = is_note_accessible(itr, svs, uid, id)?;

  (if accessible {
    Ok(())
  } else {
    Err(simple_error::SimpleError::new(
      "permission error: can't access this note",
    ))
  })?;

  // TODO: deny access ^

  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());

  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Inbound)
    .limit(1)
    .t(Type::new("owner")?);

  let user = UserId(
    itr
      .get_edges(eq)?
      .first()
      .ok_or(SimpleError::new("user not found2!"))?
      .key
      .outbound_id,
  );

  // query for getting user name.
  let uq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(user.0).into());

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

pub fn read_zklistnote<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<UserId>,
  id: Uuid,
) -> Result<ZkListNote, errors::Error> {
  let accessible = is_note_accessible(itr, svs, uid, id)?;

  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());

  // println!("all props: {:?}", itr.get_all_vertex_properties(vq.clone()));

  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Inbound)
    .limit(1)
    .t(Type::new("owner")?);

  let user = UserId(
    itr
      .get_edges(eq)?
      .first()
      .ok_or(SimpleError::new("owner not found3!"))?
      .key
      .outbound_id,
  );

  // println!("allprops {:?}", itr.get_all_vertex_properties(vq.clone())?);

  Ok(ZkListNote {
    id: id,
    title: getprop(itr, &vq, "title")?,
    user: user,
    createdate: getprop(itr, &vq, "createdate")?,
    changeddate: getprop(itr, &vq, "changeddate")?,
  })
}

pub fn delete_zknote<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  id: Uuid,
) -> Result<(), errors::Error> {
  if !is_note_mine(itr, id, uid)? {
    Err(simple_error::SimpleError::new("can't delete another user's note.").into())
  } else {
    let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());
    itr.delete_vertices(vq)?;
    Ok(())
  }
}

pub fn read_zklinks<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<UserId>,
  id: Uuid,
) -> Result<Vec<ZkLink>, errors::Error> {
  let ib = indradb::SpecificVertexQuery::single(id).inbound();
  // .t(Type::default());

  println!("read_zklinks - is_note_accessible(itr, svs, uid, id)?;");

  if !is_note_accessible(itr, svs, uid, id)? {
    println!("no");
    Ok(Vec::new())
  } else {
    println!("yes");
    let mut links = Vec::new();

    for e in itr.get_edges(ib)?.iter() {
      assert_eq!(e.key.inbound_id, id);

      println!("got inbound edge: {:?}", e);

      if is_note_accessible(itr, svs, uid, e.key.outbound_id)? {
        println!("is accessible: {:?}", id);
        let nq: indradb::VertexQuery =
          indradb::SpecificVertexQuery::single(e.key.outbound_id).into();

        let eq: indradb::EdgeQuery = indradb::SpecificEdgeQuery::single(e.key.clone()).into();

        // let uidstr = uid.to_string();
        let count: i64 = itr
          .get_edge_properties(indradb::EdgePropertyQuery::new(eq.clone(), "count"))?
          .first()
          .and_then(|ep| ep.value.as_i64())
          .unwrap_or(0);

        let mine: bool = uid
          .and_then(|id| getoptedgeprop(itr, &eq, id.0.to_string().as_str()).ok())
          .unwrap_or(None)
          .unwrap_or(false);

        // create link for every user??
        links.push(ZkLink {
          from: e.key.outbound_id,
          to: id,
          mine: mine,
          others: (count > (if mine { 1 } else { 0 })),
          delete: None,
          fromname: getoptprop(itr, &nq, "title")?,
          toname: None,
        });
      }
    }

    let ob = indradb::SpecificVertexQuery::single(id).outbound();
    // .t(Type::default());
    for e in itr.get_edges(ob)?.iter() {
      assert_eq!(e.key.outbound_id, id);

      println!("got outbound edge: {:?}", e);

      if is_note_accessible(itr, svs, uid, e.key.inbound_id)? {
        let nq: indradb::VertexQuery =
          indradb::SpecificVertexQuery::single(e.key.inbound_id).into();
        let eq: indradb::EdgeQuery = indradb::SpecificEdgeQuery::single(e.key.clone()).into();

        // let uidstr = uid.to_string();
        let count: i64 = itr
          .get_edge_properties(indradb::EdgePropertyQuery::new(eq.clone(), "count"))?
          .first()
          .and_then(|ep| ep.value.as_i64())
          .unwrap_or(0);

        let mine: bool = uid
          .and_then(|id| getoptedgeprop(itr, &eq, id.0.to_string().as_str()).ok())
          .unwrap_or(None)
          .unwrap_or(false);

        // create link for every user??
        links.push(ZkLink {
          from: id,
          to: e.key.inbound_id,
          mine: mine,
          others: (count > (if mine { 1 } else { 0 })),
          delete: None,
          fromname: None,
          toname: getoptprop(itr, &nq, "title")?,
        });
      }
    }

    Ok(links)
  }
}

pub fn read_zknoteedit<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  id: Uuid,
) -> Result<ZkNoteEdit, errors::Error> {
  let svs = get_systemvs(itr)?;

  println!("read_zknoteedit");

  // should do an ownership check for us
  let zknote = read_zknote(itr, &svs, Some(uid), id)?;

  let zklinks = read_zklinks(itr, &svs, Some(uid), id)?;

  Ok(ZkNoteEdit {
    zknote: zknote,
    links: zklinks,
  })
}

/*
pub fn power_delete_zknotes<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  search: &TagSearch,
) -> Result<i64, errors::Error> {
}
*/

pub fn mkpropquery(
  vtype: String,
  prop: String,
) -> Box<dyn Fn(Option<Uuid>) -> Result<indradb::VertexPropertyQuery, errors::Error>> {
  Box::new(move |uuid| match uuid {
    Some(id) => Ok(indradb::VertexPropertyQuery::new(
      indradb::RangeVertexQuery::new()
        .t(Type::new(vtype.as_str())?)
        .start_id(id)
        .into(),
      prop.as_str(),
    )),
    None => Ok(indradb::VertexPropertyQuery::new(
      indradb::RangeVertexQuery::new()
        .t(Type::new(vtype.as_str())?)
        .into(),
      prop.as_str(),
    )),
  })
}

#[derive(Eq, Debug, PartialEq, Clone, Hash)]
pub struct TagTerm {
  pub exact: bool,
  pub note: bool,
  pub user: bool,
  pub term: String,
}

// impl std::cmp::PartialEq for TagTerm {}

pub fn checknote<T: indradb::Transaction>(
  itr: &T,
  uuid: Uuid,
  ts: &TagSearch,
  user: &UserId,
  mut tcache: &mut HashMap<TagTerm, Vec<Uuid>>,
) -> Result<bool, errors::Error> {
  // println!("checknote: {:?}", uuid);
  match ts {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut note = false;
      let mut usermod = false;
      for m in mods {
        match m {
          SearchMod::ExactMatch => {
            exact = true;
          }
          SearchMod::Tag => {
            tag = true;
          }
          SearchMod::Note => {
            note = true;
          }
          SearchMod::User => {
            usermod = true;
          }
        }
      }

      let ret = if tag {
        let tt = TagTerm {
          exact: exact,
          note: note,
          user: usermod,
          term: term.clone(),
        };

        let uuids = match tcache.get(&tt) {
          Some(uuids) => uuids,
          None => {
            let mods = mods
              .iter()
              .filter(|m| **m != SearchMod::Tag)
              .map(|m| m.clone())
              .collect();
            let ts = TagSearch::SearchTerm {
              mods: mods,
              term: term.clone(),
            };
            println!("tagsearcing with ts: {:?}", ts);
            let uuids = tagsearch(itr, user, &ts)?;
            println!("result: {:?}", uuids);
            tcache.insert(tt.clone(), uuids);
            tcache
              .get(&tt)
              .ok_or(simple_error::SimpleError::new("wat"))?
          }
        };

        if uuids.is_empty() {
          false
        } else {
          let mut eks = Vec::new();
          for id in uuids {
            eks.push(indradb::EdgeKey::new(uuid, indradb::Type::default(), *id));
            eks.push(indradb::EdgeKey::new(*id, indradb::Type::default(), uuid));
          }

          let edges = itr.get_edges(indradb::SpecificEdgeQuery::new(eks))?;
          !edges.is_empty()
        }
      } else {
        let q = indradb::SpecificVertexQuery::single(uuid).into();
        let vpq = if note {
          indradb::VertexPropertyQuery::new(q, "content")
        } else {
          indradb::VertexPropertyQuery::new(q, "title")
        };
        let vps = itr.get_vertex_properties(vpq)?;

        let mut ret = false;
        if exact {
          for p in vps {
            match p.value.as_str() {
              Some(s) => {
                if s == *term {
                  ret = true;
                  break;
                }
              }
              None => (),
            }
          }
        } else {
          for p in vps {
            match p
              .value
              .to_string()
              .to_lowercase()
              .find(&term.to_lowercase())
            {
              Some(_) => {
                ret = true;
                break;
              }
              None => {}
            }
          }
        }
        ret
      };
      Ok(ret)
    }
    TagSearch::Not { ts } => Ok(!checknote(itr, uuid, ts, &user, &mut tcache)?),
    TagSearch::Boolex { ts1, ao, ts2 } => Ok(match ao {
      AndOr::Or => {
        checknote(itr, uuid, ts1, &user, &mut tcache)?
          || checknote(itr, uuid, ts2, &user, &mut tcache)?
      }
      AndOr::And => {
        checknote(itr, uuid, ts1, &user, &mut tcache)?
          && checknote(itr, uuid, ts2, &user, &mut tcache)?
      }
    }),
  }
}

pub fn tagsearch<T: indradb::Transaction>(
  itr: &T,
  user: &UserId,
  search: &TagSearch,
) -> Result<Vec<Uuid>, errors::Error> {
  // lets search notes this user owns.
  let uq = indradb::SpecificVertexQuery::single(user.0)
    .outbound()
    .inbound()
    .t(indradb::Type::new("note")?);

  let mut tcache = HashMap::new();

  let verts = itr.get_vertices(uq)?;

  let mut res = Vec::new();

  for v in verts {
    if checknote(itr, v.id, search, &user, &mut tcache)? {
      res.push(v.id);
    }
  }

  Ok(res)
}

pub fn search_zknotes<T: indradb::Transaction>(
  itr: &T,
  systemvs: &SystemVs,
  user: UserId,
  search: &ZkNoteSearch,
) -> Result<ZkNoteSearchResult, errors::Error> {
  let ids = tagsearch(itr, &user, &search.tagsearch)?;

  let mut notes = Vec::new();

  for id in ids {
    let zkln = read_zklistnote(itr, systemvs, Some(user), id)?;
    notes.push(zkln);
  }

  Ok(ZkNoteSearchResult {
    notes: notes,
    offset: search.offset,
  })
}
