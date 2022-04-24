use crate::util::{is_token_expired, now};
use barrel::backend::Sqlite;
use barrel::{types, Migration};
use crypto_hash::{hex_digest, Algorithm};
use log::info;
use orgauth::dbfun;
use orgauth::dbfun::user_id;
use orgauth::migrations;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::content::{
  Direction, EditLink, ExtraLoginData, GetZkLinks, GetZkNoteComments, GetZkNoteEdit, ImportZkNote,
  SaveZkLink, SaveZkNote, SavedZkNote, ZkLink, ZkNote, ZkNoteEdit,
};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub name: String,
  pub noteid: i64,
  pub homenoteid: Option<i64>,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
}

pub fn new_user(
  dbfile: &Path,
  name: String,
  hashwd: String,
  salt: String,
  email: String,
  registration_key: String,
) -> Result<i64, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let usernoteid = note_id(&conn, "system", "user")?;
  let publicnoteid = note_id(&conn, "system", "public")?;
  let systemid = user_id(&conn, "system")?;

  let now = now()?;

  // make a corresponding note,
  conn.execute(
    "insert into zknote (title, content, user, editable, showtitle, createdate, changeddate)
     values (?1, ?2, ?3, 0, 1, ?4, ?5)",
    params![name, "", systemid, now, now],
  )?;

  let zknid = conn.last_insert_rowid();

  // make a user record.
  conn.execute(
    "insert into user (name, zknote, hashwd, salt, email, registration_key, createdate)
      values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    params![name, zknid, hashwd, salt, email, registration_key, now],
  )?;

  let uid = conn.last_insert_rowid();

  conn.execute(
    "update zknote set sysdata = ?1
        where id = ?2",
    params![systemid, uid.to_string().as_str()],
  )?;

  // indicate a 'user' record, and 'public'
  save_zklink(&conn, zknid, usernoteid, systemid, None)?;
  save_zklink(&conn, zknid, publicnoteid, systemid, None)?;

  Ok(uid)
}

pub fn login_data(conn: &Connection, uid: i64) -> Result<LoginData, Box<dyn Error>> {
  let user = orgauth::dbfun::read_user_by_id(&conn, uid)?;
  Ok(LoginData {
    userid: uid,
    name: user.name,
    zknote: user.noteid,
    homenote: user.homenoteid,
    publicid: note_id(conn, "system", "public")?,
    shareid: note_id(conn, "system", "share")?,
    searchid: note_id(conn, "system", "search")?,
    commentid: note_id(conn, "system", "comment")?,
  })
}

// TODO move to interfaces?
pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<Option<LoginData>, Box<dyn Error>> {
  let conn = connection_open(config.db.as_path())?;
  match session.get("token")? {
    None => Ok(None),
    Some(token) => {
      match dbfun::read_user_by_token(&conn, token, Some(config.login_token_expiration_ms)) {
        Ok(user) => Ok(Some(dbfun::login_data(&conn, user.id)?)),
        Err(_) => Ok(None),
      }
    }
  }
}

pub fn connection_open(dbfile: &Path) -> Result<Connection, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  // conn.busy_timeout(Duration::from_millis(500))?;
  conn.busy_handler(Some(|count| {
    info!("busy_handler: {}", count);
    let d = Duration::from_millis(500);
    std::thread::sleep(d);
    true
  }))?;

  conn.execute("PRAGMA foreign_keys = true;", params![])?;

  Ok(conn)
}

