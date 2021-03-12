use crate::util::{is_token_expired, now};
use barrel::backend::Sqlite;
use barrel::{types, Migration};
use log::{debug, error, info, log_enabled, Level};
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use zkprotocol::content::{
  Direction, GetZkLinks, GetZkNoteEdit, ImportZkNote, LoginData, SaveZkLink, SaveZkNote,
  SavedZkNote, ZkLink, ZkNote, ZkNoteEdit,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
  pub token: Option<Uuid>,
  pub tokendate: Option<i64>,
}

pub fn login_data(conn: &Connection, uid: i64) -> Result<LoginData, Box<dyn Error>> {
  Ok(LoginData {
    userid: uid,
    name: user_name(&conn, uid)?,
    publicid: note_id(conn, "system", "public")?,
    shareid: note_id(conn, "system", "share")?,
    searchid: note_id(conn, "system", "search")?,
  })
}

pub fn uid_for_token(conn: &Connection, token: Uuid) -> Result<i64, Box<dyn Error>> {
  let r = conn.query_row(
    "select id
          from user where token = ?1",
    params![token.to_string()],
    |row| Ok(row.get(0)?),
  )?;
  Ok(r)
}

pub fn connection_open(dbfile: &Path) -> rusqlite::Result<Connection> {
  let conn = Connection::open(dbfile)?;

  // conn.busy_timeout(Duration::from_millis(500))?;
  conn.busy_handler(Some(|count| {
    println!("busy_handler: {}", count);
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

  let now = now()?;

  let mut m3 = Migration::new();

  m3.drop_table("usertemp");

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

pub fn dbinit(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  let exists = dbfile.exists();

  let conn = connection_open(dbfile)?;

  if !exists {
    println!("initialdb");
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
    println!("udpate1");
    conn.execute_batch(udpate1().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "1")?;
  }

  if nlevel < 2 {
    println!("udpate2");
    conn.execute_batch(udpate2().make::<Sqlite>().as_str())?;
    set_single_value(&conn, "migration_level", "2")?;
  }
  if nlevel < 3 {
    println!("udpate3");
    udpate3(&dbfile)?;
    set_single_value(&conn, "migration_level", "3")?;
  }
  if nlevel < 4 {
    println!("udpate4");
    udpate4(&dbfile)?;
    set_single_value(&conn, "migration_level", "4")?;
  }
  if nlevel < 5 {
    println!("udpate5");
    udpate5(&dbfile)?;
    set_single_value(&conn, "migration_level", "5")?;
  }

  println!("db up to date.");

  Ok(())
}

// user CRUD

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
    "insert into zknote (title, content, user, createdate, changeddate)
     values (?1, ?2, ?3, ?4, ?5)",
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

pub fn save_zklink(
  conn: &Connection,
  fromid: i64,
  toid: i64,
  user: i64,
  linkzknote: Option<i64>,
) -> Result<i64, Box<dyn Error>> {
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
      on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
    params![fromid, toid, user, linkzknote],
  )?;

  Ok(conn.last_insert_rowid())
}

pub fn user_name(conn: &Connection, uid: i64) -> Result<String, Box<dyn Error>> {
  let user = conn.query_row(
    "select name
      from user where id = ?1",
    params![uid],
    |row| Ok(row.get(0)?),
  )?;

  Ok(user)
}

pub fn read_user(
  dbfile: &Path,
  name: &str,
  token_expiration_ms: Option<i64>,
) -> Result<User, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let (mut user, tokestr) = conn.query_row(
    "select id, hashwd, salt, email, registration_key, token, tokendate
      from user where name = ?1",
    params![name],
    |row| {
      Ok((
        User {
          id: row.get(0)?,
          name: name.to_string(),
          hashwd: row.get(1)?,
          salt: row.get(2)?,
          email: row.get(3)?,
          registration_key: row.get(4)?,
          token: None,
          tokendate: row.get(6)?,
        },
        row.get::<usize, Option<String>>(5)?,
      ))
    },
  )?;

  user.token = match tokestr {
    Some(s) => Some(Uuid::parse_str(s.as_str())?),
    None => None,
  };

  // if expiration supplied, check for token expiration.
  match token_expiration_ms {
    Some(texp) => {
      if user
        .tokendate
        .map(|td| is_token_expired(texp, td))
        .unwrap_or(true)
      {
        bail!("login expired")
      } else {
        Ok(user)
      }
    }
    None => Ok(user),
  }
}

pub fn read_user_by_token(
  conn: &Connection,
  token: Uuid,
  token_expiration_ms: Option<i64>,
) -> Result<User, Box<dyn Error>> {
  let user = conn.query_row(
    "select id, name, hashwd, salt, email, registration_key, tokendate
      from user where token = ?1",
    params![token.to_string()],
    |row| {
      Ok(User {
        id: row.get(0)?,
        name: row.get(1)?,
        hashwd: row.get(2)?,
        salt: row.get(3)?,
        email: row.get(4)?,
        registration_key: row.get(5)?,
        token: Some(token),
        tokendate: row.get(6)?,
      })
    },
  )?;

  match token_expiration_ms {
    Some(texp) => {
      if user
        .tokendate
        .map(|td| is_token_expired(texp, td))
        .unwrap_or(true)
      {
        bail!("login expired")
      } else {
        Ok(user)
      }
    }
    None => Ok(user),
  }
}

pub fn update_user(dbfile: &Path, user: &User) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  match (user.token, user.tokendate) {
    (Some(token), Some(tokendate)) => {
      conn.execute(
        "update user set name = ?1, hashwd = ?2, salt = ?3, email = ?4, registration_key = ?5,
            token = ?6, tokendate = ?7
           where id = ?8",
        params![
          user.name,
          user.hashwd,
          user.salt,
          user.email,
          user.registration_key,
          token.to_string(),
          tokendate,
          user.id,
        ],
      )?;
    }
    _ => {
      conn.execute(
        "update user set name = ?1, hashwd = ?2, salt = ?3, email = ?4, registration_key = ?5,
           where id = ?6",
        params![
          user.name,
          user.hashwd,
          user.salt,
          user.email,
          user.registration_key,
          user.id,
        ],
      )?;
    }
  };

  Ok(())
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

pub fn user_id(conn: &Connection, name: &str) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select id from user
      where user.name = ?1",
    params![name],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
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
  let rec_iter = pstmt.query_map(params![shareid, usernoteid], |row| Ok(row.get(0)?))?;
  let mut pv = Vec::new();

  for rsrec in rec_iter {
    match rsrec {
      Ok(rec) => {
        pv.push(rec);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
}

pub fn is_zknote_shared(
  conn: &Connection,
  zknoteid: i64,
  uid: i64,
) -> Result<bool, Box<dyn Error>> {
  let shareid: i64 = note_id(conn, "system", "share")?;
  let usernoteid: i64 = user_note_id(&conn, uid)?;

  match conn.query_row(
    "select count(*) from
      zklink L, zklink M, zklink U where
        (L.fromid = ?1 and L.toid = M.fromid and M.toid = ?2 and
        ((U.fromid = ?3 and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?3)))
        or (L.fromid = ?1 and L.toid = ?3 or L.toid = ?3 and L.fromid = ?1)",
    params![zknoteid, shareid, usernoteid],
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

// zknote CRUD

pub fn save_zknote(
  conn: &Connection,
  uid: i64,
  note: &SaveZkNote,
) -> Result<SavedZkNote, Box<dyn Error>> {
  let now = now()?;

  match note.id {
    Some(id) => {
      conn.execute(
        "update zknote set title = ?1, content = ?2, changeddate = ?3, pubid = ?4
         where id = ?5 and user = ?6",
        params![note.title, note.content, now, note.pubid, note.id, uid],
      )?;
      Ok(SavedZkNote {
        id: id,
        changeddate: now,
      })
    }
    None => {
      conn.execute(
        "insert into zknote (title, content, user, pubid, createdate, changeddate)
         values (?1, ?2, ?3, ?4, ?5, ?6)",
        params![note.title, note.content, uid, note.pubid, now, now],
      )?;
      Ok(SavedZkNote {
        id: conn.last_insert_rowid(),
        changeddate: now,
      })
    }
  }
}

pub fn read_zknote(conn: &Connection, uid: Option<i64>, id: i64) -> Result<ZkNote, Box<dyn Error>> {
  let note = conn.query_row(
    "select ZN.title, ZN.content, ZN.user, U.name, ZN.pubid, ZN.createdate, ZN.changeddate
      from zknote ZN, user U where ZN.id = ?1 and U.id = ZN.user",
    params![id],
    |row| {
      Ok(ZkNote {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        user: row.get(2)?,
        username: row.get(3)?,
        pubid: row.get(4)?,
        createdate: row.get(5)?,
        changeddate: row.get(6)?,
      })
    },
  )?;

  if uid == Some(note.user) {
    Ok(note)
  } else if is_zknote_public(conn, id)? {
    Ok(note)
  } else {
    match uid {
      Some(uid) => {
        if is_zknote_shared(conn, id, uid)? {
          Ok(note)
        } else {
          bail!("can't read zknote; note is private")
        }
      }
      None => bail!("can't read zknote; note is private"),
    }
  }
}

pub fn read_zknotepubid(
  conn: &Connection,
  uid: Option<i64>,
  pubid: &str,
) -> Result<ZkNote, Box<dyn Error>> {
  let publicid = note_id(&conn, "system", "public")?;
  let note = conn.query_row(
    "select A.id, A.title, A.content, A.user, U.name, A.pubid, A.createdate, A.changeddate
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
        pubid: row.get(5)?,
        createdate: row.get(6)?,
        changeddate: row.get(7)?,
      })
    },
  )?;

  match uid {
    Some(uid) => {
      // if !is_zknote_member(&conn, uid, note.id)? {
      if note.user != uid {
        bail!("can't read zknote; you are not a member of this zk");
      }
    }
    None => {
      if !is_zknote_public(&conn, note.id)? {
        bail!("can't read zknote; note is private");
      }
    }
  }

  Ok(note)
}

// delete the note; fails if there are links to it.
pub fn delete_zknote(dbfile: &Path, uid: i64, noteid: i64) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  // only delete when user is in the zk
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
        conn.execute(
        "insert into zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
          on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
        params![zklink.from, zklink.to, uid, zklink.linkzknote],
      )?;
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
      Direction::From => (zknid, link.otherid),
      Direction::To => (link.otherid, zknid),
    };
    if link.user == uid {
      if link.delete == Some(true) {
        conn.execute(
          "delete from zklink where fromid = ?1 and toid = ?2 and user = ?3",
          params![from, to, uid],
        )?;
      } else {
        conn.execute(
        "insert into zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
          on conflict (fromid, toid, user) do update set linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
        params![from, to, uid, link.zknote],
      )?;
      }
    }
  }

  Ok(())
}

