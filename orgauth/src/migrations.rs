use barrel::backend::Sqlite;
use barrel::{types, Migration};
use rusqlite::Connection;
use std::error::Error;
use std::path::Path;

pub fn udpate1(dbfile: &Path) -> Result<(), Box<dyn Error>> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  let mut m = Migration::new();

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

  // add token table.  multiple tokens per user to support multiple browsers and/or devices.
  m.create_table("orgauth_token", |t| {
    t.add_column("user", types::foreign("orgauth_user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "orgauth_tokenunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  // add newemail table.  each request for a new email creates an entry.
  m.create_table("orgauth_newemail", |t| {
    t.add_column("user", types::foreign("orgauth_user", "id").nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "orgauth_newemailunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  // add newpassword table.  each request for a new password creates an entry.
  m.create_table("orgauth_newpassword", |t| {
    t.add_column("user", types::foreign("orgauth_user", "id").nullable(false));
    t.add_column("token", types::text().nullable(false));
    t.add_column("tokendate", types::integer().nullable(false));
    t.add_index(
      "orgauth_resetpasswordunq",
      types::index(vec!["user", "token"]).unique(true),
    );
  });

  conn.execute_batch(m.make::<Sqlite>().as_str())?;

  Ok(())
}
