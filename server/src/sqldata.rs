use rusqlite::{params, Connection};
use serde_json;
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

#[derive(Serialize, Debug, Clone)]
pub struct FullZkNote {
  id: i64,
  title: String,
  content: String,
  zk: i64,
  public: bool,
  createdate: i64,
  changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
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

#[derive(Serialize, Debug, Clone)]
pub struct ZkListNote {
  id: i64,
  title: String,
  zk: i64,
  createdate: i64,
  changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct SavedZkNote {
  id: i64,
  changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  id: Option<i64>,
  zk: i64,
  title: String,
  public: bool,
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
  pub zk: i64,
  pub links: Vec<ZkLink>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetZkLinks {
  pub zknote: i64,
  pub zk: i64,
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
  }));

  conn.execute("PRAGMA foreign_keys = true;", params![])?;

  Ok(conn)
}

pub fn dbinit(dbfile: &Path) -> rusqlite::Result<()> {
  let conn = connection_open(dbfile)?;

  // println!("pre user");
  conn.execute(
    "CREATE TABLE user (
                id          INTEGER NOT NULL PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                hashwd      TEXT NOT NULL,
                salt        TEXT NOT NULL,
                email       TEXT NOT NULL,
                registration_key  TEXT,
                createdate  INTEGER NOT NULL
                )",
    params![],
  )?;

  // println!("pre zk");
  conn.execute(
    "CREATE TABLE zk (
                id            INTEGER NOT NULL PRIMARY KEY,
                name          TEXT NOT NULL,
                description   TEXT NOT NULL,
                createdate    INTEGER NOT NULL,
                changeddate   INTEGER NOT NULL
                )",
    params![],
  )?;

  // println!("pre bm");
  conn.execute(
    "CREATE TABLE zkmember (
                user          INTEGER NOT NULL,
                zk            INTEGER NOT NULL,
                FOREIGN KEY(user) REFERENCES user(id),
                FOREIGN KEY(zk) REFERENCES zk(id),
                CONSTRAINT unq UNIQUE (user, zk)
                )",
    params![],
  )?;

  // println!("pre be");
  conn.execute(
    "CREATE TABLE zknote (
                id            INTEGER NOT NULL PRIMARY KEY,
                title         TEXT NOT NULL,
                content       TEXT NOT NULL,
                public        BOOL NOT NULL,
                zk            INTEGER NOT NULL,
                createdate    INTEGER NOT NULL,
                changeddate   INTEGER NOT NULL,
                FOREIGN KEY(zk) REFERENCES zk(id)
                )",
    params![],
  )?;

  // println!("pre bl");
  conn.execute(
    "CREATE TABLE zklink (
                fromid        INTEGER NOT NULL,
                toid          INTEGER NOT NULL,
                zk            INTEGER NOT NULL,
                linkzknote    INTEGER,
                FOREIGN KEY(linkzknote) REFERENCES zknote(id),
                FOREIGN KEY(fromid) REFERENCES zknote(id),
                FOREIGN KEY(toid) REFERENCES zknote(id),
                FOREIGN KEY(zk) REFERENCES zk(id),
                CONSTRAINT unq UNIQUE (fromid, toid, zk)
                )",
    params![],
  )?;

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

pub fn add_user(dbfile: &Path, name: &str, hashwd: &str) -> Result<i64, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let nowi64secs = now()?;

  println!("adding user: {}", name);
  let wat = conn.execute(
    "INSERT INTO user (name, hashwd, createdate)
                VALUES (?1, ?2, ?3)",
    params![name, hashwd, nowi64secs],
  )?;

  println!("wat: {}", wat);

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

pub fn new_user(
  dbfile: &Path,
  name: String,
  hashwd: String,
  salt: String,
  email: String,
  registration_key: String,
) -> Result<i64, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let now = now()?;

  let user = conn.execute(
    "INSERT INTO user (name, hashwd, salt, email, registration_key, createdate)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![name, hashwd, salt, email, registration_key, now],
  )?;

  Ok(conn.last_insert_rowid())
}

// zk CRUD

pub fn save_zk(dbfile: &Path, uid: i64, savezk: &SaveZk) -> Result<i64, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let now = now()?;

  match savezk.id {
    Some(id) => {
      println!("updating zk: {}", savezk.name);

      // TODO ensure user auth here.

      conn.execute(
        "UPDATE zk SET name = ?1, description = ?2, changeddate = ?3
         WHERE id = ?4",
        params![savezk.name, savezk.description, now, savezk.id],
      )?;
      Ok(id)
    }
    None => {
      println!("adding zk: {}", savezk.name);
      conn.execute(
        "INSERT INTO zk (name, description, createdate, changeddate)
         VALUES (?1, ?2, ?3, ?4)",
        params![savezk.name, savezk.description, now, now],
      )?;

      let zkid = conn.last_insert_rowid();

      conn.execute(
        "INSERT INTO zkmember (zk, user)
         VALUES (?1, ?2)",
        params![zkid, uid],
      )?;

      Ok(zkid)
    }
  }
}