pub fn read_zklinks(
  conn: &Connection,
  uid: i64,
  gzl: &GetZkLinks,
) -> Result<Vec<ZkLink>, Box<dyn Error>> {
  let pubid = note_id(&conn, "system", "public")?;

  let usershares = user_shares(&conn, uid)?;

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
  // not-mine zklinks with from = this, and 'to' in usershares.

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
           (A.fromid = ?2 and A.toid = B.toid and B.fromid in ({})))",
    s, s, s, s
  );

  let mut pstmt = conn.prepare(sqlstr.as_str())?;

  let rec_iter = pstmt.query_map(params![uid, gzl.zknote, pubid], |row| {
    Ok(ZkLink {
      from: row.get(0)?,
      to: row.get(1)?,
      user: row.get(2)?,
      delete: None,
      linkzknote: row.get(3)?,
      fromname: row.get(4)?,
      toname: row.get(5)?,
    })
  })?;

  let mut pv = Vec::new();

  for rsrec in rec_iter {
    match rsrec {
      Ok(rec) => {
        pv.push(rec);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
}

pub fn read_public_zklinks(conn: &Connection, noteid: i64) -> Result<Vec<ZkLink>, Box<dyn Error>> {
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

  let rec_iter = pstmt.query_map(params![noteid, pubid, sysid], |row| {
    Ok(ZkLink {
      from: row.get(0)?,
      to: row.get(1)?,
      user: row.get(2)?,
      delete: None,
      linkzknote: row.get(3)?,
      fromname: row.get(4)?,
      toname: row.get(5)?,
    })
  })?;

  let mut pv = Vec::new();

  for rsrec in rec_iter {
    match rsrec {
      Ok(rec) => {
        pv.push(rec);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
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

  // Users
  let mut ustmt = conn.prepare(
    "select id, name, hashwd, salt, email, registration_key
      from user",
  )?;

  let u_iter = ustmt.query_map(params![], |row| {
    Ok(User {
      id: row.get(0)?,
      name: row.get(1)?,
      hashwd: row.get(2)?,
      salt: row.get(3)?,
      email: row.get(4)?,
      registration_key: row.get(5)?,
      token: None::<Uuid>,
      tokendate: None,
    })
  })?;

  let mut uv = Vec::new();

  for rsrec in u_iter {
    match rsrec {
      Ok(rec) => {
        uv.push(rec);
      }
      Err(_) => (),
    }
  }

  // Notes
  let mut nstmt = conn.prepare(
    "select ZN.id, ZN.title, ZN.content, ZN.user, ZN.pubid, ZN.createdate, ZN.changeddate
      from zknote ZN",
  )?;

  let n_iter = nstmt.query_map(params![], |row| {
    Ok(ZkNote {
      id: row.get(0)?,
      title: row.get(1)?,
      content: row.get(2)?,
      user: row.get(3)?,
      username: "".to_string(),
      pubid: row.get(4)?,
      createdate: row.get(5)?,
      changeddate: row.get(6)?,
    })
  })?;

  let mut nv = Vec::new();

  for rsrec in n_iter {
    match rsrec {
      Ok(rec) => {
        nv.push(rec);
      }
      Err(_) => (),
    }
  }

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

  let mut lv: Vec<ZkLink> = Vec::new();

  for rsrec in l_iter {
    match rsrec {
      Ok(rec) => {
        lv.push(rec);
      }
      Err(_) => (),
    }
  }

  Ok(ZkDatabase {
    notes: nv,
    links: lv,
    users: uv,
  })
}
