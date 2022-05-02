use crate::data::Config;
use crate::data::{ChangeEmail, ChangePassword, LoginData, User};
// use crate::email;
use crate::util::{is_token_expired, now};
// use actix_session::Session;
// use actix_web::HttpRequest;
// use barrel::backend::Sqlite;
// use barrel::{types, Migration};
use crypto_hash::{hex_digest, Algorithm};
// use either::Either::{Left, Right};
use log::info;
use rusqlite::{params, Connection};
// use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

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

pub fn new_user(
  conn: &Connection,
  name: String,
  hashwd: String,
  salt: String,
  email: String,
  registration_key: String,
) -> Result<i64, Box<dyn Error>> {
  let now = now()?;

  // make a user record.
  conn.execute(
    "insert into orgauth_user (name, hashwd, salt, email, registration_key, createdate)
      values (?1, ?2, ?3, ?4, ?5, ?6)",
    params![name, hashwd, salt, email, registration_key, now],
  )?;

  let uid = conn.last_insert_rowid();

  Ok(uid)
}

pub fn user_id(conn: &Connection, name: &str) -> Result<i64, Box<dyn Error>> {
  let id: i64 = conn.query_row(
    "select id from orgauth_user
      where orgauth_user.name = ?1",
    params![name],
    |row| Ok(row.get(0)?),
  )?;
  Ok(id)
}

pub fn login_data(conn: &Connection, uid: i64) -> Result<LoginData, Box<dyn Error>> {
  let user = read_user_by_id(&conn, uid)?;
  Ok(LoginData {
    userid: uid,
    name: user.name,
    data: None,
  })
}

pub fn login_data_cb(
  conn: &Connection,
  uid: i64,
  mut extra_login_data: Box<
    dyn FnMut(&Connection, i64) -> Result<Option<serde_json::Value>, Box<dyn Error>>,
  >,
) -> Result<LoginData, Box<dyn Error>> {
  let user = read_user_by_id(&conn, uid)?;
  Ok(LoginData {
    userid: uid,
    name: user.name,
    data: extra_login_data(&conn, uid)?,
  })
}

pub fn read_user_by_name(conn: &Connection, name: &str) -> Result<User, Box<dyn Error>> {
  let user = conn.query_row(
    "select id, hashwd, salt, email, registration_key
      from orgauth_user where name = ?1",
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

pub fn read_user_by_id(conn: &Connection, id: i64) -> Result<User, Box<dyn Error>> {
  let user = conn.query_row(
    "select id, name, hashwd, salt, email, registration_key
      from orgauth_user where id = ?1",
    params![id],
    |row| {
      Ok(User {
        id: row.get(0)?,
        name: row.get(1)?,
        hashwd: row.get(2)?,
        salt: row.get(3)?,
        email: row.get(4)?,
        registration_key: row.get(5)?,
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
    "select id, name, hashwd, salt, email, registration_key, orgauth_token.tokendate
      from orgauth_user, orgauth_token where orgauth_user.id = orgauth_token.user and orgauth_token.token = ?1",
    params![token.to_string()],
    |row| {
      Ok((
        User {
          id: row.get(0)?,
          name: row.get(1)?,
          hashwd: row.get(2)?,
          salt: row.get(3)?,
          email: row.get(4)?,
          registration_key: row.get(5)?,
        },
        row.get(6)?,
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
    "insert into orgauth_token (user, token, tokendate)
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
      orgauth_token where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired orgauth_token records", count);

    conn.execute(
      "delete from orgauth_token
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
      orgauth_newemail where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired orgauth_newemail records", count);

    conn.execute(
      "delete from orgauth_newemail
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
      orgauth_newpassword where tokendate < ?1",
    params![expdt],
    |row| Ok(row.get(0)?),
  )?;

  if count > 0 {
    info!("removing {} expired orgauth_newpassword records", count);

    conn.execute(
      "delete from orgauth_newpassword
        where tokendate < ?1",
      params![expdt],
    )?;
  }

  Ok(())
}

pub fn purge_tokens(config: &Config) -> Result<(), Box<dyn Error>> {
  let conn = connection_open(config.db.as_path())?;

  purge_login_tokens(&conn, config.login_token_expiration_ms)?;

  purge_email_tokens(&conn, config.email_token_expiration_ms)?;

  purge_reset_tokens(&conn, config.reset_token_expiration_ms)?;

  Ok(())
}

pub fn update_user(conn: &Connection, user: &User) -> Result<(), Box<dyn Error>> {
  conn.execute(
    "update orgauth_user set name = ?1, hashwd = ?2, salt = ?3, email = ?4, registration_key = ?5
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
    "insert into orgauth_newemail (user, email, token, tokendate)
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
    "select email, tokendate from orgauth_newemail
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
    "delete from orgauth_newemail
     where user = ?1 and token = ?2",
    params![user, token.to_string()],
  )?;

  Ok(())
}

// password reset request.
pub fn add_newpassword(conn: &Connection, user: i64, token: Uuid) -> Result<(), Box<dyn Error>> {
  let now = now()?;
  conn.execute(
    "insert into orgauth_newpassword (user, token, tokendate)
     values (?1, ?2, ?3)",
    params![user, token.to_string(), now],
  )?;

  Ok(())
}

// password reset request.
pub fn read_newpassword(conn: &Connection, user: i64, token: Uuid) -> Result<i64, Box<dyn Error>> {
  let result = conn.query_row(
    "select tokendate from orgauth_newpassword
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
    "delete from orgauth_newpassword
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
