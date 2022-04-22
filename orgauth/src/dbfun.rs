use crate::config::Config;
use crate::data;
use crate::email;
use crate::util::is_token_expired;
use actix_session::Session;
use crypto_hash::{hex_digest, Algorithm};
use either::Either::{Left, Right};
use log::info;
// use simple_error::bail;
use actix_web::HttpRequest;
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

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

pub fn user_id(conn: &Connection, name: &str) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select id from user
      where user.name = ?1",
    params![name],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn read_user_by_name(conn: &Connection, name: &str) -> Result<User, Box<dyn Error>> {
  let user = conn.query_row(
    "select id, zknote, homenote, hashwd, salt, email, registration_key
      from user where name = ?1",
    params![name],
    |row| {
      Ok(User {
        id: row.get(0)?,
        name: name.to_string(),
        noteid: row.get(1)?,
        homenoteid: row.get(2)?,
        hashwd: row.get(3)?,
        salt: row.get(4)?,
        email: row.get(5)?,
        registration_key: row.get(6)?,
      })
    },
  )?;

  Ok(user)
}

pub fn read_user_by_id(conn: &Connection, id: i64) -> Result<User, Box<dyn Error>> {
  let user = conn.query_row(
    "select id, name, zknote, homenote, hashwd, salt, email, registration_key
      from user where id = ?1",
    params![id],
    |row| {
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
    },
  )?;

  Ok(user)
}

pub fn read_user_by_token(
  conn: &Connection,
  token: Uuid,
  token_expiration_ms: Option<i64>,
) -> Result<User, Box<dyn Error>> {
  let (user, tokendate) = conn.query_row(
    "select id, name, zknote, homenote, hashwd, salt, email, registration_key, token.tokendate
      from user, token where user.id = token.user and token = ?1",
    params![token.to_string()],
    |row| {
      Ok((
        User {
          id: row.get(0)?,
          name: row.get(1)?,
          noteid: row.get(2)?,
          homenoteid: row.get(3)?,
          hashwd: row.get(4)?,
          salt: row.get(5)?,
          email: row.get(6)?,
          registration_key: row.get(7)?,
        },
        row.get(8)?,
      ))
    },
  )?;

  match token_expiration_ms {
    Some(texp) => {
      if is_token_expired(texp, tokendate) {
        bail!("login expired")
      } else {
        Ok(user)
      }
    }
    None => Ok(user),
  }
}

pub fn add_token(conn: &Connection, user: i64, token: Uuid) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  conn.execute(
    "insert into token (user, token, tokendate)
     values (?1, ?2, ?3)",
    params![user, token.to_string(), now],
  )?;

  Ok(())
}

pub fn purge_login_tokens(
  conn: &Connection,
  token_expiration_ms: i64,
) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  let expdt = now - token_expiration_ms;

  let count: i64 = conn.query_row(
    "select count(*) from
      token where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired token records", count);

    conn.execute(
      "delete from token
        where tokendate < ?1",
      params![expdt],
    )?;
  }

  Ok(())
}

pub fn purge_email_tokens(
  conn: &Connection,
  token_expiration_ms: i64,
) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  let expdt = now - token_expiration_ms;

  let count: i64 = conn.query_row(
    "select count(*) from
      newemail where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired newemail records", count);

    conn.execute(
      "delete from newemail
        where tokendate < ?1",
      params![expdt],
    )?;
  }

  Ok(())
}

pub fn purge_reset_tokens(
  conn: &Connection,
  token_expiration_ms: i64,
) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  let expdt = now - token_expiration_ms;

  let count: i64 = conn.query_row(
    "select count(*) from
      newpassword where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired newpassword records", count);

    conn.execute(
      "delete from newpassword
        where tokendate < ?1",
      params![expdt],
    )?;
  }

  Ok(())
}

fn purge_tokens(config: &Config) -> Result<(), Box<dyn Error>> {
  let conn = sqldata::connection_open(config.db.as_path())?;

  sqldata::purge_login_tokens(&conn, config.login_token_expiration_ms)?;

  sqldata::purge_email_tokens(&conn, config.email_token_expiration_ms)?;

  sqldata::purge_reset_tokens(&conn, config.reset_token_expiration_ms)?;

  Ok(())
}

