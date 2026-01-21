use crate::error as zkerr;
use crate::util::{now, nowms};
use barrel::backend::Sqlite;
use barrel::{types, Migration};
use orgauth::migrations;
use regex::{Captures, Regex};
use rusqlite::{params, Connection};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use tracing::{error, info};
use uuid::Uuid;
use zkprotocol::constants::SpecialUuids;
use zkprotocol::search::TagSearch;
use zkprotocol::specialnotes as SN;

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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zk",
      types::foreign(
        "zk",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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
    t.add_column(
      "zk",
      types::foreign(
        "zk",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  m.create_table("zklink", |t| {
    t.add_column(
      "zkleft",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zkright",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzk",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zk",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zk",
      types::foreign(
        "zk",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_index(
      "unq",
      types::index(vec!["fromid", "toid", "zk"]).unique(true),
    );
  });

  m
}

pub fn udpate3(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "zk",
      types::foreign(
        "zk",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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
    t.add_column(
      "zk",
      types::foreign(
        "zk",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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

pub fn udpate4(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });
  m1.create_table("zklinktemp", |t| {
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  // new zklink with column 'user' instead of 'zk'.
  m2.create_table("zklink", |t| {
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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

pub fn udpate5(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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

pub fn udpate6(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("salt", types::text().nullable(false));
    t.add_column("email", types::text().nullable(false));
    t.add_column("registration_key", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  // add token table.  multiple tokens per user to support multiple browsers and/or devices.
  m2.create_table("token", |t| {
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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

pub fn udpate7(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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

pub fn udpate8(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  let pubid: i64 = conn.query_row(
    "select zknote.id from
      zknote, user
      where zknote.title = ?2
      and user.name = ?1
      and zknote.user = user.id",
    params!["system", "public"],
    |row| Ok(row.get(0)?),
  )?;
  let sysid: i64 = conn.query_row(
    "select id from user
      where user.name = ?1",
    params!["system"],
    |row| Ok(row.get(0)?),
  )?;
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

pub fn udpate9(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
  let mut m1 = Migration::new();

  // add newemail table.  each request for a new email creates an entry.
  m1.create_table("newemail", |t| {
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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

pub fn udpate10(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column(
      "homenote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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

pub fn udpate11(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
  let mut m1 = Migration::new();

  // add newpassword table.  each request for a new password creates an entry.
  m1.create_table("newpassword", |t| {
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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

pub fn udpate12(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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
    t.add_column(
      "user",
      types::foreign(
        "user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
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
pub fn udpate13(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

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
        select user, token, tokendate from newpassword",
    params![],
  )?;

  // Hmmm would switch to 'zkuser', but I'll keep the user table so I don't have to rebuild every table with
  // new foreign keys.

  let mut m1 = Migration::new();

  m1.create_table("usertemp", |t| {
    t.add_column(
      "id",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column(
      "homenote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

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
    t.add_column(
      "id",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "zknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column(
      "homenote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
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

  // --------------------------------------------------------------------------------------
  // remake zknotes table to point at orgauth_user instead of user.

  conn.execute(
    "CREATE TABLE IF NOT EXISTS \"zklinktemp\" (
      \"fromid\" INTEGER REFERENCES zknote(id) NOT NULL,
      \"toid\" INTEGER REFERENCES zknote(id) NOT NULL,
      \"user\" INTEGER REFERENCES user(id) NOT NULL,
      \"linkzknote\" INTEGER REFERENCES zknote(id))",
    params![],
  )?;

  conn.execute(
    "CREATE TABLE IF NOT EXISTS \"zknotetemp\" (
      \"id\" INTEGER PRIMARY KEY NOT NULL,
      \"title\" TEXT NOT NULL,
      \"content\" TEXT NOT NULL,
      \"sysdata\" TEXT,
      \"pubid\" TEXT UNIQUE,
      \"user\" INTEGER REFERENCES user(id) NOT NULL,
      \"editable\" BOOLEAN NOT NULL,
      \"showtitle\" BOOLEAN NOT NULL,
      \"createdate\" INTEGER NOT NULL,
      \"changeddate\" INTEGER NOT NULL)",
    params![],
  )?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, showtitle, createdate, changeddate)
        select id, title, content, null, pubid, user, editable, showtitle, createdate, changeddate from zknote",
    params![],
  )?;

  // copy everything from zklink.
  conn.execute(
    "insert into zklinktemp (fromid, toid, user, linkzknote)
        select fromid, toid, user, linkzknote from zklink",
    params![],
  )?;

  // drop tables.
  conn.execute("drop table zknote", params![])?;
  conn.execute("drop table zklink", params![])?;

  // new tables referencing orgauth_user.
  conn.execute(
    "CREATE TABLE IF NOT EXISTS \"zklink\" (
      \"fromid\" INTEGER REFERENCES zknote(id) NOT NULL,
      \"toid\" INTEGER REFERENCES zknote(id) NOT NULL,
      \"user\" INTEGER REFERENCES orgauth_user(id) NOT NULL,
      \"linkzknote\" INTEGER REFERENCES zknote(id))",
    params![],
  )?;

  conn.execute(
    "CREATE TABLE IF NOT EXISTS \"zknote\" (
      \"id\" INTEGER PRIMARY KEY NOT NULL,
      \"title\" TEXT NOT NULL,
      \"content\" TEXT NOT NULL,
      \"sysdata\" TEXT,
      \"pubid\" TEXT UNIQUE,
      \"user\" INTEGER REFERENCES orgauth_user(id) NOT NULL,
      \"editable\" BOOLEAN NOT NULL,
      \"showtitle\" BOOLEAN NOT NULL,
      \"createdate\" INTEGER NOT NULL,
      \"changeddate\" INTEGER NOT NULL)",
    params![],
  )?;

  conn.execute(
    "CREATE UNIQUE INDEX \"zklinkunq\" ON \"zklink\" (\"fromid\", \"toid\", \"user\")",
    params![],
  )?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, createdate, changeddate)
        select id, title, content, null, pubid, user, editable, showtitle, createdate, changeddate from zknotetemp",
    params![],
  )?;

  // copy everything from zklinktemp.
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote)
        select fromid, toid, user, linkzknote from zklinktemp",
    params![],
  )?;

  // drop temp tables.
  conn.execute("drop table zknotetemp", params![])?;
  conn.execute("drop table zklinktemp", params![])?;

  Ok(())
}

pub fn udpate14(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate2(dbfile)?;
  Ok(())
}

pub fn udpate15(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate3(dbfile)?;
  Ok(())
}

pub fn udpate16(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate4(dbfile)?;
  Ok(())
}

pub fn udpate17(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // Add archive system note.

  let conn = Connection::open(dbfile)?;

  // get "public" note.
  let pubid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "public"],
    |row| Ok(row.get(0)?),
  )?;

  // get "system" userid.
  let sysid: i64 = conn.query_row(
    "select id from orgauth_user
      where orgauth_user.name = ?1",
    params!["system"],
    |row| Ok(row.get(0)?),
  )?;
  let now = now()?;

  // new note.
  conn.execute(
    "insert into zknote (title, content, pubid, editable, showtitle, user, createdate, changeddate)
      values ('archive', '', null, 0, 0, ?1, ?2, ?3)",
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

pub fn udpate18(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, showtitle, createdate, changeddate)
        select id, title, content, null, pubid, user, editable, showtitle, createdate, changeddate from zknote",
    params![],
  )?;

  let mut m2 = Migration::new();

  m2.drop_table("zknote");

  // new zknote with deleted column
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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, createdate, changeddate)
        select id, title, content, null, pubid, user, 0, 1, 0, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate19(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  // we don't just call note_id because note_id ftn of the future may be
  // different!  then migrations would break.  yes this happened.
  // let archiveid = note_id(&conn, "system", "archive")?;
  let archiveid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "archive"],
    |row| Ok(row.get(0)?),
  )?;
  // flag links from archive notes to their targets with archive note id.
  // so they get skipped in searches.
  conn.execute(
    "update zklink set linkzknote = ?1
      where fromid in (select fromid from zklink where toid = ?1)
        and toid is not ?1",
    params![archiveid],
  )?;

  Ok(())
}

pub fn udpate20(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate5(dbfile)?;
  Ok(())
}

pub fn udpate21(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  let mut m = Migration::new();

  m.create_table("file", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("hash", types::text().nullable(false).unique(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m.make::<Sqlite>().as_str())?;

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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, createdate, changeddate from zknote",
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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column(
      "file",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate22(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate6(dbfile)?;
  Ok(())
}

pub fn udpate23(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate7(dbfile)?;
  Ok(())
}

pub fn udpate24(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  let mut m1 = Migration::new();

  m1.create_table("filetemp", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("hash", types::text().nullable(false).unique(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  conn.execute(
    "insert into filetemp (id, hash, createdate)
      select id, hash, createdate from file",
    params![],
  )?;

  let mut m2 = Migration::new();

  m2.drop_table("file");

  m2.create_table("file", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("hash", types::text().nullable(false).unique(true));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("size", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  let mut pstmt = conn.prepare("select id, hash, createdate from filetemp")?;
  let r: Vec<(i64, String, i64)> = pstmt
    .query_map(params![], |row| {
      let id: i64 = row.get(0)?;
      let s: String = row.get(1)?;
      let i: i64 = row.get(2)?;
      Ok((id, s, i))
    })?
    .filter_map(|x| x.ok())
    .collect();

  for (id, hash, createdate) in r {
    // get file size.
    let pstr = format!("files/{}", hash);
    let stpath = Path::new(pstr.as_str());
    let md = std::fs::metadata(stpath)?;
    let size = md.len();

    conn.execute(
      "insert  into file (id, hash, createdate, size)
      values (?1, ?2, ?3, ?4)",
      params![id, hash, createdate, size],
    )?;
  }

  Ok(())
}

pub fn udpate25(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  let mut m1 = Migration::new();

  // files yeeted with youtube-dl.
  m1.create_table("yeetfile", |t| {
    t.add_column("yeetkey", types::text().nullable(false));
    t.add_column("audio", types::boolean().nullable(false));
    t.add_column("filename", types::integer().nullable(false));
    t.add_column(
      "fileid",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Cascade,
        types::ReferentialAction::Cascade,
      )
      .nullable(false),
    );
    t.add_index("unq", types::index(vec!["yeetkey", "audio"]).unique(true));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate26(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  let mut m1 = Migration::new();

  // files yeeted with youtube-dl.
  m1.drop_table("yeetfile");

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate27(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // add uuids to zknotes table.

  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column(
      "file",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("uuid", types::text().nullable(true));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, createdate, changeddate from zknote",
    params![],
  )?;

  // now generate a uuid for every note.
  let mut pstmt = conn.prepare("select id from zknote")?;
  let ids: Vec<i64> = pstmt
    .query_map(params![], |row| Ok(row.get(0)?))?
    .filter_map(|x| x.ok())
    .collect();

  // this is horrifically slow
  for id in ids {
    let snow = nowms()?;
    let uuid = uuid::Uuid::new_v4();
    let unow = nowms()?;
    println!("uuid duration: {}", unow - snow);

    conn.execute(
      "update zknotetemp set uuid = ?1 where id = ?2",
      params![uuid.to_string(), id],
    )?;
    let upnow = nowms()?;
    println!("update duration: {}", upnow - unow);

    println!("updated {} {}", id, uuid);
  }

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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column(
      "file",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("uuid", types::text().nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
    t.add_index("unq_uuid", types::index(vec!["uuid"]).unique(true));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, createdate, changeddate from zknotetemp",
    params![],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate28(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
  let mut m1 = Migration::new();

  m1.create_table("zklinktemp", |t| {
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_index(
      "unqtemp",
      types::index(vec!["fromid", "toid", "user"]).unique(true),
    );
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  // copy everything from zklink.
  conn.execute(
    "insert into zklinktemp (fromid, toid, user, linkzknote)
        select fromid, toid, user, linkzknote from zklink",
    params![],
  )?;

  let mut m2 = Migration::new();
  // drop zknote.
  m2.drop_table("zklink");

  // new zklink with column 'user' instead of 'zk'.
  m2.create_table("zklink", |t| {
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_index(
      "zklinkunq",
      types::index(vec!["fromid", "toid", "user"]).unique(true),
    );
  });

  // archive table.  each time a link is deleted, create a record containing the
  // original link create date, and the link delete date.
  m2.create_table("zklinkarchive", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column(
      "fromid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "toid",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "linkzknote",
      types::foreign(
        "zknote",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("deletedate", types::integer().nullable(false));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  let now = now()?;

  // copy everything from zklinktemp, adding dates.
  conn.execute(
    "insert into zklink (fromid, toid, user, linkzknote, createdate)
        select fromid, toid, user, linkzknote, ?1 from zklinktemp",
    params![now],
  )?;

  let mut m3 = Migration::new();
  m3.drop_table("zklinktemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  Ok(())
}

pub fn udpate29(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  orgauth::migrations::udpate8(dbfile)?;
  Ok(())
}

pub fn udpate30(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // db connection without foreign key checking.
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  // update system notes with specific uuids.
  let archiveid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "archive"],
    |row| Ok(row.get(0)?),
  )?;

  let update_note_id = |title, uuid| {
    conn.execute(
      "update zknote set uuid = ?1
      where zknote.title = ?2
        and not exists (select * from zklink where fromid = zknote.id and toid = ?3)
        and user in (select id from orgauth_user where name = 'system')",
      params![uuid, title, archiveid],
    )
  };

  update_note_id("public", SpecialUuids::Public.str())?;
  update_note_id("share", SpecialUuids::Share.str())?;
  update_note_id("search", SpecialUuids::Search.str())?;
  update_note_id("user", SpecialUuids::User.str())?;
  update_note_id("archive", SpecialUuids::Archive.str())?;
  update_note_id("comment", SpecialUuids::Comment.str())?;

  // update system user uuid.
  conn.execute(
    "update orgauth_user set uuid = ?1
      where name= 'system'",
    params![SpecialUuids::System.str()],
  )?;

  Ok(())
}

fn replace_all<E>(
  re: &Regex,
  haystack: &str,
  replacement: impl Fn(&Captures) -> Result<String, E>,
) -> Result<String, E> {
  let mut new = String::with_capacity(haystack.len());
  let mut last_match = 0;
  for caps in re.captures_iter(haystack) {
    let m = caps.get(1).unwrap();
    new.push_str(&haystack[last_match..m.start()]);
    new.push_str(&replacement(&caps)?);
    last_match = m.end();
  }

  new.push_str(&haystack[last_match..]);
  Ok(new)
}

#[test]
fn testreplaceall() {
  let meh = r#"<panel noteid="16340"/>
### the old nix-channels way

I'm used to having multiple nix channels in my system, like this:

```
[bburdette@HOSS:/etc/nixos]$ sudo nix-channel --list
nixos https://nixos.org/channels/nixos-22.11
nixos-unstable https://nixos.org/channels/nixos-unstable
```

And then in my configuration.nix I typically do

```
{ config, pkgs, ... }:

let"#;
  let replaceid = |caps: &Captures| {
    if let Ok(id) = caps[1].parse::<i64>() {
      assert!(id == 16340);
      println!("id == 16340");
    } else {
      // bad id.  leave it.
      println!("bad: {}", String::from(&caps[1]));
      assert!(false);
    }
    Ok::<String, zkerr::Error>("replaced".to_string())
  };

  let panelstyle = Regex::new(r#"\<panel noteid=\"([0-9]+)\"/>"#).unwrap();
  let replaced = replace_all(&panelstyle, meh, replaceid).unwrap();
  println!("{}", replaced);
  assert!(replaced.find(r#"<panel noteid="replaced""#) != None);
}

// replace note ids with uuids in hyperlinks.
pub fn udpate31(dbfile: &Path) -> Result<(), zkerr::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;

  let mut stmt = conn.prepare("select zknote.id, zknote.content from zknote")?;

  let notes = stmt
    .query_map(params![], |row| {
      Ok((row.get::<usize, i64>(0)?, row.get::<usize, String>(1)?))
    })?
    .filter_map(|x| x.ok());

  let idconn = Connection::open(dbfile)?;
  let replaceid = |caps: &Captures| {
    if let Ok(id) = caps[1].parse::<i64>() {
      let rt = idconn.query_row("select uuid from zknote where id=?1", params![id], |row| {
        row.get::<usize, String>(0)
      });
      match rt {
        Ok(rt) => Ok::<String, zkerr::Error>(rt),
        Err(_) => Ok(String::from(&caps[1])), // note id not found.  leave it.
      }
    } else {
      // bad id.  leave it.
      Ok(String::from(&caps[1]))
    }
  };

  let oldstyle = Regex::new(r#"\[.+\]\(\/note\/([0-9]+)\)"#)?;
  let newstyle = Regex::new(r#"\<note id=\"([0-9]+)\"/>"#)?;
  let panelstyle = Regex::new(r#"\<panel noteid=\"([0-9]+)\"/>"#)?;

  for (id, text) in notes {
    // search and replace the three note styles:
    // `<note id="20050"/>`
    // `[this kind](/note/20077).`
    // '<panel noteid=\"16340\"/>'

    let r = replace_all(&oldstyle, text.as_str(), replaceid)?;
    let r = replace_all(&newstyle, r.as_str(), replaceid)?;
    let r = replace_all(&panelstyle, r.as_str(), replaceid)?;

    if r != text {
      conn.execute("update zknote set content=?1 where id = ?2", params![r, id])?;
    }
  }

  Ok(())
}

// add 'sync' system note.
pub fn udpate32(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;

  let sysid: i64 = conn.query_row(
    "select id from orgauth_user
      where orgauth_user.name = ?1",
    params!["system"],
    |row| Ok(row.get(0)?),
  )?;
  let now = now()?;

  let publicid: i64 = conn.query_row(
    "select zknote.id from
      zknote where uuid = ?1",
    params![SpecialUuids::Public.str()],
    |row| Ok(row.get(0)?),
  )?;

  conn.execute(
    "insert into zknote (title, content, showtitle, editable, deleted, user, uuid, createdate, changeddate)
      values ('sync', '', 0, 0, 0, ?1, ?2, ?3, ?4)",
    params![sysid, SpecialUuids::Sync.str(), now, now],
  )?;

  let id = conn.last_insert_rowid();

  conn.execute(
    "insert into zklink (fromid, toid, user, createdate) 
    values (?1, ?2, ?3, ?4)",
    params![id, publicid, sysid, now],
  )?;

  Ok(())
}

// default system notes get 0 dates so they won't generate archive notes on first sync.
pub fn udpate33(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;

  let mut ids = Vec::new();

  ids.push(format!("'{}'", SpecialUuids::Public.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::Comment.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::Share.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::Search.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::User.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::Archive.str()).to_string());
  ids.push(format!("'{}'", SpecialUuids::Sync.str()).to_string());

  conn.execute(
    format!(
      "update zknote set createdate = 0, changeddate = 0 where uuid in ({})",
      ids.join(",").as_str()
    )
    .as_str(),
    params![],
  )?;

  Ok(())
}

// add unique server id in singlevalue table.
pub fn udpate34(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;

  let sid = Uuid::new_v4();

  conn.execute(
    "insert into singlevalue (name, value) values (?1, ?2)",
    params!["server_id", sid.to_string()],
  )?;

  // sync id for unique sync work table name
  conn.execute(
    "insert into singlevalue (name, value) values (?1, ?2)",
    params!["sync_id", 0],
  )?;

  Ok(())
}

// file_source table
pub fn udpate35(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
  let mut m = Migration::new();

  m.create_table("file_source", |t| {
    t.add_column(
      "file_id",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column(
      "user_id",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_index(
      "file_source_unq",
      types::index(vec!["file_id", "user_id"]).unique(true),
    );
  });

  conn.execute_batch(m.make::<Sqlite>().as_str())?;

  Ok(())
}

// note origin...
pub fn udpate36(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // add uuids to zknotes table.

  let conn = Connection::open(dbfile)?;
  conn.execute("PRAGMA foreign_keys = false;", params![])?;
  let tr = conn.unchecked_transaction()?;

  let mut m1 = Migration::new();

  m1.create_table("server", |t| {
    t.add_column(
      "id",
      types::integer()
        .primary(true)
        .increments(true)
        .nullable(false),
    );
    t.add_column("uuid", types::text().nullable(false).unique(true));
    t.add_column("createdate", types::integer().nullable(false));
  });

  // new zknote with showtitle column
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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column(
      "file",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("uuid", types::text().nullable(false));
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
  });

  conn.execute_batch(m1.make::<Sqlite>().as_str())?;

  let now = now()?;
  // make a server entry.
  let server_id = conn.execute(
    "insert into server (uuid, createdate) select value, ?1 from singlevalue where name = 'server_id'",
    params![now],
  )?;

  // copy everything from zknote.
  conn.execute(
    "insert into zknotetemp (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, createdate, changeddate from zknote",
    params![],
  )?;

  let mut m2 = Migration::new();

  m2.drop_table("zknote");

  // new zknote with new column
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
    t.add_column(
      "user",
      types::foreign(
        "orgauth_user",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(false),
    );
    t.add_column("editable", types::boolean());
    t.add_column("showtitle", types::boolean());
    t.add_column("deleted", types::boolean());
    t.add_column(
      "file",
      types::foreign(
        "file",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      )
      .nullable(true),
    );
    t.add_column("uuid", types::text().nullable(false));
    t.add_column(
      "server",
      types::foreign(
        "server",
        "id",
        types::ReferentialAction::Restrict,
        types::ReferentialAction::Restrict,
      ),
    );
    t.add_column("createdate", types::integer().nullable(false));
    t.add_column("changeddate", types::integer().nullable(false));
    t.add_index("unq_uuid", types::index(vec!["uuid"]).unique(true));
  });

  conn.execute_batch(m2.make::<Sqlite>().as_str())?;

  // copy everything from zknotetemp, plus server id.
  conn.execute(
    "insert into zknote (id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, server, createdate, changeddate)
        select id, title, content, sysdata, pubid, user, editable, showtitle, deleted, file, uuid, ?1, createdate, changeddate from zknotetemp",
    params![server_id],
  )?;

  let mut m3 = Migration::new();

  m3.drop_table("zknotetemp");

  conn.execute_batch(m3.make::<Sqlite>().as_str())?;

  tr.commit()?;

  Ok(())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OldSync {
  pub after: Option<i64>,
  pub now: i64,
}

// modify search and sync notes to new format.
pub fn udpate37(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // find notes that are owned by system, linked to 'sync', and the contents deserialize to an old sync record.

  let conn = Connection::open(dbfile)?;
  let tr = conn.unchecked_transaction()?;

  // implement here instead of using sqldata functions, since those functions may change in the future!
  let searchid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "search"],
    |row| Ok(row.get(0)?),
  )?;

  let syncid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "sync"],
    |row| Ok(row.get(0)?),
  )?;

  // find notes linked to 'search' where the contents deserialize to zknotesearch.  post failures.
  let mut pstmt = conn.prepare(
    "select zknote.id, content from zknote, zklink
    where zklink.fromid = zknote.id
    and zklink.toid = ?1",
  )?;
  let r: Vec<(i64, String)> = pstmt
    .query_map(params![searchid], |row| {
      let id: i64 = row.get(0)?;
      let s: String = row.get(1)?;
      Ok((id, s))
    })?
    .filter_map(|x| x.ok())
    .collect();

  for (id, content) in r {
    match serde_json::from_str::<Vec<TagSearch>>(content.as_str()) {
      Ok(zn) => {
        let sn = SN::SpecialNote::SnSearch(zn);

        let sns = serde_json::to_string(&serde_json::to_value(sn)?)?;

        // update without changing changed date!
        conn.execute(
          "update zknote set content = ?1 where id = ?2",
          params![sns, id],
        )?;
        info!("updated search {}", id);
      }
      _ => match serde_json::from_str::<TagSearch>(content.as_str()) {
        Ok(zn) => {
          let sn = SN::SpecialNote::SnSearch(vec![zn]);

          let sns = serde_json::to_string(&serde_json::to_value(sn)?)?;

          // update without changing changed date!
          conn.execute(
            "update zknote set content = ?1 where id = ?2",
            params![sns, id],
          )?;
          info!("updated search {}", id);
        }
        Err(e) => {
          error!("search update failed: {} {:?}\n{}", id, e, content);
        }
      },
    };
  }

  println!("searching for sync {}", syncid);
  // find notes linked to 'sync' where the contents deserialize to sync.  post failures.
  let mut pstmt = conn.prepare(
    "select zknote.id, content from zknote, zklink
    where zklink.fromid = zknote.id
    and zklink.toid = ?1",
  )?;
  let r: Vec<(i64, String)> = pstmt
    .query_map(params![syncid], |row| {
      let id: i64 = row.get(0)?;
      let s: String = row.get(1)?;
      Ok((id, s))
    })?
    .filter_map(|x| x.ok())
    .collect();

  for (id, content) in r {
    match serde_json::from_str::<OldSync>(content.as_str()) {
      Ok(oldsync) => {
        let sn = SN::SpecialNote::SnSync(SN::CompletedSync {
          after: oldsync.after,
          now: oldsync.now,
          remote: None,
        });

        let sns = serde_json::to_string(&serde_json::to_value(sn)?)?;

        // update without changing changed date!
        conn.execute(
          "update zknote set content = ?1 where id = ?2",
          params![sns, id],
        )?;
        info!("updated sync: {}, {:?}", id, sns);
      }
      Err(e) => {
        error!("oldsync parse error: {:?}", e);
      }
    };
  }

  tr.commit()?;

  Ok(())
}

pub fn udpate38(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // delete mistakenly archived searches, fix changeddates"

  let conn = Connection::open(dbfile)?;
  let tr = conn.unchecked_transaction()?;

  let searchid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "search"],
    |row| Ok(row.get(0)?),
  )?;

  let archiveid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "archive"],
    |row| Ok(row.get(0)?),
  )?;

  let updatecount: usize = conn.execute(
    "update zknote set changeddate = createdate
      where zknote.id in
      (select zknote.id from
      zknote, orgauth_user, zklink
      where zklink.fromid = zknote.id
      and zklink.toid = ?2
      and orgauth_user.name = ?1
      and zknote.user != orgauth_user.id)",
    params!["system", searchid],
  )?;

  info!("updated {} search records", updatecount);

  let mut pstmt = conn.prepare(
    "select N.id, N.title from zknote N, zklink A, zklink B, zklink C where
      A.fromid = N.id
      and A.toid = ?1
      and B.fromid = N.id
      and C.fromid = B.toid
      and C.toid = ?2",
  )?;

  let r: Vec<(i64, String)> = pstmt
    .query_map(params![archiveid, searchid], |row| {
      let id: i64 = row.get(0)?;
      let title: String = row.get(1)?;
      // info!("title {:?}", row.get::<usize, String>(1)?);
      Ok((id, title))
    })?
    .filter_map(|x| x.ok())
    .collect();

  let deletecount = r.len();

  for (id, title) in r {
    info!("deleting record {}, {}", id, title);
    conn.execute("delete from zklink where fromid = ?1", params![id])?;
    conn.execute("delete from zknote where id = ?1", params![id])?;
  }

  info!("deleted {} archived search records", deletecount);

  tr.commit()?;

  Ok(())
}

pub fn udpate39(dbfile: &Path) -> Result<(), orgauth::error::Error> {
  // delete mistakenly archived searches, fix changeddates"

  let conn = Connection::open(dbfile)?;
  let tr = conn.unchecked_transaction()?;

  conn.execute(
    "CREATE TABLE IF NOT EXISTS \"zkarch\"
	(\"id\" INTEGER PRIMARY KEY NOT NULL,
	 \"title\" TEXT NOT NULL,
	 \"content\" TEXT NOT NULL,
	 \"sysdata\" TEXT,
	 \"pubid\" TEXT UNIQUE,
	 \"user\" INTEGER NOT NULL REFERENCES orgauth_user(id) ON UPDATE RESTRICT ON DELETE RESTRICT,
	 \"editable\" BOOLEAN NOT NULL,
	 \"showtitle\" BOOLEAN NOT NULL,
	 \"deleted\" BOOLEAN NOT NULL,
	 \"file\" INTEGER REFERENCES file(id) ON UPDATE RESTRICT ON DELETE RESTRICT,
	 \"uuid\" TEXT NOT NULL,
	 \"server\" INTEGER NOT NULL REFERENCES server(id) ON UPDATE RESTRICT ON DELETE RESTRICT,
	 \"createdate\" INTEGER NOT NULL,
	 \"changeddate\" INTEGER NOT NULL);",
    params![],
  )?;

  // update system notes with specific uuids.
  let archiveid: i64 = conn.query_row(
    "select zknote.id from
      zknote, orgauth_user
      where zknote.title = ?2
      and orgauth_user.name = ?1
      and zknote.user = orgauth_user.id",
    params!["system", "archive"],
    |row| Ok(row.get(0)?),
  )?;

  conn.execute(
    "insert into zkarch
    ( id, title, content, sysdata, pubid, user, editable, showtitle,
    deleted, file, uuid, server, createdate, changeddate)
	select  id, title, content, sysdata, pubid, user, editable, showtitle,
	deleted, file, uuid, server, createdate, changeddate
	from zknote where zknote.id in (select fromid from zklink where toid = ?1); ",
    params![archiveid],
  )?;

  conn.execute("delete from zklink where toid = ?1; ", params![archiveid])?;

  conn.execute(
    "delete from zknote where zknote.id in (select id from zkarch); ",
    params![archiveid],
  )?;

  tr.commit()?;

  Ok(())
}
