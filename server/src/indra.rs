use indradb::Datastore;
use indradb::Transaction;
// use std::convert::TryInto;
// use std::error::Error;
use errors;
use indradb::{Edge, EdgeKey, EdgeProperty, EdgeQueryExt, Type, Vertex, VertexQueryExt};
use simple_error::SimpleError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::time::Duration;
use icontent::{
  GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkNote, SavedZkNote, UserId, ZkLink,
  ZkListNote, ZkNote, ZkNoteEdit,
};
use indra_util::{find_all_q, find_first_q, getoptedgeprop, getoptprop, getprop};
use isearch::{SearchMod, TagSearch, ZkNoteSearch, ZkNoteSearchResult};
use std::time::SystemTime;
use user::{User, ZkDatabase};
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

    // save id
    uids.insert(u.id, uid);
    unotes.insert(u.zknote, uid);
  }

  // note ids.
  let mut nids = HashMap::new();

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

    // make type be user id?
    // then easy to search on links for that user.
    // but what about search for any link?  have to specify type in search.

    // make id a property:
    // "<uuid>" true
    // how to determine if no users have the link anymore then?
    // 		more common to search links than to delete links.  optimize for that.
    // 		on delete, get_all_edge_properties() perhaps, or at least count them.
    // 		could keep a count on the edge in a property.  GENIAL

    // "is link mine"
    // 	getprop(<uuid>)
    // "search only on my links"
    // 	get links, propsearch my id.
    // "delete link"
    // 	remove id, decrement counter.  0?
    // "create link"
    // 	find link, add id.
    // 	create link, add id.
    // "get all users of link"
    // 	get_all_edge_properties()
    // 	convert all to uuid, the ones that succeed are ids.  ew.
    // "delete user"
    // 	propquery of <uuid> gets links.
    // "delete vertex"
    // 	are vertices ever deleted?  flag as deleted?
    // 	change type to deleted?
    // vertex history, link history?
    //
    // link to user?
    // is uid a property?
    // can't have links from links to users.
    // users array?  ugh
    // ditch link ownership??

    // save id
    // nids.insert(n.id, v.id);
  }

  Ok(())
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

  let uid = UserId(v.id);

  // TODO add link to 'public'
  save_zklink(itr, &v.id, &public, &uid, &None)?;

  Ok(uid)
}

