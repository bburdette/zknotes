use rusqlite::{params, Connection};
use serde_json;
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize, Debug, Clone)]
pub struct PdfInfo {
  pub last_read: Option<i64>,
  pub filename: String,
  pub state: Option<serde_json::Value>,
}

#[derive(Serialize, Debug, Clone)]
pub struct FullBlogEntry {
  id: i64,
  title: String,
  content: String,
  blog: i64,
  createdate: i64,
  changeddate: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct BlogListEntry {
  id: i64,
  title: String,
  user: i64,
  createdate: i64,
  changeddate: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SaveBlogEntry {
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

  println!("pre blog");
  conn.execute(
    "CREATE TABLE blog (
                id          	INTEGER NOT NULL PRIMARY KEY,
                name 					TEXT NOT NULL,
                createdate 		INTEGER NOT NULL,
                changeddate 	INTEGER NOT NULL
                )",
    params![],
  )?;

  println!("pre bm");
  conn.execute(
    "CREATE TABLE blogmember (
                user 					INTEGER NOT NULL,
                blog          INTEGER NOT NULL,
                createdate 		INTEGER NOT NULL,
                changeddate 	INTEGER NOT NULL,
                FOREIGN KEY(user) REFERENCES user(id),
                FOREIGN KEY(blog) REFERENCES blog(id),
                CONSTRAINT unq UNIQUE (user, blog)
                )",
    params![],
  )?;

  println!("pre be");
  conn.execute(
    "CREATE TABLE blogentry (
                id          	INTEGER NOT NULL PRIMARY KEY,
                title					TEXT NOT NULL,
                content 			TEXT NOT NULL,
                blog 					INTEGER NOT NULL,
                createdate 		INTEGER NOT NULL,
                changeddate 	INTEGER NOT NULL,
                FOREIGN KEY(blog) REFERENCES blog(id)
                )",
    params![],
  )?;

  println!("pre bl");
  conn.execute(
    "CREATE TABLE bloglink (
                blogleft   		 INTEGER NOT NULL,
                blogright      INTEGER NOT NULL,
                linkblog 			 INTEGER,
                FOREIGN KEY(linkblog) REFERENCES blogentry(id),
                FOREIGN KEY(blogleft) REFERENCES blogentry(id),
                FOREIGN KEY(blogright) REFERENCES blogentry(id),
                CONSTRAINT unq UNIQUE (blogleft, blogright)
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

pub fn add_tag(dbfile: &Path, name: &str, user: i64) -> Result<i64, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  println!("adding tag: {}", name);
  conn.execute(
    "INSERT INTO tag (name, user)
                VALUES (?1, ?2)",
    params![name, user],
  )?;

  Ok(conn.last_insert_rowid())
}

pub fn save_blogentry(
  dbfile: &Path,
  uid: i64,
  entry: &SaveBlogEntry,
) -> Result<i64, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let now = naiow()?;

  match entry.id {
    Some(id) => {
      println!("updating blogentry: {}", entry.title);
      conn.execute(
        "UPDATE blogentry SET title = ?1, content = ?2, changeddate = ?3
         WHERE id = ?4",
        params![entry.title, entry.content, now, entry.id],
      )?;
      Ok(id)
    }
    None => {
      println!("adding blogentry: {}", entry.title);
      conn.execute(
        "INSERT INTO blogentry (title, content, user, createdate, changeddate)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![entry.title, entry.content, uid, now, now],
      )?;

      Ok(conn.last_insert_rowid())
    }
  }
}

pub fn read_blogentry(dbfile: &Path, id: i64) -> Result<FullBlogEntry, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let rbe = conn.query_row(
    "SELECT title, content, blog, createdate, changeddate
      FROM blogentry WHERE id = ?1",
    params![id],
    |row| {
      Ok(FullBlogEntry {
        id: id,
        title: row.get(0)?,
        content: row.get(1)?,
        blog: row.get(2)?,
        createdate: row.get(3)?,
        changeddate: row.get(4)?,
      })
    },
  )?;

  Ok(rbe)
}
pub fn delete_blogentry(dbfile: &Path, uid: i64, beid: i64) -> Result<(), Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  conn.execute(
    "DELETE FROM blogentry WHERE id = ?1 and user = ?2",
    params![beid, uid],
  )?;

  Ok(())
}

pub fn bloglisting(dbfile: &Path, user: i64) -> rusqlite::Result<Vec<BlogListEntry>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare(
    "SELECT id, title, createdate, changeddate
      FROM blogentry where user = ?1",
  )?;

  let pdfinfo_iter = pstmt.query_map(params![user], |row| {
    Ok(BlogListEntry {
      id: row.get(0)?,
      title: row.get(1)?,
      user: user,
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
