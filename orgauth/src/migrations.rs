use crate::util::{is_token_expired, now};
use barrel::backend::Sqlite;
use barrel::{types, Migration};
use crypto_hash::{hex_digest, Algorithm};
use log::info;
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use simple_error::bail;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

pub fn udpate1(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m = Migration::new();

  // table for storing single values.
  // m.create_table("orgauth_singlevalue", |t| {
  //   t.add_column("name", types::text().nullable(false).unique(true));
  //   t.add_column("value", types::text().nullable(false));
  // });

  // add token table.  multiple tokens per user to support multiple browsers and/or devices.
  m.create_table("orgauth_token", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index("tokenunq", types::index(vec!["user", "token"]).unique(true));
  });

  // add newemail table.  each request for a new email creates an entry.
  m.create_table("orgauth_newemail", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "newemailunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  // new user table with new columns for session tokens.
  m.create_table("orgauth_user", |t| {
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

  // add newpassword table.  each request for a new password creates an entry.
  m.create_table("orgauth_newpassword", |t| {
    t.add_column("user", types::foreign("user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "resetpasswordunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  conn.execute_batch(m.make::<Sqlite>().as_str())?;

  Ok(())
}