pub fn read_user<T: indradb::Transaction>(itr: &T, name: String) -> Result<User, errors::Error> {
  let tuid = find_first_q(
    itr,
    mkpropquery("user".to_string(), "name".to_string()),
    |x| x.value == "test",
  )?
  .ok_or(SimpleError::new("user not found"))?
  .id;

  let uq: indradb::VertexQuery = indradb::SpecificVertexQuery::single(tuid).into();

  Ok(User {
    // id: tuid,
    id: 0,
    zknote: 0,
    name: name,
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
    *toid,
    match ltype {
      Some(t) => Type::new(t.to_string())?,
      None => Type::default(),
    },
    *fromid,
  );

  let useruuid = user.0.to_string();

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
    .ok_or(SimpleError::new("user not found!"))?
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

  let vq = indradb::VertexQuery::Specific(indradb::SpecificVertexQuery::single(id).into());

  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Inbound)
    .limit(1)
    .t(Type::new("owner")?);

  let user = UserId(
    itr
      .get_edges(eq)?
      .first()
      .ok_or(SimpleError::new("user not found!"))?
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

  // get note owner.
  let eq = indradb::PipeEdgeQuery::new(Box::new(vq.clone()), indradb::EdgeDirection::Inbound)
    .limit(1)
    .t(Type::new("owner")?);

  let user = UserId(
    itr
      .get_edges(eq)?
      .first()
      .ok_or(SimpleError::new("user not found!"))?
      .key
      .outbound_id,
  );

  println!("allprops {:?}", itr.get_all_vertex_properties(vq.clone())?);

  Ok(ZkListNote {
    id: id,
    title: getprop(itr, &vq, "title")?,
    user: user,
    createdate: getprop(itr, &vq, "createdate")?,
    changeddate: getprop(itr, &vq, "changeddate")?,
  })
}

pub fn read_zklinks<T: indradb::Transaction>(
  itr: &T,
  svs: &SystemVs,
  uid: Option<UserId>,
  id: Uuid,
) -> Result<Vec<ZkLink>, errors::Error> {
  let ib = indradb::SpecificVertexQuery::single(id)
    .inbound()
    .t(Type::default());

  let mut links = Vec::new();

  for e in itr.get_edges(ib)?.iter() {
    assert_eq!(e.key.inbound_id, id);

    if is_note_accessible(itr, svs, uid, id)? {
      let nq: indradb::VertexQuery = indradb::SpecificVertexQuery::single(e.key.outbound_id).into();

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

  let ob = indradb::SpecificVertexQuery::single(id)
    .outbound()
    .t(Type::default());
  for e in itr.get_edges(ob)?.iter() {
    assert_eq!(e.key.outbound_id, id);

    if is_note_accessible(itr, svs, uid, id)? {
      let nq: indradb::VertexQuery = indradb::SpecificVertexQuery::single(e.key.inbound_id).into();

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

pub fn read_zknoteedit<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  gzl: &GetZkNoteEdit,
) -> Result<ZkNoteEdit, errors::Error> {
  let svs = get_systemvs(itr)?;

  // should do an ownership check for us
  let zknote = read_zknote(itr, &svs, Some(uid), gzl.zknote)?;

  let zklinks = read_zklinks(itr, &svs, Some(uid), zknote.id)?;

  Ok(ZkNoteEdit {
    zknote: zknote,
    links: zklinks,
  })
}

/*
pub fn search_zknotes<T: indradb::Transaction>(
  itr: &T,
  uid: UserId,
  search: &ZkNoteSearch,
) -> Result<ZkNoteSearchResult, errors::Error> {
}

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

#[cfg(test)]
mod test {
  use super::*;
  use std::fs;

  pub fn test_db() -> Result<(), errors::Error> {
    println!("test-db starrt");
    let path = "indra-test";
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

      assert_eq!(
        true,
        checknote(
          &itr,
          sid4.id,
          &TagSearch::SearchTerm {
            mods: Vec::new(),
            term: "4".to_string()
          }
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
          }
        )?
      );

      println!("----------------search test -------------------------");

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

pub fn checknote<T: indradb::Transaction>(
  itr: &T,
  uuid: Uuid,
  ts: &TagSearch,
) -> Result<bool, errors::Error> {
  println!("checknote: {:?}", uuid);
  match ts {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut note = false;
      let mut user = false;
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
            user = true;
          }
        }
      }

      let q = indradb::SpecificVertexQuery::single(uuid).into();
      let vpq = indradb::VertexPropertyQuery::new(q, "title");
      let mut vps = itr.get_vertex_properties(vpq)?;

      let mut ret = false;
      for p in vps {
        println!("pval: {:?}", p.value);
        match p.value.to_string().to_lowercase().find(term) {
          Some(_) => {
            println!("found: {}", term);

            ret = true;
            break;
          }
          None => {}
        }
      }
      Ok(ret)
    }
    TagSearch::Not { ts } => Ok(false),
    TagSearch::Boolex { ts1, ao, ts2 } => Ok(false),
  }
}

pub fn tagsearch<T: indradb::Transaction>(
  itr: &T,
  user: UserId,
  search: &TagSearch,
) -> Result<Vec<Uuid>, errors::Error> {
  // lets search notes this user owns.
  let uq = indradb::SpecificVertexQuery::single(user.0)
    .outbound()
    .inbound();

  let verts = itr.get_vertices(uq)?;

  let mut res = Vec::new();

  for v in verts {
    println!("v1: {:?}", v);
    if checknote(itr, v.id, search)? {
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
  let ids = tagsearch(itr, user, &search.tagsearch)?;

  println!("ids: {:?}", ids);

  let mut notes = Vec::new();

  for id in ids {
    let zkln = read_zklistnote(itr, systemvs, Some(user), id)?;
    println!("zkln {:?}", zkln);
    notes.push(zkln);
  }

  Ok(ZkNoteSearchResult {
    notes: notes,
    offset: search.offset,
  })
}
