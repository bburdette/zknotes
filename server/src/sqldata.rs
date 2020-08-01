use rusqlite::{params, Connection};
use serde_json;
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize, Debug, Clone)]
pub struct FullZkNote {
  id: i64,
  title: String,
  content: String,
  zk: i64,
  createdate: i64,
  changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkList {
  id: i64,
  name: String,
  description: String,
  createdate: i64,
  changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZk {
  id: Option<i64>,
  name: String,
  description: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ZkNoteList {
  id: i64,
  title: String,
  zk: i64,
  createdate: i64,
  changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveZkNote {
  id: Option<i64>,
  title: String,
  content: String,
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

pub fn dbinit(dbfile: &Path) -> rusqlite::Result<()> {
  let conn = Connection::open(dbfile)?;

  println!("pre user");
  // create the pdfinfo table.
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

  println!("pre zk");
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

  println!("pre bm");
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

  println!("pre be");
  conn.execute(
    "CREATE TABLE zknote (
                id            INTEGER NOT NULL PRIMARY KEY,
                title         TEXT NOT NULL,
                content       TEXT NOT NULL,
                zk            INTEGER NOT NULL,
                createdate    INTEGER NOT NULL,
                changeddate   INTEGER NOT NULL,
                FOREIGN KEY(zk) REFERENCES zk(id)
                )",
    params![],
  )?;

  println!("pre bl");
  conn.execute(
    "CREATE TABLE zklink (
                zkleft        INTEGER NOT NULL,
                zkright       INTEGER NOT NULL,
                linkzk        INTEGER,
                FOREIGN KEY(linkzk) REFERENCES zknote(id),
                FOREIGN KEY(zkleft) REFERENCES zknote(id),
                FOREIGN KEY(zkright) REFERENCES zknote(id),
                CONSTRAINT unq UNIQUE (zkleft, zkright)
                )",
    params![],
  )?;

  Ok(())
}

pub fn naiow() -> Result<i64, Box<dyn Error>> {
  let nowsecs = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map(|n| n.as_secs())?;
  let s: i64 = nowsecs.try_into()?;
  Ok(s * 1000)
}

// user CRUD

pub fn add_user(dbfile: &Path, name: &str, hashwd: &str) -> Result<i64, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let nowi64secs = naiow()?;

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
  let conn = Connection::open(dbfile)?;

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
  let conn = Connection::open(dbfile)?;

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
  let conn = Connection::open(dbfile)?;

  let now = naiow()?;

  let user = conn.execute(
    "INSERT INTO user (name, hashwd, salt, email, registration_key, createdate)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![name, hashwd, salt, email, registration_key, now],
  )?;

  Ok(conn.last_insert_rowid())
}

// zk CRUD

pub fn save_zk(dbfile: &Path, uid: i64, savezk: &SaveZk) -> Result<i64, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let now = naiow()?;

  match savezk.id {
    Some(id) => {
      println!("updating zk: {}", savezk.name);
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

// pub fn read_zk(dbfile: &Path, id: i64) -> Result<FullZkNote, Box<dyn Error>> {
//   let conn = Connection::open(dbfile)?;

//   let rbe = conn.query_row(
//     "SELECT title, content, zk, createdate, changeddate
//       FROM zk WHERE id = ?1",
//     params![id],
//     |row| {
//       Ok(FullZkNote {
//         id: id,
//         title: row.get(0)?,
//         content: row.get(1)?,
//         zk: row.get(2)?,
//         createdate: row.get(3)?,
//         changeddate: row.get(4)?,
//       })
//     },
//   )?;

//   Ok(rbe)
// }

pub fn delete_zk(dbfile: &Path, uid: i64, id: i64) -> Result<(), Box<dyn Error>> {
  // let conn = Connection::open(dbfile)?;

  // // is this user in the zkmember table for this zk?
  // conn.execute(

  // // if delete successful, also remove the zmember entries.

  // // only delete when user is in the zk
  // conn.execute(
  //   "DELETE FROM zk WHERE id = ?1
  //     AND zk IN (SELECT zk FROM zkmember WHERE user = ?2)",
  //   params![id, uid],
  // )?;
  // Ok(())

  bail!("unimplemented");

  // Err(Box::new(simple_error::SimpleError {
  //   err: "unimplemented".to_string(),
  // }))
}

pub fn zklisting(dbfile: &Path, user: i64) -> rusqlite::Result<Vec<ZkList>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, title, createdate, changeddate
      FROM zk, zkmember
      where zkmember.user = ?1
      and zknote.zk = zknote.id",
  )?;

  let rec_iter = pstmt.query_map(params![user], |row| {
    Ok(ZkList {
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

// zknote CRUD

pub fn save_zknote(dbfile: &Path, uid: i64, note: &SaveZkNote) -> Result<i64, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let now = naiow()?;

  match note.id {
    Some(id) => {
      println!("updating zknote: {}", note.title);
      conn.execute(
        "UPDATE zknote SET title = ?1, content = ?2, changeddate = ?3
         WHERE id = ?4",
        params![note.title, note.content, now, note.id],
      )?;
      Ok(id)
    }
    None => {
      println!("adding zknote: {}", note.title);
      conn.execute(
        "INSERT INTO zknote (title, content, user, createdate, changeddate)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![note.title, note.content, uid, now, now],
      )?;

      Ok(conn.last_insert_rowid())
    }
  }
}

pub fn read_zknote(dbfile: &Path, id: i64) -> Result<FullZkNote, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let rbe = conn.query_row(
    "SELECT title, content, zk, createdate, changeddate
      FROM zknote WHERE id = ?1",
    params![id],
    |row| {
      Ok(FullZkNote {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        zk: row.get(2)?,
        createdate: row.get(3)?,
        changeddate: row.get(4)?,
      })
    },
  )?;

  Ok(rbe)
}
pub fn delete_zknote(dbfile: &Path, uid: i64, noteid: i64) -> Result<(), Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  // only delete when user is in the zk
  conn.execute(
    "DELETE FROM zknote WHERE id = ?1 
      AND zk IN (SELECT zk FROM zkmember WHERE user = ?2)",
    params![noteid, uid],
  )?;

  Ok(())
}

pub fn zknotelisting(dbfile: &Path, user: i64, zk: i64) -> rusqlite::Result<Vec<ZkNoteList>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, title, createdate, changeddate
      FROM zknote where zk = ?1 and
        zk IN (select id from zk where user = ?2",
  )?;

  let pdfinfo_iter = pstmt.query_map(params![zk, user], |row| {
    Ok(ZkNoteList {
      id: row.get(0)?,
      title: row.get(1)?,
      zk: zk,
      createdate: row.get(2)?,
      changeddate: row.get(3)?,
    })
  })?;

  let mut pv = Vec::new();

  for rspdfinfo in pdfinfo_iter {
    match rspdfinfo {
      Ok(pdfinfo) => {
        pv.push(pdfinfo);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
}