pub fn initialdb() -> Migration {
  let mut m = Migration::new();

  m.create_table("user", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  m.create_table("zk", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false));
    t.add_column("description", types::text().nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  m.create_table("zkmember", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("zk", types::foreign("zk", "id").nullable(false));
  });

  m.create_table("zknote", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("public", types::boolean().nullable(false));
    t.add_column("zk", types::foreign("zk", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  m.create_table("zklink", |t| {
    t.add_column("zkleft", types::foreign("zknote", "id").nullable(false));
    t.add_column("zkright", types::foreign("zknote", "id").nullable(false));
    t.add_column("linkzk", types::foreign("zknote", "id").nullable(true));
    t.add_index("unq", types::index(vec!["zkleft", "zkright"]).unique(true));
  });

  m
}

pub fn udpate1() -> Migration {
  let mut m = Migration::new();

  // can't rename columns in sqlite, so drop and recreate table.
  // shouldn't be any links in the old style one anyway.
  m.drop_table("zklink");

  m.create_table("zklink", |t| {
    t.add_column("fromid", types::foreign("zknote", "id").nullable(false));
    t.add_column("toid", types::foreign("zknote", "id").nullable(false));
    t.add_column("zk", types::foreign("zknote", "id").nullable(true));
    t.add_column("linkzknote", types::foreign("zknote", "id").nullable(true));
    t.add_index(
      "unq",
      types::index(vec!["fromid", "toid", "zk"]).unique(true),
    );
  });

  // table for storing single values.
  m.create_table("singlevalue", |t| {
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("value", types::text().nullable(false));
  });

  m
}

pub fn udpate2() -> Migration {
  let mut m = Migration::new();

  // can't change keys or constraints in sqlite, so drop and recreate table.
  // shouldn't be any links in the old style one anyway.
  m.drop_table("zklink");

  m.create_table("zklink", |t| {
    t.add_column("fromid", types::foreign("zknote", "id").nullable(false));
    t.add_column("toid", types::foreign("zknote", "id").nullable(false));
    t.add_column("zk", types::foreign("zk", "id").nullable(false));
    t.add_column("linkzknote", types::foreign("zknote", "id").nullable(true));
    t.add_index(
      "unq",
      types::index(vec!["fromid", "toid", "zk"]).unique(true),
    );
  });

  m
}

pub fn udpate3(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  // temp table to hold zknote data.
  m1.create_table("zknotetemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("public", types::boolean().nullable(false));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("zk", types::foreign("zk", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, public, pubid, zk, createdate, changeddate)
        select id, title, content, public, null, zk, createdate, changeddate from zknote",
    params![],
  )?;

  let mut m2 = Migration::new();
  // drop zknote.
  m2.drop_table("zknote");

  // new zknote with new column.
  m2.create_table("zknote", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("public", types::boolean().nullable(false));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("zk", types::foreign("zk", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, public, pubid, zk, createdate, changeddate)
        select id, title, content, public, pubid, zk, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();
  // drop zknotetemp.
  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate4(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  // temp table to hold zknote data.
  m1.create_table("zknotetemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    // t.add_column("public", types::boolean().nullable(false));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });
  m1.create_table("zklinktemp", |t| {
    t.add_column("fromid", types::foreign("zknote", "id").nullable(false));
    t.add_column("toid", types::foreign("zknote", "id").nullable(false));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("linkzknote", types::foreign("zknote", "id").nullable(true));
    t.add_index(
      "unqtemp",
      types::index(vec!["fromid", "toid", "user"]).unique(true),
    );
  });
  m1.create_table("usertemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // only user 2 will keep their stuff!  that's me.
  conn.execute("delete from zkmember where user <> 2", params![])?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, pubid, user, createdate, changeddate)
        select zknote.id, title, content, pubid, user.id, zknote.createdate, zknote.changeddate from zknote, user
        where user.id in (select user from zkmember where zkmember.zk = zknote.zk)",
    params![],
  )?;

  // copy everything from zklink.
  conn.execute(
    "insert into zklinktemp (fromid, toid, user)
        select fromid, toid, user.id from zklink, user
        where user.id in (select user from zkmember where zkmember.zk = zklink.zk)",
    params![],
  )?;

  // copy everything from user.
  conn.execute(
    "insert into usertemp (id, name, hashwd, salt, email, registration_key, createdate)
        select id, name, hashwd, salt, email, registration_key, createdate from user",
    params![],
  )?;

  let mut m2 = Migration::new();
  // drop zknote.
  m2.drop_table("zknote");
  m2.drop_table("zkmember");
  m2.drop_table("zk");
  m2.drop_table("zklink");
  m2.drop_table("user");

  // new user table with new column, 'zknote'
  m2.create_table("user", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  // new zknote with column 'user' instead of 'zk'.
  m2.create_table("zknote", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("sysdata", types::text().nullable(true));
    // t.add_column("public", types::boolean().nullable(false));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  // new zklink with column 'user' instead of 'zk'.
  m2.create_table("zklink", |t| {
    t.add_column("fromid", types::foreign("zknote", "id").nullable(false));
    t.add_column("toid", types::foreign("zknote", "id").nullable(false));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("linkzknote", types::foreign("zknote", "id").nullable(true));
    t.add_index(
      "zklinkunq",
      types::index(vec!["fromid", "toid", "user"]).unique(true),
    );
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, pubid, user, createdate, changeddate)
        select id, title, content, pubid, user, createdate, changeddate from zknotetemp",
    params![],
  )?;

  // copy everything from zklinktemp.
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote)
        select fromid, toid, user, linkzknote from zklinktemp",
    params![],
  )?;

  // copy everything from usertemp.
  conn.execute(
    "insert into user (id, name, hashwd, salt, email, registration_key, createdate)
        select id, name, hashwd, salt, email, registration_key, createdate from usertemp",
    params![],
  )?;

  let now = now()?;

  // create system user.
  conn.execute(
    "insert into user (name, hashwd, salt, email, registration_key, createdate)
      values ('system', '', '', '', null, ?1)",
    params![now],
  )?;
  let sysid = conn.last_insert_rowid();

  // system tags.
  conn.execute(
    "insert into zknote (title, content, pubid, user, createdate, changeddate)
      values ('public', '', null, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;

  let publicid = conn.last_insert_rowid();

  conn.execute(
    "insert into zknote (title, content, pubid, user, createdate, changeddate)
      values ('share', '', null, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;

  conn.execute(
    "insert into zknote (title, content, pubid, user, createdate, changeddate)
      values ('search', '', null, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;

  conn.execute(
    "insert into zknote (title, content, pubid, user, createdate, changeddate)
      values ('user', '', null, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;
  let userid = conn.last_insert_rowid();

  // create user zknotes.
  conn.execute(
    "insert into zknote (sysdata, title, content, pubid, user, createdate, changeddate)
       select id, name, '', null, ?1, ?2, ?3 from user",
    params![sysid, now, now],
  )?;

  // ids of notes into user recs.
  conn.execute(
    "update user set zknote = (select id from zknote where title = user.name and zknote.user = ?1)",
    params![sysid],
  )?;

  // link user recs to user.
  conn.execute(
    "insert into zklink (fromid, toid, user)
     select zknote.id, ?1, ?2 from
      zknote, user where zknote.user = ?2 and zknote.title = user.name",
    params![userid, sysid],
  )?;
  // select zknote.id from zknote, user where zknote.user = 8 and zknote.title = user.name;

  // link system recs to public.
  conn.execute(
    "insert into zklink (fromid, toid, user)
     select id, ?1, ?2 from
      zknote where zknote.user = ?2",
    params![publicid, sysid],
  )?;

  let mut m3 = Migration::new();
  // drop zknotetemp.
  m3.drop_table("zknotetemp");
  m3.drop_table("zklinktemp");
  m3.drop_table("usertemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate5(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  m1.create_table("usertemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from user.
  conn.execute(
    "insert into usertemp (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from user",
    params![],
  )?;

  let mut m2 = Migration::new();
  m2.drop_table("user");

  // new user table with new columns for session tokens.
  m2.create_table("user", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("token", types::text().nullable(true));
    t.add_column("tokendate", types::integer().nullable(true));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from usertemp.
  conn.execute(
    "insert into user (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from usertemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("usertemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate6(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  m1.create_table("usertemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from user.
  conn.execute(
    "insert into usertemp (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from user",
    params![],
  )?;

  let mut m2 = Migration::new();
  m2.drop_table("user");

  // new user table WITHOUT columns for session tokens.
  m2.create_table("user", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  // add token table.  multiple tokens per user to support multiple browsers and/or devices.
  m2.create_table("token", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index("tokenunq", types::index(vec!["user", "token"]).unique(true));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from usertemp.
  conn.execute(
    "insert into user (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from usertemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("usertemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate7(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  m1.create_table("zknotetemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("sysdata", types::text().nullable(true));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, createdate, changeddate)
        select id, title, content, null, pubid, user, createdate, changeddate from zknote",
    params![],
  )?;

  let mut m2 = Migration::new();

  m2.drop_table("zknote");

  // new zknote with readonly column
  m2.create_table("zknote", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("sysdata", types::text().nullable(true));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("editable", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, createdate, changeddate)
        select id, title, content, null, pubid, user, 0, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate8(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;
  let now = now()?;

  conn.execute(
    "insert into zknote (title, content, pubid, editable, user,  createdate, changeddate)
      values ('comment', '', null, 0, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;

  let zknid = conn.last_insert_rowid();

  // link system recs to public.
  conn.execute(
    "insert into zklink (fromid, toid, user)
     values (?1, ?2, ?3)",
    params![zknid, pubid, sysid],
  )?;

  Ok(())
}

pub fn udpate9(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  // add newemail table.  each request for a new email creates an entry.
  m1.create_table("newemail", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "newemailunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate10(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  m1.create_table("usertemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from user.
  conn.execute(
    "insert into usertemp (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from user",
    params![],
  )?;

  let mut m2 = Migration::new();
  m2.drop_table("user");

  // new user table with homenote
  m2.create_table("user", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("name", types::text().nullable(false).unique(true));
    t.add_column("hashwd", types::text().nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("homenote", types::foreign("zknote", "id").nullable(true));
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from usertemp.
  conn.execute(
    "insert into user (id, name, hashwd, zknote, salt, email, registration_key, createdate)
        select id, name, hashwd, zknote, salt, email, registration_key, createdate from usertemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("usertemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate11(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  // add newpassword table.  each request for a new password creates an entry.
  m1.create_table("newpassword", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "resetpasswordunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate12(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m1 = Migration::new();

  m1.create_table("zknotetemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("sysdata", types::text().nullable(true));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("editable", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, createdate, changeddate)
        select id, title, content, null, pubid, user, editable, createdate, changeddate from zknote",
    params![],
  )?;

  let mut m2 = Migration::new();

  m2.drop_table("zknote");

  // new zknote with showtitle column
  m2.create_table("zknote", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("title", types::text().nullable(false));
    t.add_column("content", types::text().nullable(false));
    t.add_column("sysdata", types::text().nullable(true));
    t.add_column("pubid", types::text().nullable(true).unique(true));
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, createdate, changeddate)
        select id, title, content, null, pubid, user, 0, 1, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

// --------------------------------------------------------------------------------
// orgauth enters the chat
// --------------------------------------------------------------------------------
pub fn udpate13(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;

  migrations::udpate1(dbfile)?;

  // copy from old user tables into orgauth tables.
  conn.execute(
    "insert into orgauth_user (id, name, hashwd, salt, email, registration_key, createdate)
        select id, name, hashwd, salt, email, registration_key, createdate from user",
    params![],
  )?;

  conn.execute(
    "insert into orgauth_token (user, token, tokendate)
        select user, token, tokendate from token",
    params![],
  )?;

  conn.execute(
    "insert into orgauth_newemail (user, email, token, tokendate)
        select user, email, token, tokendate from newemail",
    params![],
  )?;

  conn.execute(
    "insert into orgauth_newpassword (user, token, tokendate)
        select user, token, tokendate fromi newpassword",
    params![],
  )?;

  // Hmmm would switch to 'zkuser', but I'll keep the user table so I don't have to rebuild every table with
  // new foreign keys.

  let mut m1 = Migration::new();

  m1.create_table("usertemp", |t| {
    t.add_column("id", types::foreign("orgauth_user", "id").nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("homenote", types::foreign("zknote", "id").nullable(true));
  });

  conn.execute_batch(mt.make::<Sqlite>().as_str())?;

  // copy from user.
  conn.execute(
    "insert into usertemp (id, zknote, homenote)
        select id, zknote, homenote from user",
    params![],
  )?;

  let mut m2 = Migration::new();
  m2.drop_table("user");

  // new user table with homenote
  m2.create_table("user", |t| {
    t.add_column("id", types::foreign("orgauth_user", "id").nullable(false));
    t.add_column("zknote", types::foreign("zknote", "id").nullable(true));
    t.add_column("homenote", types::foreign("zknote", "id").nullable(true));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from usertemp.
  conn.execute(
    "insert into user (id, zknote, homenote)
        select id, zknote, homenote from usertemp",
    params![],
  )?;

  let mut m3 = Migration::new();
  m3.drop_table("usertemp");
  m3.drop_table("token");
  m3.drop_table("newemail");
  m3.drop_table("newpassword");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn get_single_value(conn: &Connection, name: &str) -> Result<Option<String>, Box<dyn Error>> {
  match conn.query_row(
    "select value from singlevalue where name = ?1",
    params![name],
    |row| Ok(row.get(0)?),
  ) {
    Ok(v) => Ok(Some(v)),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
    Err(x) => Err(Box::new(x)),
  }
}

pub fn set_single_value(conn: &Connection, name: &str, value: &str) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "insert into singlevalue (name, value) values (?1, ?2)
        on conflict (name) do update set value = ?2 where name = ?1",
    params![name, value],
  )?;
  Ok(())
}

pub fn dbinit(dbfile: &Path, token_expiration_ms: i64) -> Result<(), Box<dyn Error>> {
  let exists = dbfile.exists();

  let conn = connection_open(dbfile)?;

  if !exists {
    info!("initialdb");
    conn.execute_batch(initialdb().make::<Sqlite>().as_str())?;
  }

  let nlevel = match get_single_value(&conn, "migration_level") {
    Err(_) => 0,
    Ok(None) => 0,
    Ok(Some(level)) => {
      let l = level.parse::<i32>()?;
      l
    }
  };

  if nlevel < 1 {
    info!("udpate1");
    conn.execute_batch(udpate1().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "1")?;
  }

  if nlevel < 2 {
    info!("udpate2");
    conn.execute_batch(udpate2().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "2")?;
  }
  if nlevel < 3 {
    info!("udpate3");
    udpate3(&dbfile)?;
    set_single_value(&conn, "migration_level", "3")?;
  }
  if nlevel < 4 {
    info!("udpate4");
    udpate4(&dbfile)?;
    set_single_value(&conn, "migration_level", "4")?;
  }
  if nlevel < 5 {
    info!("udpate5");
    udpate5(&dbfile)?;
    set_single_value(&conn, "migration_level", "5")?;
  }
  if nlevel < 6 {
    info!("udpate6");
    udpate6(&dbfile)?;
    set_single_value(&conn, "migration_level", "6")?;
  }
  if nlevel < 7 {
    info!("udpate7");
    udpate7(&dbfile)?;
    set_single_value(&conn, "migration_level", "7")?;
  }
  if nlevel < 8 {
    info!("udpate8");
    udpate8(&dbfile)?;
    set_single_value(&conn, "migration_level", "8")?;
  }
  if nlevel < 9 {
    info!("udpate9");
    udpate9(&dbfile)?;
    set_single_value(&conn, "migration_level", "9")?;
  }
  if nlevel < 10 {
    info!("udpate10");
    udpate10(&dbfile)?;
    set_single_value(&conn, "migration_level", "10")?;
  }
  if nlevel < 11 {
    info!("udpate11");
    udpate11(&dbfile)?;
    set_single_value(&conn, "migration_level", "11")?;
  }
  if nlevel < 12 {
    info!("udpate12");
    udpate12(&dbfile)?;
    set_single_value(&conn, "migration_level", "12")?;
  }
  if nlevel < 11 {
    info!("udpate11");
    udpate11(&dbfile)?;
    set_single_value(&conn, "migration_level", "11")?;
  }

  info!("db up to date.");

  orgauth::dbfun::purge_login_tokens(&conn, token_expiration_ms)?;

  Ok(())
}

// user CRUD

pub fn save_zklink(
  conn: &Connection,
  fromid: i64,
  toid: i64,
  user: i64,
  linkzknote: Option<i64>,
) -> Result<i64, Box<dyn Error>> {
  // ok to link to notes you don't own.
  // can link between notes you don't own, even.
  // but linking from a note you don't own to 'share' or 'public' is not allowed.
  // if one note is a share, then you must be a member of that share to link, or own that share.

  let shareid = note_id(&conn, "system", "share")?;
  let publicid = note_id(&conn, "system", "public")?;
  let usernote = user_note_id(&conn, user)?;

  let authed = if fromid == shareid || fromid == publicid || fromid == usernote {
    // can't link non-me notes to shareid or public or usernote.
    let izm = is_zknote_mine(&conn, toid, user)?;
    izm
  } else if toid == shareid || toid == publicid || toid == usernote {
    // can't link non-me notes to shareid or public or usernote.
    let izm = is_zknote_mine(&conn, fromid, user)?;
    izm
  } else if are_notes_linked(&conn, fromid, shareid)? && !are_notes_linked(&conn, usernote, fromid)?
  {
    // fromid is a share.
    // user does not link to it.
    // not allowed!
    is_zknote_mine(&conn, fromid, user)? // unless user owns fromid.
  } else if are_notes_linked(&conn, toid, shareid)? && !are_notes_linked(&conn, usernote, toid)? {
    // fromid is a share.
    // user does not link to it.
    // not allowed!
    is_zknote_mine(&conn, toid, user)? // unless user owns toid.
  } else {
    true
  };

  // yeesh.  doing this to exit with ? instead of having a big if-then to the end.
  let orwat: Result<(), Box<dyn Error>> = if authed {
    Ok(())
  } else {
    bail!("link not allowed")
  };
  let _wat = orwat?;

  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
      on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
    params![fromid, toid, user, linkzknote],
  )?;

  Ok(conn.last_insert_rowid())
}

pub fn note_id(conn: &Connection, name: &str, title: &str) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select zknote.id from
      zknote, user
      where zknote.title = ?2
      and user.name = ?1
      and zknote.user = user.id",
    params![name, title],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn note_id2(conn: &Connection, uid: i64, title: &str) -> Result<Option<i64>, Box<dyn Error>> {
  match conn.query_row(
    "select zknote.id from
      zknote
      where zknote.title = ?2
      and zknote.user = ?1",
    params![uid, title],
    |row| Ok(row.get(0)?),
  ) {
    Err(e) => match e {
      rusqlite::Error::QueryReturnedNoRows => Ok(None),
      _ => Err(Box::new(e)),
    },
    Ok(i) => Ok(i),
  }
}

pub fn user_note_id(conn: &Connection, uid: i64) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select zknote from user
      where user.id = ?1",
    params![uid],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn user_shares(conn: &Connection, uid: i64) -> Result<Vec<i64>, Box<dyn Error>> {
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = user_note_id(&conn, uid)?;

  // user shares!
  //   looking for notes that link to 'shareid' and link to 'usernoteid'
  let mut pstmt = conn.prepare(
    "select A.fromid from zklink A, zklink B
      where A.toid = ?1 and
        ((A.fromid = B.fromid and B.toid = ?2) or
         (A.fromid = B.toid and B.fromid = ?2))
     union
    select A.toid from zklink A, zklink B
      where A.fromid = ?1 and
        ((A.toid = B.fromid and B.toid = ?2) or
         (A.toid = B.toid and B.fromid = ?2))
      ",
  )?;
  let r = pstmt
    .query_map(params![shareid, usernoteid], |row| Ok(row.get(0)?))?
    .filter_map(|x| x.ok())
    .collect();
  Ok(r)
}

// is there a connection between this note and uid's user note?
pub fn is_zknote_usershared(
  conn: &Connection,
  zknoteid: i64,
  uid: i64,
) -> Result<bool, Box<dyn Error>> {
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  let ret = are_notes_linked(&conn, zknoteid, usernoteid)?;

  Ok(ret)
}

pub fn is_zknote_shared(
  conn: &Connection,
  zknoteid: i64,
  uid: i64,
) -> Result<bool, Box<dyn Error>> {
  let shareid: i64 = note_id(conn, "system", "share")?;
  let publicid: i64 = note_id(conn, "system", "public")?;
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  // does note link to a note that links to share?
  // and does that note link to usernoteid?
  let ret = match conn.query_row(
    "select count(*)
      from zklink L,
        (select U.toid id
            from zklink U, zklink V
            where
              ((U.fromid = ?3 and U.toid = V.fromid and V.toid = ?2) or
               (U.fromid = ?3 and U.toid = V.toid and V.fromid = ?2)) and
              U.toid != ?4
         union
         select U.fromid id
            from zklink U, zklink V
            where
              ((U.toid = ?3 and U.fromid = V.fromid and V.toid = ?2) or
               (U.toid = ?3 and U.fromid = V.toid and V.fromid = ?2)) and
              U.toid != ?4 ) shares
      where
        (L.fromid = shares.id and L.toid = ?1) or
        (L.toid = shares.id and L.fromid = ?1)",
    params![zknoteid, shareid, usernoteid, publicid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(Box::new(x)),
  };

  Ok(ret?)
}

pub fn is_zknote_public(conn: &Connection, zknoteid: i64) -> Result<bool, Box<dyn Error>> {
  let pubid: i64 = note_id(conn, "system", "public")?;
  match conn.query_row(
    "select count(*) from
      zklink, zknote R
      where (zklink.fromid = ?1 and zklink.toid = ?2)
      or (zklink.fromid = ?2  and zklink.toid = ?1)",
    params![zknoteid, pubid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(Box::new(x)),
  }
}

pub fn is_zknote_mine(
  conn: &Connection,
  zknoteid: i64,
  userid: i64,
) -> Result<bool, Box<dyn Error>> {
  match conn.query_row(
    "select count(*) from
      zknote 
      where id = ?1 and user = ?2",
    params![zknoteid, userid],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(Box::new(x)),
  }
}

pub fn are_notes_linked(conn: &Connection, nid1: i64, nid2: i64) -> Result<bool, Box<dyn Error>> {
  match conn.query_row(
    "select count(*) from
      zklink
      where (fromid = ?1 and toid = ?2)
      or (toid = ?1 and fromid = ?2)",
    params![nid1, nid2],
    |row| {
      let i: i64 = row.get(0)?;
      Ok(i)
    },
  ) {
    Ok(count) => Ok(count > 0),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(Box::new(x)),
  }
}

// zknote CRUD

pub fn save_zknote(
  conn: &Connection,
  uid: i64,
  note: &SaveZkNote,
) -> Result<SavedZkNote, Box<dyn Error>> {
  let now = now()?;

  match note.id {
    Some(id) => {
      // existing note.  update IF mine.
      match conn.execute(
        "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4, editable = ?5, showtitle = ?6
         where id = ?7 and user = ?8",
        params![
          note.title,
          note.content,
          now,
          note.pubid,
          note.editable,
          note.showtitle,
          note.id,
          uid
        ],
      ) {
        Ok(1) => Ok(SavedZkNote {
          id: id,
          changeddate: now,
        }),
        Ok(0) => {
          match zknote_access_id(conn, Some(uid), id)? {
            Access::ReadWrite => {
              // update other user's record!  editable flag must be true.
              conn.execute(
                "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4, showtitle = ?5
                 where id = ?6 and editable = 1",
                params![note.title, note.content, now, note.pubid, note.showtitle, id],
              )?;
              Ok(SavedZkNote {
                id: id,
                changeddate: now,
              })
            }
            _ => bail!("can't update; note is not writable"),
          }
        }
        Ok(_) => bail!("unexpected update success!"),
        Err(e) => Err(e)?,
      }
    }
    None => {
      // new note!
      conn.execute(
        "insert into zknote (title, content, user, pubid, editable, showtitle, createdate, changeddate)
         values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
          note.title,
          note.content,
          uid,
          note.pubid,
          note.editable,
          note.showtitle,
          now,
          now
        ],
      )?;
      Ok(SavedZkNote {
        id: conn.last_insert_rowid(),
        changeddate: now,
      })
    }
  }
}

pub fn get_sysids(conn: &Connection, sysid: i64, noteid: i64) -> Result<Vec<i64>, rusqlite::Error> {
  let mut pstmt = conn.prepare(
    // return system notes that are linked TO by noteid.
    "select A.toid
       from zklink A, zknote N
      where
       (A.fromid = ?1 and A.toid = N.id and N.user = ?2)",
  )?;

  let r = Ok(
    pstmt
      .query_map(params![noteid, sysid], |row| Ok(row.get(0)?))?
      .filter_map(|x| x.ok())
      .collect(),
  );

  r
}

pub fn read_zknote(conn: &Connection, uid: Option<i64>, id: i64) -> Result<ZkNote, Box<dyn Error>> {
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, id)?;

  let mut note = conn.query_row(
    "select ZN.title, ZN.content, ZN.user, U.name, U.zknote, ZN.pubid, ZN.editable, ZN.showtitle, ZN.createdate, ZN.changeddate
      from zknote ZN, user U where ZN.id = ?1 and U.id = ZN.user",
    params![id],
    |row| {
      Ok(ZkNote {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        user: row.get(2)?,
        username: row.get(3)?,
        usernote: row.get(4)?,
        pubid: row.get(5)?,
        editable: row.get(6)?,                // editable same as editableValue!
        editableValue: row.get(6)?,           // <--- same index.
        showtitle: row.get(7)?,
        createdate: row.get(8)?,
        changeddate: row.get(9)?,
        sysids: sysids,
      })
    },
  )?;

  match zknote_access(conn, uid, &note) {
    Ok(zna) => match zna {
      Access::ReadWrite => {
        note.editable = true;
        Ok(note)
      }
      Access::Read => {
        note.editable = false;
        Ok(note)
      }
      Access::Private => Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "can't read zknote; note is private",
      ))),
      // bail!("can't read zknote; note is private"),
    },
    Err(e) => Err(e),
  }
}

#[derive(Debug)]
pub enum Access {
  Private,
  Read,
  ReadWrite,
}

pub fn zknote_access(
  conn: &Connection,
  uid: Option<i64>,
  note: &ZkNote,
) -> Result<Access, Box<dyn Error>> {
  match uid {
    Some(uid) => {
      if uid == note.user {
        Ok(Access::ReadWrite)
      } else if is_zknote_usershared(conn, note.id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_shared(conn, note.id, uid)? {
        // editable and accessible.
        if note.editable {
          Ok(Access::ReadWrite)
        } else {
          Ok(Access::Read)
        }
      } else if is_zknote_public(conn, note.id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
    None => {
      if is_zknote_public(conn, note.id)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
  }
}

pub fn zknote_access_id(
  conn: &Connection,
  uid: Option<i64>,
  noteid: i64,
) -> Result<Access, Box<dyn Error>> {
  match uid {
    Some(uid) => {
      if is_zknote_mine(&conn, noteid, uid)? {
        Ok(Access::ReadWrite)
      } else if is_zknote_usershared(conn, noteid, uid)? {
        // editable and accessible.
        Ok(Access::ReadWrite)
      } else if is_zknote_shared(conn, noteid, uid)? {
        // editable and accessible.
        Ok(Access::ReadWrite)
      } else if is_zknote_public(conn, noteid)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
    None => {
      if is_zknote_public(conn, noteid)? {
        // accessible but not editable.
        Ok(Access::Read)
      } else {
        Ok(Access::Private)
      }
    }
  }
}

pub fn read_zknotepubid(
  conn: &Connection,
  uid: Option<i64>,
  pubid: &str,
) -> Result<ZkNote, Box<dyn Error>> {
  let publicid = note_id(&conn, "system", "public")?;
  let mut note = conn.query_row(
    "select A.id, A.title, A.content, A.user, U.name, U.zknote, A.pubid, A.editable, A.showtitle, A.createdate, A.changeddate
      from zknote A, user U, zklink L where A.pubid = ?1
      and ((A.id = L.fromid
      and L.toid = ?2) or (A.id = L.toid
      and L.fromid = ?2))
      and U.id = A.user",
    params![pubid, publicid],
    |row| {
      Ok(ZkNote {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        user: row.get(3)?,
        username: row.get(4)?,
        usernote: row.get(5)?,
        pubid: row.get(6)?,
        editable: false,
        editableValue: row.get(7)?,
        showtitle: row.get(8)?,
        createdate: row.get(9)?,
        changeddate: row.get(10)?,
        sysids: Vec::new(),
      })
    },
  )?;
  let sysid = user_id(&conn, "system")?;
  let sysids = get_sysids(conn, sysid, note.id)?;

  note.sysids = sysids;

  match zknote_access(conn, uid, &note) {
    Ok(zna) => match zna {
      Access::ReadWrite => {
        note.editable = true;
        Ok(note)
      }
      Access::Read => {
        note.editable = false;
        Ok(note)
      }
      Access::Private => Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "can't read zknote; note is private",
      ))),
    },
    Err(e) => Err(e),
  }
}

// delete the note; fails if there are links to it.
pub fn delete_zknote(dbfile: &Path, uid: i64, noteid: i64) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  // only delete when user is the owner.
  conn.execute(
    "delete from zknote where id = ?1 
      and user = ?2",
    params![noteid, uid],
  )?;

  Ok(())
}

// delete the note AND any links to it.
pub fn power_delete_zknote(conn: &Connection, uid: i64, noteid: i64) -> Result<(), Box<dyn Error>> {
  // only delete when user owns the links.
  conn.execute(
    "delete from zklink where
      user = ?2
      and (fromid = ?1 or toid = ?1)",
    params![noteid, uid],
  )?;

  // only delete when user is in the zk
  conn.execute(
    "delete from zknote where id = ?1
      and user = ?2",
    params![noteid, uid],
  )?;

  Ok(())
}

pub fn save_zklinks(dbfile: &Path, uid: i64, zklinks: Vec<ZkLink>) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  for zklink in zklinks.iter() {
    if zklink.user == uid {
      if zklink.delete == Some(true) {
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![zklink.from, zklink.to, uid],
        )?;
      } else {
        save_zklink(&conn, zklink.from, zklink.to, uid, zklink.linkzknote)?;
      }
    }
  }

  Ok(())
}

pub fn save_savezklinks(
  conn: &Connection,
  uid: i64,
  zknid: i64,
  zklinks: Vec<SaveZkLink>,
) -> Result<(), Box<dyn Error>> {
  for link in zklinks.iter() {
    let (from, to) = match link.direction {
      Direction::From => (link.otherid, zknid),
      Direction::To => (zknid, link.otherid),
    };
    if link.user == uid {
      if link.delete == Some(true) {
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![from, to, uid],
        )?;
      } else {
        save_zklink(&conn, from, to, uid, link.zknote)?;
      }
    }
  }

  Ok(())
}

pub fn read_zklinks(
  conn: &Connection,
  uid: i64,
  gzl: &GetZkLinks,
) -> Result<Vec<EditLink>, Box<dyn Error>> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;

  let usershares = user_shares(&conn, uid)?;

  let unid = user_note_id(&conn, uid)?;

  // user shares in '1,3,4,5,6' form (minus the quotes!)
  let mut s = usershares
    .iter()
    .map(|x| {
      let mut s = x.to_string();
      s.push_str(",");
      s
    })
    .collect::<String>();
  s.truncate(s.len() - 1);

  // good old fashioned string templating here, since I can't figure out how to
  // do array parameters.
  //
  // zklinks that are mine.
  // +
  // not-mine zklinks with from = this note and toid = note that ISA public.
  // +
  // not-mine zklinks with from = note that is public, and to = this.
  // +
  // not-mine zklinks with from/to = this, and to/from in usershares.
  // +
  // not-mine zklinks from/to notes that link to my usernote.

  let sqlstr = format!(
    "select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user = ?1 and (A.fromid = ?2 or A.toid = ?2)
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.fromid = ?2
      and B.fromid = A.toid
      and B.toid = ?3
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
      from zklink A, zklink B
      inner join zknote as L ON A.fromid = L.id
      inner join zknote as R ON A.toid = R.id
      where A.user != ?1 and A.toid = ?2
      and B.fromid = A.fromid
      and B.toid = ?3
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
        from zklink A, zklink B
        inner join zknote as L ON A.fromid = L.id
        inner join zknote as R ON A.toid = R.id
        where A.user != ?1 and
          ((A.toid = ?2 and A.fromid = B.fromid and B.toid in ({})) or
           (A.toid = ?2 and A.fromid = B.toid and B.fromid in ({})) or
           (A.fromid = ?2 and A.toid = B.fromid and B.toid in ({})) or
           (A.fromid = ?2 and A.toid = B.toid and B.fromid in ({})))
      union
    select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
        from zklink A, zklink B
        inner join zknote as L ON A.fromid = L.id
        inner join zknote as R ON A.toid = R.id
        where A.user != ?1 and
          ((A.toid = ?2 and A.fromid = B.fromid and B.toid = ?4) or
           (A.toid = ?2 and A.fromid = B.toid and B.fromid = ?4) or
           (A.fromid = ?2 and A.toid = B.fromid and B.toid = ?4) or
           (A.fromid = ?2 and A.toid = B.toid and B.fromid = ?4)) ",
    s, s, s, s
  );

  let mut pstmt = conn.prepare(sqlstr.as_str())?;
  let r = Ok(
    pstmt
      .query_map(params![uid, gzl.zknote, pubid, unid], |row| {
        let fromid = row.get(0)?;
        let toid = row.get(1)?;
        let otherid = if fromid == gzl.zknote { toid } else { fromid };
        let sysids = get_sysids(&conn, sysid, otherid)?;
        Ok(EditLink {
          otherid: otherid,
          direction: if fromid == gzl.zknote {
            Direction::To
          } else {
            Direction::From
          },
          user: row.get(2)?,
          zknote: row.get(3)?,
          othername: if fromid == gzl.zknote {
            row.get(5)?
          } else {
            row.get(4)?
          },
          sysids: sysids,
        })
      })?
      .filter_map(|x| x.ok())
      .collect(),
  );
  r
}

pub fn read_public_zklinks(
  conn: &Connection,
  noteid: i64,
) -> Result<Vec<EditLink>, Box<dyn Error>> {
  let pubid = note_id(&conn, "system", "public")?;
  let sysid = user_id(&conn, "system")?;

  let mut pstmt = conn.prepare(
    // return zklinks that link to or from notes that link to 'public'.
    "select A.fromid, A.toid, A.user, A.linkzknote, L.title, R.title
       from zklink A, zklink B
       inner join zknote as L ON A.fromid = L.id
       inner join zknote as R ON A.toid = R.id
     where
       (L.user != ?3 and R.user != ?3) and
       ((A.toid = ?1 and A.fromid = B.fromid and B.toid = ?2) or
        (A.fromid = ?1 and A.toid = B.toid and B.fromid = ?2) or
        (A.fromid = ?1 and A.toid = B.fromid and B.toid = ?2))",
  )?;

  let r = Ok(
    pstmt
      .query_map(params![noteid, pubid, sysid], |row| {
        let fromid = row.get(0)?;
        let toid = row.get(1)?;
        let otherid = if fromid == noteid { toid } else { fromid };
        Ok(EditLink {
          otherid: otherid,
          direction: if fromid == noteid {
            Direction::To
          } else {
            Direction::From
          },
          user: row.get(2)?,
          zknote: row.get(3)?,
          othername: if fromid == noteid {
            row.get(5)?
          } else {
            row.get(4)?
          },
          sysids: get_sysids(&conn, sysid, otherid)?,
        })
      })?
      .filter_map(|x| x.ok())
      .collect(),
  );
  r
}

pub fn read_zknotecomments(
  conn: &Connection,
  uid: i64,
  gznc: &GetZkNoteComments,
) -> Result<Vec<ZkNote>, Box<dyn Error>> {
  let cid = note_id(&conn, "system", "comment")?;

  // notes with a TO link to our note
  // and a TO link to 'comment'
  let mut stmt = conn.prepare(
    "select N.fromid from  zklink C, zklink N
      where N.fromid = C.fromid
      and N.toid = ?1 and C.toid = ?2",
  )?;
  let c_iter = stmt.query_map(params![gznc.zknote, cid], |row| Ok(row.get(0)?))?;

  let mut nv = Vec::new();

  for id in c_iter {
    match id {
      Ok(id) => match read_zknote(&conn, Some(uid), id) {
        Ok(note) => {
          nv.push(note);
          match gznc.limit {
            Some(l) => {
              if nv.len() >= l as usize {
                break;
              }
            }
            None => (),
          }
        }
        Err(_) => (),
      },
      Err(_) => (),
    }
  }

  Ok(nv)
}

pub fn read_zknoteedit(
  conn: &Connection,
  uid: i64,
  gzl: &GetZkNoteEdit,
) -> Result<ZkNoteEdit, Box<dyn Error>> {
  // should do an ownership check for us
  let zknote = read_zknote(conn, Some(uid), gzl.zknote)?;

  let zklinks = read_zklinks(conn, uid, &GetZkLinks { zknote: zknote.id })?;

  Ok(ZkNoteEdit {
    zknote: zknote,
    links: zklinks,
  })
}

pub fn save_importzknotes(
  conn: &Connection,
  uid: i64,
  izns: Vec<ImportZkNote>,
) -> Result<(), Box<dyn Error>> {
  for izn in izns.iter() {
    // create the note if it doesn't exist.
    let nid = match note_id2(&conn, uid, izn.title.as_str())? {
      Some(i) => {
        // update the content.
        conn.execute(
          "update zknote set content = ?1 where
            user = ?2 and id = ?3",
          params![izn.content, uid, i],
        )?;

        i
      }
      None => {
        // new note.
        save_zknote(
          &conn,
          uid,
          &SaveZkNote {
            id: None,
            title: izn.title.clone(),
            pubid: None,
            content: izn.content.clone(),
            editable: false,
            showtitle: true,
          },
        )?
        .id
      }
    };
    // now add the 'from' links.
    for title in izn.fromLinks.iter() {
      // if the 'from' note doesn't exist, create it.
      let fromid = match note_id2(&conn, uid, title)? {
        Some(n) => n,
        None => {
          // new note.
          save_zknote(
            &conn,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
            },
          )?
          .id
        }
      };

      // save link.
      save_zklink(&conn, fromid, nid, uid, None)?;
    }
    // add the 'to' links (and their notes)
    for title in izn.toLinks.iter() {
      let toid = match note_id2(&conn, uid, title)? {
        Some(n) => n,
        None => {
          // new note.
          save_zknote(
            &conn,
            uid,
            &SaveZkNote {
              id: None,
              title: title.clone(),
              pubid: None,
              content: "".to_string(),
              editable: false,
              showtitle: true,
            },
          )?
          .id
        }
      };

      // save link.
      save_zklink(&conn, nid, toid, uid, None)?;
    }
  }

  Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ZkDatabase {
  notes: Vec<ZkNote>,
  links: Vec<ZkLink>,
  users: Vec<User>,
}

pub fn export_db(dbfile: &Path) -> Result<ZkDatabase, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;
  let sysid = user_id(&conn, "system")?;

  // Users
  let mut ustmt = conn.prepare(
    "select id, name, zknote, homenote, hashwd, salt, email, registration_key
      from user",
  )?;

  let u_iter = ustmt.query_map(params![], |row| {
    Ok(User {
      id: row.get(0)?,
      name: row.get(1)?,
      noteid: row.get(2)?,
      homenoteid: row.get(3)?,
      hashwd: row.get(4)?,
      salt: row.get(5)?,
      email: row.get(6)?,
      registration_key: row.get(7)?,
    })
  })?;

  let uv = u_iter.filter_map(|x| x.ok()).collect();

  // Notes
  let mut nstmt = conn.prepare(
    "select ZN.id, ZN.title, ZN.content, ZN.user, U.name, U.zknote, ZN.pubid, ZN.editable, ZN.createdate, ZN.changeddate
      from zknote ZN, user U
      where ZN.user == U.id",
  )?;

  let n_iter = nstmt.query_map(params![], |row| {
    let sysids = get_sysids(&conn, sysid, row.get(0)?)?;
    Ok(ZkNote {
      id: row.get(0)?,
      title: row.get(1)?,
      content: row.get(2)?,
      user: row.get(3)?,
      username: row.get(4)?,
      usernote: row.get(5)?,
      pubid: row.get(6)?,
      editable: false,
      editableValue: row.get(7)?,
      showtitle: row.get(8)?,
      createdate: row.get(9)?,
      changeddate: row.get(10)?,
      sysids: sysids,
    })
  })?;

  let nv = n_iter.filter_map(|x| x.ok()).collect();

  // Links
  let mut lstmt = conn.prepare(
    "select A.fromid, A.toid, A.user, A.linkzknote 
      from zklink A",
  )?;

  let l_iter = lstmt.query_map(params![], |row| {
    Ok(ZkLink {
      from: row.get(0)?,
      to: row.get(1)?,
      user: row.get(2)?,
      delete: None,
      linkzknote: row.get(3)?,
      fromname: None,
      toname: None,
    })
  })?;

  let lv = l_iter.filter_map(|x| x.ok()).collect();

  Ok(ZkDatabase {
    notes: nv,
    links: lv,
    users: uv,
  })
}