pub fn is_zk_member(conn: &Connection, uid: i64, zkid: i64) -> Result<bool, Box<dyn Error>> {
  match conn.query_row(
    "select user, zk from zkmember where user = ?1 and zk = ?2",
    params![uid, zkid],
    |row| Ok(true),
  ) {
    Ok(b) => Ok(b),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
    Err(x) => Err(Box::new(x)),
  }
}

pub fn delete_zk(dbfile: &Path, uid: i64, zkid: i64) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  // start a transaction
  conn.execute("BEGIN TRANSACTION", params![])?;

  if !is_zk_member(&conn, uid, zkid)? {
    bail!("can't delete; user is not a member of this zk");
  }

  // delete all member entries for the zk.
  conn.execute("DELETE FROM zkmember WHERE zk = ?1", params![zkid])?;

  // if delete successful, also remove the zmember entries.
  conn.execute("DELETE FROM zk WHERE id = ?1", params![zkid])?;

  conn.execute("END TRANSACTION", params![])?;

  Ok(())
}

pub fn zklisting(dbfile: &Path, user: i64) -> rusqlite::Result<Vec<Zk>> {
  let conn = connection_open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, name, description, createdate, changeddate
      FROM zk, zkmember
      where zkmember.user = ?1
      and zkmember.zk = zk.id",
  )?;

  let rec_iter = pstmt.query_map(params![user], |row| {
    Ok(Zk {
      id: row.get(0)?,
      name: row.get(1)?,
      description: row.get(2)?,
      createdate: row.get(3)?,
      changeddate: row.get(4)?,
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

pub fn read_zk_members(dbfile: &Path, uid: i64, zkid: i64) -> Result<Vec<String>, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  if !is_zk_member(&conn, uid, zkid)? {
    bail!("can't delete; user is not a member of this zk");
  }

  let mut pstmt = conn.prepare(
    "SELECT user.name from zkmember, user where zkmember.zk = ?1 and user.id = zkmember.user",
  )?;

  let rec_iter = pstmt.query_map(params![zkid], |row| Ok(row.get(0)?))?;

  let mut pv = Vec::new();

  for rsrec in rec_iter {
    match rsrec {
      Ok(uid) => {
        pv.push(uid);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
}

pub fn add_zk_member(dbfile: &Path, uid: i64, zkm: ZkMember) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  conn.execute("BEGIN TRANSACTION", params![])?;

  if !is_zk_member(&conn, uid, zkm.zkid)? {
    bail!("can't add; you are not a member of this zk");
  }

  let r: i64 = conn.query_row(
    "SELECT id from user where name = ?1",
    params![zkm.name],
    |row| Ok(row.get(0)?),
  )?;

  conn.execute(
    "insert into zkmember (user, zk) values (?1, ?2)",
    params![r, zkm.zkid],
  )?;

  conn.execute("END TRANSACTION", params![])?;

  Ok(())
}
pub fn delete_zk_member(dbfile: &Path, uid: i64, zkm: ZkMember) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  conn.execute("BEGIN TRANSACTION", params![])?;

  if !is_zk_member(&conn, uid, zkm.zkid)? {
    bail!("can't delete; you are not a member of this zk");
  }

  let r: i64 = conn.query_row(
    "select id from user where name = ?1",
    params![zkm.name],
    |row| Ok(row.get(0)?),
  )?;

  conn.execute(
    "delete from zkmember where user = ?1 and zk = ?2",
    params![r, zkm.zkid],
  )?;

  conn.execute("END TRANSACTION", params![])?;

  Ok(())
}

pub fn read_zk(dbfile: &Path, id: i64) -> Result<Zk, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let rbe = conn.query_row(
    "SELECT name, description, createdate, changeddate
      FROM zk WHERE id = ?1",
    params![id],
    |row| {
      Ok(Zk {
        id: id,
        name: row.get(0)?,
        description: row.get(1)?,
        createdate: row.get(2)?,
        changeddate: row.get(3)?,
      })
    },
  )?;

  Ok(rbe)
}

// zknote CRUD

pub fn save_zknote(
  dbfile: &Path,
  uid: i64,
  note: &SaveZkNote,
) -> Result<SavedZkNote, Box<dyn Error>> {
  println!("prezkn conn");
  let conn = connection_open(dbfile)?;
  println!("postzkn conn");

  let now = now()?;

  // TODO ensure user auth here.

  match note.id {
    Some(id) => {
      println!("updating zknote: {}", note.title);
      conn.execute(
        "UPDATE zknote SET title = ?1, content = ?2, changeddate = ?3, public = ?4
         WHERE id = ?5",
        params![note.title, note.content, now, note.public, note.id],
      )?;
      println!("updated zknote: {}", note.title);
      Ok(SavedZkNote {
        id: id,
        changeddate: now,
      })
    }
    None => {
      println!("adding zknote: {}", note.title);
      conn.execute(
        "INSERT INTO zknote (title, content, zk, public, createdate, changeddate)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![note.title, note.content, note.zk, note.public, now, now],
      )?;
      Ok(SavedZkNote {
        id: conn.last_insert_rowid(),
        changeddate: now,
      })
    }
  }
}

pub fn read_zknote(dbfile: &Path, id: i64) -> Result<FullZkNote, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  let rbe = conn.query_row(
    "SELECT title, content, zk, public, createdate, changeddate
      FROM zknote WHERE id = ?1",
    params![id],
    |row| {
      Ok(FullZkNote {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        zk: row.get(2)?,
        public: row.get(3)?,
        createdate: row.get(4)?,
        changeddate: row.get(5)?,
      })
    },
  )?;

  Ok(rbe)
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

pub fn zknotelisting(dbfile: &Path, user: i64, zk: i64) -> rusqlite::Result<Vec<ZkListNote>> {
  let conn = connection_open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, title, createdate, changeddate
      FROM zknote where zk = ?1 and
        zk IN (select zk from zkmember where user = ?2)",
  )?;

  let rec_iter = pstmt.query_map(params![zk, user], |row| {
    Ok(ZkListNote {
      id: row.get(0)?,
      title: row.get(1)?,
      zk: zk,
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

pub fn save_zklink(
  conn: &Connection,
  uid: i64,
  zk: i64,
  zklink: &ZkLink,
) -> Result<(), Box<dyn Error>> {
  if zklink.delete == Some(true) {
    println!("zklink delete: {:?}", zklink);
    conn.execute(
      "DELETE FROM zklink WHERE fromid = ?1 and toid = ?2 and zk = ?3",
      params![zklink.from, zklink.to, zk],
    )?;
  } else {
    println!("zklink insert: {:?}", zklink);
    conn.execute(
      "INSERT INTO zklink (fromid, toid, zk, linkzknote) values (?1, ?2, ?3, ?4)
        ON CONFLICT (fromid, toid, zk) DO UPDATE SET linkzknote = ?4 where fromid = ?1 and toid = ?2 and zk = ?3",
      params![zklink.from, zklink.to, zk, zklink.linkzknote],
    )?;
  }
  Ok(())
}

pub fn save_zklinks(
  dbfile: &Path,
  uid: i64,
  zk: i64,
  zklinks: Vec<ZkLink>,
) -> Result<(), Box<dyn Error>> {
  println!("prezklink conn");
  let conn = connection_open(dbfile)?;
  println!("postzklink conn");

  // conn.execute("BEGIN TRANSACTION", params![])?;
  println!("postzklink begin trans");

  if !is_zk_member(&conn, uid, zk)? {
    bail!("can't save zklink; user is not a member of this zk");
  }

  println!("pre saves");

  for zklink in zklinks.iter() {
    save_zklink(&conn, uid, zk, &zklink)?;
  }

  // conn.execute("END TRANSACTION", params![])?;
  println!("postzklink trans");

  Ok(())
}

pub fn read_zklinks(
  dbfile: &Path,
  uid: i64,
  gzl: &GetZkLinks,
) -> Result<Vec<ZkLink>, Box<dyn Error>> {
  let conn = connection_open(dbfile)?;

  if !is_zk_member(&conn, uid, gzl.zk)? {
    bail!("can't read_zklinks; user is not a member of this zk");
  }

  let mut pstmt = conn.prepare(
    "SELECT fromid, toid, linkzknote, L.title, R.title
      FROM zklink 
      INNER JOIN zknote as L ON zklink.fromid = L.id
      INNER JOIN zknote as R ON zklink.toid = R.id
      where zklink.zk = ?1 and (zklink.fromid = ?2 or zklink.toid = ?2)
      ",
  )?;

  let rec_iter = pstmt.query_map(params![gzl.zk, gzl.zknote], |row| {
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
