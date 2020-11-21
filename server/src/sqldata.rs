use barrel::backend::Sqlite;
use barrel::{types, Migration};
use rusqlite::{params, Connection};
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

#[derive(Serialize, Debug, Clone)]
pub struct ZkNote {
  pub id: i64,
  title: String,
  content: String,
  user: i64,
  // pub public: bool,
  pubid: Option<String>,
  createdate: i64,
  changeddate: i64,
}

/*#[derive(Serialize, Debug, Clone)]
pub struct Zk {
  id: i64,
  name: String,
  description: String,
  createdate: i64,
  changeddate: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkMember {
  zkid: i64,
  name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZk {
  id: Option<i64>,
  name: String,
  description: String,
}
*/
#[derive(Serialize, Debug, Clone)]
pub struct ZkListNote {
  pub id: i64,
  pub title: String,
  pub user: i64,
  pub createdate: i64,
  pub changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct SavedZkNote {
  id: i64,
  changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  id: Option<i64>,
  title: String,
  public: bool,
  pubid: Option<String>,
  content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLink {
  from: i64,
  to: i64,
  delete: Option<bool>,
  linkzknote: Option<i64>,
  fromname: Option<String>,
  toname: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkLinks {
  pub links: Vec<ZkLink>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkLinks {
  pub zknote: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkNoteEdit {
  pub zknote: i64,
}

#[derive(Serialize, Debug)]
pub struct ZkNoteEdit {
  pub zknote: ZkNote,
  pub links: Vec<ZkLink>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
  pub id: i64,
  pub name: String,
  pub hashwd: String,
  pub salt: String,
  pub email: String,
  pub registration_key: Option<String>,
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
    "INSERT INTO zknotetemp (id, title, content, public, pubid, zk, createdate, changeddate)
        select id, title, content, public, NULL, zk, createdate, changeddate from zknote",
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
    "INSERT INTO zknote (id, title, content, public, pubid, zk, createdate, changeddate)
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
    "INSERT INTO zknotetemp (id, title, content, pubid, user, createdate, changeddate)
        select zknote.id, title, content, pubid, user.id, zknote.createdate, zknote.changeddate from zknote, user
        where user.id in (select user from zkmember where zkmember.zk = zknote.zk)",
    params![],
  )?;

  // copy everything from zklink.
  conn.execute(
    "INSERT INTO zklinktemp (fromid, toid, user)
        select fromid, toid, user.id from zklink, user
        where user.id in (select user from zkmember where zkmember.zk = zklink.zk)",
    params![],
  )?;

  // copy everything from user.
  conn.execute(
    "INSERT INTO usertemp (id, name, hashwd, salt, email, registration_key, createdate)
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
    "INSERT INTO zknote (id, title, content, pubid, user, createdate, changeddate)
        select id, title, content, pubid, user, createdate, changeddate from zknotetemp",
    params![],
  )?;

  // copy everything from zklinktemp.
  conn.execute(
    "INSERT INTO zklink (fromid, toid, user, linkzknote)
        select fromid, toid, user, linkzknote from zklinktemp",
    params![],
  )?;

  // copy everything from usertemp.
  conn.execute(
    "INSERT INTO user (id, name, hashwd, salt, email, registration_key, createdate)
        select id, name, hashwd, salt, email, registration_key, createdate from usertemp",
    params![],
  )?;

  let now = now()?;

  // create system user.
  conn.execute(
    "INSERT INTO user (name, hashwd, salt, email, registration_key, createdate)
      VALUES ('system', '', '', '', null, ?1)",
    params![now],
  )?;
  let sysid = conn.last_insert_rowid();

  // system tags.
  conn.execute(
    "INSERT INTO zknote (title, content, pubid, user, createdate, changeddate)
      VALUES ('public', '', null, ?1, ?2, ?3)",
    params![sysid, now, now],
  )?;

  let publicid = conn.last_insert_rowid();

  conn.execute(
    "INSERT INTO zknote (title, content, pubid, user, createdate, changeddate)
      VALUES ('share', '', null, ?1, ?2, ?3)",
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
    "INSERT INTO singlevalue (name, value) values (?1, ?2)
        ON CONFLICT (name) DO UPDATE SET value = ?2 where name = ?1",
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

  println!("db up to date.");

  Ok(())
}

pub fn now() -> Result<i64, Box<dyn Error>> {
  let nowsecs = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map(|n| n.as_secs())?;
  let s: i64 = nowsecs.try_into()?;
  Ok(s * 1000)
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
    "INSERT INTO zknote (title, content, user, createdate, changeddate)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![name, "", systemid, now, now],
  )?;

  let zknid = conn.last_insert_rowid();

  // make a user record.
  conn.execute(
    "INSERT INTO user (name, zknote, hashwd, salt, email, registration_key, createdate)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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
    "INSERT INTO zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
      ON CONFLICT (fromid, toid, user) DO UPDATE SET linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
    params![fromid, toid, user, linkzknote],
  )?;

  Ok(conn.last_insert_rowid())
}

pub fn read_user(dbfile: &Path, name: &str) -> Result<User, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let user = conn.query_row(
    "SELECT id, hashwd, salt, email, registration_key
      FROM user WHERE name = ?1",
    params![name],
    |row| {
      Ok(User {
        id: row.get(0)?,
        name: name.to_string(),
        hashwd: row.get(1)?,
        salt: row.get(2)?,
        email: row.get(3)?,
        registration_key: row.get(4)?,
      })
    },
  )?;

  Ok(user)
}

pub fn update_user(dbfile: &Path, user: &User) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  conn.execute(
    "UPDATE user SET name = ?1, hashwd = ?2, salt = ?3, email = ?4, registration_key = ?5
     WHERE id = ?6",
    params![
      user.name,
      user.hashwd,
      user.salt,
      user.email,
      user.registration_key,
      user.id
    ],
  )?;

  Ok(())
}

pub fn note_id(conn: &Connection, name: &str, title: &str) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select id from
      zknote, user
      where zknote.title = ?2
      and user.name = ?1
      and zknote.user = user.id",
    params![name, title],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
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

pub fn is_zknote_shared(
  conn: &Connection,
  uid: i64,
  zknoteid: i64,
) -> Result<bool, Box<dyn Error>> {
  Ok(false)

  /* shared means there is:
   * a note linked to this note.
   * that note is linked to system:share
   * that note is also linked to system:user:this-user
   *


  */

  // let shareid = match conn.query_row(
  //   "select id from
  //     zknote, user
  //     where zknote.title = \"share\"
  //     and user.name = \"system\"
  //     and zknote.user = user.id",
  //   params![],
  //   |row| row.get(0)?);

  // //

  // match conn.query_row(
  //   "select count(*) from
  //     zklink A, zklink B, zknote
  //     where A.leftid = ?1 and A.rightid = B.leftid and B.rightid = ?2
  //     and zknote.id = ?2 and zknote.user = ?3",
  //   params![zknoteid, shareid, uid],
  //   |_row| Ok(true),
  // ) {
  //   Ok(b) => Ok(b),
  //   Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
  //   Err(x) => Err(Box::new(x)),
  // }
}

pub fn is_zknote_public(conn: &Connection, zknoteid: i64) -> Result<bool, Box<dyn Error>> {
  let pubid: i64 = conn.query_row(
    "select zknote.id from
      zknote, user
      where zknote.title = \"public\"
      and user.name = \"system\"
      and zknote.user = user.id",
    params![],
    |row| Ok(row.get(0)?),
  )?;

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
  dbfile: &Path,
  uid: i64,
  note: &SaveZkNote,
) -> Result<SavedZkNote, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let now = now()?;

  // if !is_zk_member(&conn, uid, note.zk)? {
  //   bail!("can't save note; you are not a member of this zk");
  // }

  match note.id {
    Some(id) => {
      conn.execute(
        "UPDATE zknote SET title = ?1, content = ?2, changeddate = ?3, pubid = ?4
         WHERE id = ?5 and user = ?6",
        params![note.title, note.content, now, note.pubid, note.id, uid],
      )?;
      Ok(SavedZkNote {
        id: id,
        changeddate: now,
      })
    }
    None => {
      conn.execute(
        "INSERT INTO zknote (title, content, user, pubid, createdate, changeddate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![note.title, note.content, uid, note.public, note.pubid, now],
      )?;
      Ok(SavedZkNote {
        id: conn.last_insert_rowid(),
        changeddate: now,
      })
    }
  }
}

pub fn read_zknote(conn: &Connection, uid: Option<i64>, id: i64) -> Result<ZkNote, Box<dyn Error>> {
  // match uid {
  //   Some(uid) => {
  //     if !is_zknote_member(&conn, uid, id)? {
  //       bail!("can't read zknote; you are not a member of this zk");
  //     }
  //   }
  //   _ => {}
  // }

  println!("read_zknote uid, id {:?}, {}", uid, id);
  let note = conn.query_row(
    "SELECT title, content, user, pubid, createdate, changeddate
      FROM zknote WHERE id = ?1",
    params![id],
    |row| {
      Ok(ZkNote {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        user: row.get(2)?,
        pubid: row.get(3)?,
        createdate: row.get(4)?,
        changeddate: row.get(5)?,
      })
    },
  )?;

  println!("read_zknote {:?}", note);

  if uid == Some(note.user) {
    Ok(note)
  } else if is_zknote_public(conn, id)? {
    Ok(note)
  } else {
    bail!("can't read zknote; note is private")
  }
}

// pub fn is_public(conn: &Connection, zknoteid: i64) -> Result<Bool, Box<dyn Error>> {
//   let rbe = conn.query_row(
//     "SELECT id, title, content, user, pubid, createdate, changeddate
//       FROM zknote WHERE pubid = ?1",
//     params![pubid],
//     |row| {
//       Ok(ZkNote {
//         id: row.get(0)?,
//         title: row.get(1)?,
//         content: row.get(2)?,
//         user: row.get(3)?,
//         pubid: row.get(4)?,
//         createdate: row.get(5)?,
//         changeddate: row.get(7)?,
//       })
//     },
//   )?;
// }

pub fn read_zknotepubid(
  conn: &Connection,
  uid: Option<i64>,
  pubid: &str,
) -> Result<ZkNote, Box<dyn Error>> {
  let note = conn.query_row(
    "SELECT id, title, content, user, pubid, createdate, changeddate
      FROM zknote WHERE pubid = ?1",
    params![pubid],
    |row| {
      Ok(ZkNote {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        user: row.get(3)?,
        pubid: row.get(4)?,
        createdate: row.get(5)?,
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

pub fn delete_zknote(dbfile: &Path, uid: i64, noteid: i64) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  // only delete when user is in the zk
  conn.execute(
    "DELETE FROM zknote WHERE id = ?1 
      AND zk IN (SELECT zk FROM zkmember WHERE user = ?2)",
    params![noteid, uid],
  )?;

  Ok(())
}

/*pub fn zknotelisting(dbfile: &Path, user: i64, zk: i64) -> rusqlite::Result<Vec<ZkListNote>> {
  let conn = connection_open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, title, public, createdate, changeddate
      FROM zknote where zk = ?1 and
        zk IN (select zk from zkmember where user = ?2)",
  )?;

  let rec_iter = pstmt.query_map(params![user], |row| {
    Ok(ZkListNote {
      id: row.get(0)?,
      title: row.get(1)?,
      createdate: row.get(2)?,
      changeddate: row.get(3)?,
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
*/

pub fn save_zklinks(dbfile: &Path, uid: i64, zklinks: Vec<ZkLink>) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  // if !is_zk_member(&conn, uid, zk)? {
  //   bail!("can't save zklink; user is not a member of this zk");
  // }

  for zklink in zklinks.iter() {
    if zklink.delete == Some(true) {
      conn.execute(
        "DELETE FROM zklink WHERE fromid = ?1 and toid = ?2 and user = ?3",
        params![zklink.from, zklink.to, uid],
      )?;
    } else {
      conn.execute(
        "INSERT INTO zklink (fromid, toid, user, linkzknote) values (?1, ?2, ?3, ?4)
          ON CONFLICT (fromid, toid, user) DO UPDATE SET linkzknote = ?4 where fromid = ?1 and toid = ?2 and user = ?3",
        params![zklink.from, zklink.to, uid, zklink.linkzknote],
      )?;
    }
  }

  Ok(())
}

pub fn read_zklinks(
  conn: &Connection,
  uid: i64,
  gzl: &GetZkLinks,
) -> Result<Vec<ZkLink>, Box<dyn Error>> {
  // if !is_zk_member(&conn, uid, gzl.zk)? {
  //   bail!("can't read_zklinks; user is not a member of this zk");
  // }

  let mut pstmt = conn.prepare(
    "SELECT fromid, toid, linkzknote, L.title, R.title
      FROM zklink 
      INNER JOIN zknote as L ON zklink.fromid = L.id
      INNER JOIN zknote as R ON zklink.toid = R.id
      where zklink.user = ?1 and (zklink.fromid = ?2 or zklink.toid = ?2)
      ",
  )?;

  let rec_iter = pstmt.query_map(params![uid, gzl.zknote], |row| {
    Ok(ZkLink {
      from: row.get(0)?,
      to: row.get(1)?,
      delete: None,
      linkzknote: row.get(2)?,
      fromname: row.get(3)?,
      toname: row.get(4)?,
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