pub fn update_user(conn: &Connection, user: &User) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "update user set name = ?1, hashwd = ?2, salt = ?3, email = ?4, registration_key = ?5, homenote = ?6
           where id = ?7",
    params![
      user.name,
      user.hashwd,
      user.salt,
      user.email,
      user.registration_key,
      user.homenoteid,
      user.id,
    ],
  )?;

  Ok(())
}

// email change request.
pub fn add_newemail(
  conn: &Connection,
  user: i64,
  token: Uuid,
  email: String,
) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  conn.execute(
    "insert into newemail (user, email, token, tokendate)
     values (?1, ?2, ?3, ?4)",
    params![user, email, token.to_string(), now],
  )?;

  Ok(())
}

// email change request.
pub fn read_newemail(
  conn: &Connection,
  user: i64,
  token: Uuid,
) -> Result<(String, i64), Box<dyn Error>> {
  let result = conn.query_row(
    "select email, tokendate from newemail
     where user = ?1
      and token = ?2",
    params![user, token.to_string()],
    |row| Ok((row.get(0)?, row.get(1)?)),
  )?;
  Ok(result)
}
// email change request.
pub fn remove_newemail(conn: &Connection, user: i64, token: Uuid) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "delete from newemail
     where user = ?1 and token = ?2",
    params![user, token.to_string()],
  )?;

  Ok(())
}

// password reset request.
pub fn add_newpassword(conn: &Connection, user: i64, token: Uuid) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  conn.execute(
    "insert into newpassword (user, token, tokendate)
     values (?1, ?2, ?3)",
    params![user, token.to_string(), now],
  )?;

  Ok(())
}

// password reset request.
pub fn read_newpassword(conn: &Connection, user: i64, token: Uuid) -> Result<i64, Box<dyn Error>> {
  let result = conn.query_row(
    "select tokendate from newpassword
     where user = ?1
      and token = ?2",
    params![user, token.to_string()],
    |row| Ok(row.get(0)?),
  )?;
  Ok(result)
}

// password reset request.
pub fn remove_newpassword(conn: &Connection, user: i64, token: Uuid) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "delete from newpassword
     where user = ?1 and token = ?2",
    params![user, token.to_string()],
  )?;

  Ok(())
}

pub fn change_password(
  conn: &Connection,
  uid: i64,
  cp: ChangePassword,
) -> Result<(), Box<dyn Error>> {
  let mut userdata = read_user_by_id(&conn, uid)?;
  match userdata.registration_key {
    Some(_reg_key) => bail!("invalid user or password"),
    None => {
      if hex_digest(
        Algorithm::SHA256,
        (cp.oldpwd.clone() + userdata.salt.as_str())
          .into_bytes()
          .as_slice(),
      ) != userdata.hashwd
      {
        // bad password, can't change.
        bail!("invalid password!")
      } else {
        let newhash = hex_digest(
          Algorithm::SHA256,
          (cp.newpwd.clone() + userdata.salt.as_str())
            .into_bytes()
            .as_slice(),
        );
        userdata.hashwd = newhash;
        update_user(&conn, &userdata)?;
        info!("changed password for {}", userdata.name);

        Ok(())
      }
    }
  }
}

pub fn change_email(
  conn: &Connection,
  uid: i64,
  cp: ChangeEmail,
) -> Result<(String, Uuid), Box<dyn Error>> {
  let userdata = read_user_by_id(&conn, uid)?;
  match userdata.registration_key {
    Some(_reg_key) => bail!("invalid user or password"),
    None => {
      if hex_digest(
        Algorithm::SHA256,
        (cp.pwd.clone() + userdata.salt.as_str())
          .into_bytes()
          .as_slice(),
      ) != userdata.hashwd
      {
        // bad password, can't change.
        bail!("invalid password!")
      } else {
        // create a 'newemail' record.
        let token = Uuid::new_v4();
        add_newemail(&conn, uid, token, cp.email)?;

        Ok((userdata.name, token))
      }
    }
  }
}
