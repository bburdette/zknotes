use crate::sqldata;
use crate::sqldata::{delete_zknote, get_sysids, note_id};
use bytes::Bytes;
use either::Either;
use either::Either::{Left, Right};
use futures::Stream;
use orgauth::dbfun::user_id;
use rusqlite::{Connection, MappedRows, Row};
use std::convert::TryInto;
use std::error::Error;
use std::iter::IntoIterator;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use zkprotocol::content::ZkListNote;
use zkprotocol::search::{
  AndOr, SearchMod, TagSearch, ZkListNoteSearchResult, ZkNoteSearch, ZkNoteSearchResult,
};

pub fn power_delete_zknotes(
  conn: &Connection,
  file_path: PathBuf,
  user: i64,
  search: &TagSearch,
) -> Result<i64, Box<dyn Error>> {
  // get all, and delete all.  Maybe not a good idea for a big database, but ours is small
  // and soon to be replaced with indradb, perhaps.

  let nolimsearch = ZkNoteSearch {
    tagsearch: search.clone(),
    offset: 0,
    limit: None,
    what: "".to_string(),
    list: true,
    archives: false,
    created_after: None,
    created_before: None,
    changed_after: None,
    changed_before: None,
  };

  let znsr = search_zknotes(conn, user, &nolimsearch)?;
  match znsr {
    Left(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, n.id)?;
      }
      Ok(c)
    }
    Right(znsr) => {
      let c = znsr.notes.len().try_into()?;

      for n in znsr.notes {
        delete_zknote(&conn, file_path.clone(), user, n.id)?;
      }
      Ok(c)
    }
  }
}

pub fn search_zknotes(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<Either<ZkListNoteSearchResult, ZkNoteSearchResult>, Box<dyn Error>> {
  let (sql, args) = build_sql(&conn, user, search.clone())?;

  let mut pstmt = conn.prepare(sql.as_str())?;

  let sysid = user_id(&conn, "system")?;

  let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), |row| {
    let id = row.get(0)?;
    let sysids = get_sysids(conn, sysid, id)?;
    Ok(ZkListNote {
      id: id,
      title: row.get(1)?,
      is_file: {
        let wat: Option<i64> = row.get(2)?;
        match wat {
          Some(_) => true,
          None => false,
        }
      },
      user: row.get(3)?,
      createdate: row.get(4)?,
      changeddate: row.get(5)?,
      sysids: sysids,
    })
  })?;

  if search.list {
    let mut pv = Vec::new();

    for rsrec in rec_iter {
      match rsrec {
        Ok(rec) => {
          pv.push(rec);
        }
        Err(_) => (),
      }
    }

    Ok(Left(ZkListNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  } else {
    let mut pv = Vec::new();

    let s_user = if search.archives { sysid } else { user };

    for rsrec in rec_iter {
      match rsrec {
        Ok(rec) => {
          pv.push(sqldata::read_zknote(&conn, Some(s_user), rec.id)?);
        }
        Err(_) => (),
      }
    }

    Ok(Right(ZkNoteSearchResult {
      notes: pv,
      offset: search.offset,
      what: search.what.clone(),
    }))
  }
}

// pub struct ZkNoteStream<'a, T> {
//   // conn: Connection,
//   // stmt: rusqlite::Statement<'a>,
//   rec_iter: Box<dyn Iterator<Item = T> + 'a>,
//   // rec_iter: Box<MappedRows<'a, dyn FnMut(&Row<'_>) -> rusqlite::Result<T>>>,
// }

/*
// This one fails because pstmt is not consumed by query_map, only borrowed.
pub struct ZkNoteStream<'a> {
  rec_iter: Box<dyn Iterator<Item = Bytes> + 'a>,
}

impl<'a> ZkNoteStream<'a> {
  pub fn init(conn: Connection, user: i64, search: &ZkNoteSearch) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    let bytes_iter = {
      let mut pstmt = &mut conn.prepare(sql.as_str())?;
      let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), move |row| {
        let id = row.get(0)?;
        // commented for simplicity.
        // let sysids = get_sysids(&conn, sysid, id)?;
        Ok(ZkListNote {
          id: id,
          title: row.get(1)?,
          is_file: {
            let wat: Option<i64> = row.get(2)?;
            wat.is_some()
          },
          user: row.get(3)?,
          createdate: row.get(4)?,
          changeddate: row.get(5)?,
          // sysids: sysids,
          sysids: Vec::new(),
        })
      })?;

      let val_iter = rec_iter
        .filter_map(|x| x.ok())
        .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

      val_iter
        .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
        .map(|x| Bytes::from(x.to_string()))
    };

    Ok(ZkNoteStream {
      rec_iter: Box::new(bytes_iter),
    })
  }
}

impl<'a> Stream for ZkNoteStream<'a> {
  type Item = Bytes;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.next())
  }
}
*/

/*
// The ZnsMaker approach.  It compiles!  But ZnsMaker has to live as long as the stream, which isn't possible if you
// return the stream in HttpResponse::Ok().stream(znssstream)
pub struct ZkNoteStream<'a, T> {
  rec_iter: Box<dyn Iterator<Item = T> + 'a>,
}

impl<'a> Stream for ZkNoteStream<'a, Result<ZkListNote, rusqlite::Error>> {
  type Item = Result<ZkListNote, rusqlite::Error>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.next())
  }
}

pub struct ZnsMaker<'a> {
  pstmt: rusqlite::Statement<'a>,
  sysid: i64,
  args: Vec<String>,
}

impl<'a> ZnsMaker<'a> {
  pub fn init(
    conn: &'a Connection,
    user: i64,
    search: &ZkNoteSearch,
  ) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      args: args,
      sysid: sysid,
      pstmt: conn.prepare(sql.as_str())?,
    })
  }

  pub fn make_stream(
    &'a mut self,
    conn: &'a Connection, // have to pass the connection in here instead of storing in ZnsMaker, for Reasons.
  ) -> Result<ZkNoteStream<'a, Result<ZkListNote, rusqlite::Error>>, rusqlite::Error> {
    let sysid = self.sysid;
    let rec_iter =
      self
        .pstmt
        .query_map(rusqlite::params_from_iter(self.args.iter()), move |row| {
          let id = row.get(0)?;
          let sysids = get_sysids(&conn, sysid, id)?;
          Ok(ZkListNote {
            id: id,
            title: row.get(1)?,
            is_file: {
              let wat: Option<i64> = row.get(2)?;
              wat.is_some()
            },
            user: row.get(3)?,
            createdate: row.get(4)?,
            changeddate: row.get(5)?,
            sysids: sysids,
          })
        })?;

    Ok(ZkNoteStream::<'a, Result<ZkListNote, rusqlite::Error>> {
      rec_iter: Box::new(rec_iter),
    })
  }
}
*/

// OK this one compiles, but needs to return Bytes and make its own Connection obj.
/*
// The ZnsMaker approach.  Can consume self??  Probably not.
pub struct ZkNoteStream<'a> {
  znsmaker: ZnsMaker<'a>,
  rec_iter: Option<Box<dyn Iterator<Item = Result<ZkListNote, rusqlite::Error>> + 'a>>,
}

impl<'a> ZkNoteStream<'a> {
  fn init(znsmaker: ZnsMaker<'a>) -> ZkNoteStream<'a> {
    ZkNoteStream::<'a> {
      znsmaker: znsmaker,
      rec_iter: None,
    }
  }

  fn go_stream(self: &'a mut Self) -> Result<(), rusqlite::Error> {
    let sysid = self.znsmaker.sysid;

    self.rec_iter = Some(Box::new(self.znsmaker.pstmt.query_map(
      rusqlite::params_from_iter(self.znsmaker.args.iter()),
      move |row| {
        let id = row.get(0)?;
        // let sysids = get_sysids(&self.znsmaker.conn, sysid, id)?;
        Ok(ZkListNote {
          id: id,
          title: row.get(1)?,
          is_file: {
            let wat: Option<i64> = row.get(2)?;
            wat.is_some()
          },
          user: row.get(3)?,
          createdate: row.get(4)?,
          changeddate: row.get(5)?,
          // sysids: sysids,
          sysids: Vec::new(),
        })
      },
    )?));

    Ok(())
  }
}

impl<'a> Stream for ZkNoteStream<'a> {
  type Item = Result<ZkListNote, rusqlite::Error>;

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.as_mut().and_then(|x| x.next()))
  }
}

pub struct ZnsMaker<'a> {
  // conn: Connection,
  pstmt: rusqlite::Statement<'a>,
  sysid: i64,
  args: Vec<String>,
}

impl<'a> ZnsMaker<'a> {
  pub fn init(
    conn: &'a Connection, // won't last!
    // conn: Connection,
    user: i64,
    search: &ZkNoteSearch,
  ) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      // conn: conn,
      args: args,
      sysid: sysid,
      pstmt: conn.prepare(sql.as_str())?,
    })
  }
}
*/

/*
// Almost working, but needs to own the conn.
pub struct ZkNoteStream<'a> {
  znsmaker: ZnsMaker<'a>,
  rec_iter: Option<Box<dyn Iterator<Item = Result<Bytes, orgauth::error::Error>> + 'a>>,
}

impl<'a> ZkNoteStream<'a> {
  pub fn init(znsmaker: ZnsMaker<'a>) -> ZkNoteStream<'a> {
    ZkNoteStream::<'a> {
      znsmaker: znsmaker,
      rec_iter: None,
    }
  }

  pub fn go_stream(self: &'a mut Self) -> Result<(), rusqlite::Error> {
    let sysid = self.znsmaker.sysid;

    let i = self.znsmaker.pstmt.query_map(
      rusqlite::params_from_iter(self.znsmaker.args.iter()),
      move |row| {
        let id = row.get(0)?;
        // let sysids = get_sysids(&self.znsmaker.conn, sysid, id)?;
        Ok(ZkListNote {
          id: id,
          title: row.get(1)?,
          is_file: {
            let wat: Option<i64> = row.get(2)?;
            wat.is_some()
          },
          user: row.get(3)?,
          createdate: row.get(4)?,
          changeddate: row.get(5)?,
          // sysids: sysids,
          sysids: Vec::new(),
        })
      },
    )?;

    let val_iter = i
      .filter_map(|x| x.ok())
      .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

    let bytes_iter = val_iter
      .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
      .map(|x| Ok(Bytes::from(x.to_string())));

    self.rec_iter = Some(Box::new(bytes_iter));

    Ok(())
  }
}

impl<'a> Stream for ZkNoteStream<'a> {
  type Item = Result<Bytes, orgauth::error::Error>;
  // E: Into<BoxError> + 'static,

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.as_mut().and_then(|x| x.next()))
  }
}

pub struct ZnsMaker<'a> {
  // conn: Connection,
  pstmt: rusqlite::Statement<'a>,
  sysid: i64,
  args: Vec<String>,
}

impl<'a> ZnsMaker<'a> {
  pub fn init(
    conn: &'a Connection, // won't last!
    // conn: Connection,
    user: i64,
    search: &ZkNoteSearch,
  ) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      // conn: conn,
      args: args,
      sysid: sysid,
      pstmt: conn.prepare(sql.as_str())?,
    })
  }
}
*/

/*
// This looks like it should work, but 'znsmaker doesn't live long enough'.
pub struct ZkNoteStream<'a> {
  znsmaker: ZnsMaker,
  pstmt: rusqlite::Statement<'a>,
  rec_iter: Option<Box<dyn Iterator<Item = Result<Bytes, orgauth::error::Error>> + 'a>>,
}

impl<'a> ZkNoteStream<'a> {
  pub fn init(mut znsmaker: ZnsMaker) -> Result<ZkNoteStream<'a>, orgauth::error::Error> {
    // self.znsmaker.pstmt = Some(self.znsmaker.conn.prepare(self.znsmaker.sql.as_str())?);

    // The problem is creating a new obj with znsmaker info AND pstmt that depends on it,
    // and the compiler knowing that they will have the same lifetime.
    let pstmt = znsmaker.pstmt()?;
    Ok(ZkNoteStream::<'a> {
      znsmaker: znsmaker,
      pstmt: pstmt,
      rec_iter: None,
    })
  }

  pub fn go_stream(self: &'a mut Self) -> Result<(), Box<dyn Error>> {
    let sysid = self.znsmaker.sysid;
    let i = self
      .pstmt
      // .ok_or::<orgauth::error::Error>("wups".into())?
      .query_map(
        rusqlite::params_from_iter(self.znsmaker.args.iter()),
        move |row| {
          let id = row.get(0)?;
          // let sysids = get_sysids(&self.znsmaker.conn, sysid, id)?;
          Ok(ZkListNote {
            id: id,
            title: row.get(1)?,
            is_file: {
              let wat: Option<i64> = row.get(2)?;
              wat.is_some()
            },
            user: row.get(3)?,
            createdate: row.get(4)?,
            changeddate: row.get(5)?,
            // sysids: sysids,
            sysids: Vec::new(),
          })
        },
      )?;

    let val_iter = i
      .filter_map(|x| x.ok())
      .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

    let bytes_iter = val_iter
      .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
      .map(|x| Ok(Bytes::from(x.to_string())));

    self.rec_iter = Some(Box::new(bytes_iter));

    Ok(())
  }
}

impl<'a> Stream for ZkNoteStream<'a> {
  type Item = Result<Bytes, orgauth::error::Error>;
  // E: Into<BoxError> + 'static,

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.as_mut().and_then(|x| x.next()))
  }
}

pub struct ZnsMaker {
  conn: Connection,
  sysid: i64,
  sql: String,
  args: Vec<String>,
  // pstmt: Option<rusqlite::Statement<'a>>,
}

impl ZnsMaker {
  pub fn init(conn: Connection, user: i64, search: &ZkNoteSearch) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      conn: conn,
      sql: sql,
      args: args,
      sysid: sysid,
      // pstmt: None,
    })
  }
  pub fn pstmt<'a>(&'a mut self) -> Result<rusqlite::Statement<'a>, rusqlite::Error> {
    self.conn.prepare(self.sql.as_str())
  }
}
*/

// try pstmt in an Option.  I think this didn't work before but one more time...
// doesn't work because go_stream borrows self to make pstmt, looks like!
/*

error[E0597]: `z` does not live long enough
   --> server-lib/src/interfaces.rs:111:7
    |
110 |       let mut z = ZkNoteStream::init(ZnsMaker::init(conn, uid, &search)?)?;
    |           ----- binding `z` declared here
111 |       z.go_stream()?;
    |       ^------------
    |       |
    |       borrowed value does not live long enough
    |       argument requires that `z` is borrowed for `'static`
...
122 |     }
    |     - `z` dropped here while still borrowed

error[E0505]: cannot move out of `z` because it is borrowed
   --> server-lib/src/interfaces.rs:112:39
    |
110 |       let mut z = ZkNoteStream::init(ZnsMaker::init(conn, uid, &search)?)?;
    |           ----- binding `z` declared here
111 |       z.go_stream()?;
    |       -------------
    |       |
    |       borrow of `z` occurs here
    |       argument requires that `z` is borrowed for `'static`
112 |       Ok(HttpResponse::Ok().streaming(z))
    |                                       ^ move out of `z` occurs here
*/

pub struct ZkNoteStream<'a> {
  znsmaker: ZnsMaker,
  pstmt: Option<rusqlite::Statement<'a>>,
  rec_iter: Option<Box<dyn Iterator<Item = Result<Bytes, orgauth::error::Error>> + 'a>>,
}

impl<'a> ZkNoteStream<'a> {
  pub fn init(mut znsmaker: ZnsMaker) -> Result<ZkNoteStream<'a>, orgauth::error::Error> {
    // self.znsmaker.pstmt = Some(self.znsmaker.conn.prepare(self.znsmaker.sql.as_str())?);

    // The problem is creating a new obj with znsmaker info AND pstmt that depends on it,
    // and the compiler knowing that they will have the same lifetime.
    Ok(ZkNoteStream::<'a> {
      znsmaker: znsmaker,
      pstmt: None,
      rec_iter: None,
    })
  }

  // This results in a borrow of self!?
  pub fn go_stream(self: &'a mut Self) -> Result<(), Box<dyn Error>> {
    // let sysid = self.znsmaker.sysid;
    let args = self.znsmaker.args.clone();
    self.pstmt = Some(self.znsmaker.pstmt()?);
    let i = match &mut self.pstmt {
      Some(ref mut pstmt) => {
        pstmt.query_map(rusqlite::params_from_iter(args.iter()), move |row| {
          let id = row.get(0)?;
          // let sysids = get_sysids(&self.znsmaker.conn, sysid, id)?;
          Ok(ZkListNote {
            id: id,
            title: row.get(1)?,
            is_file: {
              let wat: Option<i64> = row.get(2)?;
              wat.is_some()
            },
            user: row.get(3)?,
            createdate: row.get(4)?,
            changeddate: row.get(5)?,
            // sysids: sysids,
            sysids: Vec::new(),
          })
        })?
      }
      None => return Err("wups".into()),
    };

    let val_iter = i
      .filter_map(|x| x.ok())
      .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

    let bytes_iter = val_iter
      .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
      .map(|x| Ok(Bytes::from(x.to_string())));

    self.rec_iter = Some(Box::new(bytes_iter));

    Ok(())
  }
}

impl<'a> Stream for ZkNoteStream<'a> {
  type Item = Result<Bytes, orgauth::error::Error>;
  // E: Into<BoxError> + 'static,

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    Poll::Ready(self.rec_iter.as_mut().and_then(|x| x.next()))
  }
}

pub struct ZnsMaker {
  conn: Connection,
  sysid: i64,
  sql: String,
  args: Vec<String>,
  // pstmt: Option<rusqlite::Statement<'a>>,
}

impl ZnsMaker {
  pub fn init(conn: Connection, user: i64, search: &ZkNoteSearch) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      conn: conn,
      sql: sql,
      args: args,
      sysid: sysid,
      // pstmt: None,
    })
  }
  pub fn pstmt<'a>(&'a mut self) -> Result<rusqlite::Statement<'a>, rusqlite::Error> {
    self.conn.prepare(self.sql.as_str())
  }
}

// pub fn make_stream(
//   &'a mut self,
//   // mut self: Self,
//   conn: &'a Connection, // have to pass the connection in here instead of storing in ZnsMaker, for Reasons.
// ) -> Result<ZkNoteStream<'a, Result<ZkListNote, rusqlite::Error>>, rusqlite::Error> {
//   let sysid = self.sysid;
//   let rec_iter =
//     self
//       .pstmt
//       .query_map(rusqlite::params_from_iter(self.args.iter()), move |row| {
//         let id = row.get(0)?;
//         let sysids = get_sysids(&conn, sysid, id)?;
//         Ok(ZkListNote {
//           id: id,
//           title: row.get(1)?,
//           is_file: {
//             let wat: Option<i64> = row.get(2)?;
//             wat.is_some()
//           },
//           user: row.get(3)?,
//           createdate: row.get(4)?,
//           changeddate: row.get(5)?,
//           sysids: sysids,
//         })
//       })?;

//   Ok(ZkNoteStream::<'a, Result<ZkListNote, rusqlite::Error>> {
//     znsmaker: *self,
//     rec_iter: Box::new(rec_iter),
//   })
// }

/*
pub struct ZnsMaker<'a> {
  conn: rusqlite::Connection,
  pstmt: rusqlite::Statement<'a>,
  sysid: i64,
  args: Vec<String>,
}

impl<'a> IntoIterator for ZnsMaker<'a> {
  type Item = Result<Bytes, orgauth::error::Error>;
  type IntoIter = Box<Iterator<Item = Result<Bytes, orgauth::error::Error>>>;

  fn into_iter(mut self) -> Box<dyn Iterator<Item = Result<Bytes, orgauth::error::Error>> + 'a> {
    // pub fn make_stream(
    //   &'a mut self,
    // ) -> Result<ZkNoteStream<'a, Result<Bytes, orgauth::error::Error>>, rusqlite::Error> {
    let sysid = self.sysid;
    let rec_iter =
      match self
        .pstmt
        .query_map(rusqlite::params_from_iter(self.args.iter()), move |row| {
          let id = row.get(0)?;
          let sysids = get_sysids(&self.conn, sysid, id)?;
          Ok(ZkListNote {
            id: id,
            title: row.get(1)?,
            is_file: {
              let wat: Option<i64> = row.get(2)?;
              wat.is_some()
            },
            user: row.get(3)?,
            createdate: row.get(4)?,
            changeddate: row.get(5)?,
            sysids: sysids,
          })
        }) {
        Ok(x) => x,
        Err(_) => return Box::new(std::iter::empty()),
      };

    let val_iter = rec_iter
      .filter_map(|x| x.ok())
      .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

    let bytes_iter = val_iter
      .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
      // .filter_map(|x| x.to_string())
      .map(|x| Ok(Bytes::from(x.to_string())));

    Box::new(bytes_iter)

    // Ok(ZkNoteStream::<'a, Result<Bytes, orgauth::error::Error>> {
    //   rec_iter: Box::new(bytes_iter),
    // })
  }
}

impl<'a> ZnsMaker<'a> {
  pub fn init(conn: Connection, user: i64, search: &ZkNoteSearch) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      conn: conn,
      args: args,
      sysid: sysid,
      pstmt: conn.prepare(sql.as_str())?,
    })
  }
*/

// pub fn make_stream(
//   &'a mut self,
// ) -> Result<ZkNoteStream<'a, Result<Bytes, orgauth::error::Error>>, rusqlite::Error> {
//   let sysid = self.sysid;
//   let rec_iter =
//     self
//       .pstmt
//       .query_map(rusqlite::params_from_iter(self.args.iter()), move |row| {
//         let id = row.get(0)?;
//         let sysids = get_sysids(&conn, sysid, id)?;
//         Ok(ZkListNote {
//           id: id,
//           title: row.get(1)?,
//           is_file: {
//             let wat: Option<i64> = row.get(2)?;
//             wat.is_some()
//           },
//           user: row.get(3)?,
//           createdate: row.get(4)?,
//           changeddate: row.get(5)?,
//           sysids: sysids,
//         })
//       })?;

//   let val_iter = rec_iter
//     .filter_map(|x| x.ok())
//     .map(|x| serde_json::to_value(x).map_err(|e| e.into()));

//   let bytes_iter = val_iter
//     .filter_map(|x: Result<serde_json::Value, orgauth::error::Error>| x.ok())
//     .filter_map(|x| x.as_str())
//     .map(|x| Ok(Bytes::from(x)));

//   Ok(ZkNoteStream::<'a, Result<Bytes, orgauth::error::Error>> {
//     rec_iter: Box::new(bytes_iter),
//   })
// }

/*
  impl<'a> ZnsMaker<'a> {
  pub fn init(
    conn: &'a Connection,
    user: i64,
    search: &ZkNoteSearch,
  ) -> Result<Self, Box<dyn Error>> {
    let (sql, args) = build_sql(&conn, user, search.clone())?;

    let sysid = user_id(&conn, "system")?;

    Ok(ZnsMaker {
      args: args,
      sysid: sysid,
      pstmt: conn.prepare(sql.as_str())?,
    })
  }

  pub fn make_stream(
    &'a mut self,
    conn: &'a Connection,
  ) -> Result<ZkNoteStream<'a, Result<ZkListNote, rusqlite::Error>>, rusqlite::Error> {
    let sysid = self.sysid;
    let rec_iter =
      self
        .pstmt
        .query_map(rusqlite::params_from_iter(self.args.iter()), move |row| {
          let id = row.get(0)?;
          let sysids = get_sysids(&conn, sysid, id)?;
          Ok(ZkListNote {
            id: id,
            title: row.get(1)?,
            is_file: {
              let wat: Option<i64> = row.get(2)?;
              wat.is_some()
            },
            user: row.get(3)?,
            createdate: row.get(4)?,
            changeddate: row.get(5)?,
            sysids: sysids,
          })
        })?;

    Ok(ZkNoteStream::<'a, Result<ZkListNote, rusqlite::Error>> {
      rec_iter: Box::new(rec_iter),
    })
  }
}
*/

// impl<'a> ZkNoteStream<'a, Result<ZkListNote, rusqlite::Error>> {
//   // pub fn init(
//   //   conn: &'a Connection,
//   //   user: i64,
//   //   search: &ZkNoteSearch,
//   // ) -> Result<Self, Box<dyn Error>> {
//   //   let (sql, args) = build_sql(&conn, user, search.clone())?;

//   //   let sysid = user_id(&conn, "system")?;
//   //   let mut pstmt = conn.prepare(sql.as_str())?;

//   //   let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), move |row| {
//   //     let id = row.get(0)?;
//   //     let sysids = get_sysids(&conn, sysid, id)?;
//   //     Ok(ZkListNote {
//   //       id: id,
//   //       title: row.get(1)?,
//   //       is_file: {
//   //         let wat: Option<i64> = row.get(2)?;
//   //         wat.is_some()
//   //       },
//   //       user: row.get(3)?,
//   //       createdate: row.get(4)?,
//   //       changeddate: row.get(5)?,
//   //       sysids: sysids,
//   //     })
//   //   })?;

//   //   Ok(ZkNoteStream::<Result<ZkListNote, rusqlite::Error>> {
//   //     rec_iter: Box::new(rec_iter),
//   //   })
//   // }
//   // pub fn init(
//   //   conn: &'a Connection,
//   //   user: i64,
//   //   search: &ZkNoteSearch,
//   // ) -> Result<Self, Box<dyn Error>> {
//   //   let (sql, args) = build_sql(&conn, user, search.clone())?;

//   //   let sysid = user_id(&conn, "system")?;
//   //   let mut pstmt = conn.prepare(sql.as_str())?;

//   //   let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), move |row| {
//   //     let id = row.get(0)?;
//   //     let sysids = get_sysids(&conn, sysid, id)?;
//   //     Ok(ZkListNote {
//   //       id: id,
//   //       title: row.get(1)?,
//   //       is_file: {
//   //         let wat: Option<i64> = row.get(2)?;
//   //         wat.is_some()
//   //       },
//   //       user: row.get(3)?,
//   //       createdate: row.get(4)?,
//   //       changeddate: row.get(5)?,
//   //       sysids: sysids,
//   //     })
//   //   })?;

//   //   Ok(ZkNoteStream::<Result<ZkListNote, rusqlite::Error>> {
//   //     rec_iter: Box::new(rec_iter),
//   //   })
//   // }
// }

pub fn search_zknotes_stream(
  conn: &Connection,
  user: i64,
  search: &ZkNoteSearch,
) -> Result<Either<ZkListNoteSearchResult, ZkNoteSearchResult>, Box<dyn Error>> {
  Err("wat".into())

  // let (sql, args) = build_sql(&conn, user, search.clone())?;

  // let mut pstmt = conn.prepare(sql.as_str())?;

  // let sysid = user_id(&conn, "system")?;

  // let rec_iter = pstmt.query_map(rusqlite::params_from_iter(args.iter()), |row| {
  //   let id = row.get(0)?;
  //   let sysids = get_sysids(conn, sysid, id)?;
  //   Ok(ZkListNote {
  //     id: id,
  //     title: row.get(1)?,
  //     is_file: {
  //       let wat: Option<i64> = row.get(2)?;
  //       match wat {
  //         Some(_) => true,
  //         None => false,
  //       }
  //     },
  //     user: row.get(3)?,
  //     createdate: row.get(4)?,
  //     changeddate: row.get(5)?,
  //     sysids: sysids,
  //   })
  // })?;
}

pub fn build_sql(
  conn: &Connection,
  uid: i64,
  search: ZkNoteSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (mut dtcls, mut dtclsargs) = build_daterange_clause(&search)?;

  println!("dtcls: {:?}", dtcls);

  let (cls, clsargs) = build_tagsearch_clause(&conn, uid, false, &search.tagsearch)?;

  let publicid = note_id(&conn, "system", "public")?;
  let archiveid = note_id(&conn, "system", "archive")?;
  let shareid = note_id(&conn, "system", "share")?;
  let usernoteid = sqldata::user_note_id(&conn, uid)?;

  let limclause = match search.limit {
    Some(lm) => format!(" limit {} offset {}", lm, search.offset),
    None => "".to_string(), // no limit, no offset either
                            // None => format!(" offset {}", search.offset),
  };

  let ordclause = " order by N.changeddate desc ";

  let archives = search.archives;

  let (mut sqlbase, mut baseargs) = if archives {
    (
      // archives of notes that are mine.
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zknote O, zklink OL, zklink AL where
      (O.user = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
      and O.deleted = 0"
      ),
      vec![uid.to_string(), archiveid.to_string()],
    )
  } else {
    // notes that are mine.
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N where N.user = ?
      and N.deleted = 0"
      ),
      vec![uid.to_string()],
    )
  };

  // notes that are public, and not mine.
  let (mut sqlpub, mut pubargs) = if archives {
    (
      // archives of notes that are public, and not mine.
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where (N.user != ?
        and L.fromid = N.id and L.toid = ?
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?)
      and N.deleted = 0"
      ),
      vec![uid.to_string(), publicid.to_string(), archiveid.to_string()],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (N.user != ? and L.fromid = N.id and L.toid = ?)
      and N.deleted = 0"
      ),
      vec![uid.to_string(), publicid.to_string()],
    )
  };

  // notes shared with a share tag, and not mine.
  // clause 1: user is not-me
  //
  // clause 2: is N linked to a share note?
  // link M is to shareid, and L links either to or from M's from.
  //
  // clause 3 is M.from (the share)
  // is that share linked to usernoteid?
  let (mut sqlshare, mut shareargs) = if archives {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U, zknote O, zklink OL, zklink AL
      where (N.user != ? and
        (M.toid = ? and (
          (L.fromid = N.id and L.toid = M.fromid ) or
          (L.toid = N.id and L.fromid = M.fromid ))
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?))
      and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        archiveid.to_string(),
        archiveid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zklink M, zklink U
      where (N.user != ? 
        and M.toid = ? 
        and ((L.fromid = N.id and L.toid = M.fromid )
             or (L.toid = N.id and L.fromid = M.fromid ))
      and
        L.linkzknote is not ?
      and
        ((U.fromid = ? and U.toid = M.fromid) or (U.fromid = M.fromid and U.toid = ?)))
      and N.deleted = 0",
      ),
      vec![
        uid.to_string(),
        shareid.to_string(),
        archiveid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  };

  // notes that are tagged with my usernoteid, and not mine.
  let (mut sqluser, mut userargs) = if archives {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L, zknote O, zklink OL, zklink AL
      where N.user != ?
        and ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?))
        and OL.fromid = N.id and OL.toid = O.id
        and AL.fromid = N.id and AL.toid = ?
        and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
        archiveid.to_string(),
      ],
    )
  } else {
    (
      format!(
        "select N.id, N.title, N.file, N.user, N.createdate, N.changeddate
      from zknote N, zklink L
      where (
        N.user != ? and
        ((L.fromid = N.id and L.toid = ?) or (L.toid = N.id and L.fromid = ?)))
        and N.deleted = 0"
      ),
      vec![
        uid.to_string(),
        usernoteid.to_string(),
        usernoteid.to_string(),
      ],
    )
  };
  // local ftn to add clause and args.
  let addcls = |sql: &mut String, args: &mut Vec<String>| {
    if !args.is_empty() {
      sql.push_str(" and ");
      sql.push_str(cls.as_str());

      // clone, otherwise no clause vals next time!
      let mut pendargs = clsargs.clone();
      args.append(&mut pendargs);
    }
  };

  addcls(&mut sqlbase, &mut baseargs);
  addcls(&mut sqlpub, &mut pubargs);
  addcls(&mut sqluser, &mut userargs);
  addcls(&mut sqlshare, &mut shareargs);
  addcls(&mut dtcls, &mut dtclsargs);

  // combine the queries.
  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlpub.as_str());
  baseargs.append(&mut pubargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqlshare.as_str());
  baseargs.append(&mut shareargs);

  sqlbase.push_str(" union ");
  sqlbase.push_str(sqluser.as_str());
  baseargs.append(&mut userargs);

  // add order clause to the end.
  sqlbase.push_str(ordclause);

  // add limit clause to the end.
  sqlbase.push_str(limclause.as_str());

  Ok((sqlbase, baseargs))
}

fn build_daterange_clause(search: &ZkNoteSearch) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let clawses = [
    search
      .created_after
      .map(|dt| ("N.createddate < ?", dt.to_string())),
    search
      .created_before
      .map(|dt| ("N.createddate < ?", dt.to_string())),
    search
      .changed_after
      .map(|dt| ("N.changeddate < ?", dt.to_string())),
    search
      .changed_before
      .map(|dt| ("N.changeddate < ?", dt.to_string())),
  ];
  let clause = clawses
    .iter()
    .filter_map(|pair| pair.as_ref().map(|(s, _)| s.to_string()))
    .collect::<Vec<String>>()
    .join(" and ");
  let args = clawses
    .iter()
    .filter_map(|pair| pair.as_ref().map(|(_, dt)| dt.clone()))
    .collect();
  Ok((clause, args))
}

fn build_tagsearch_clause(
  conn: &Connection,
  uid: i64,
  not: bool,
  search: &TagSearch,
) -> Result<(String, Vec<String>), Box<dyn Error>> {
  let (cls, args) = match search {
    TagSearch::SearchTerm { mods, term } => {
      let mut exact = false;
      let mut tag = false;
      let mut desc = false;
      let mut user = false;
      let mut file = false;

      for m in mods {
        match m {
          SearchMod::ExactMatch => exact = true,
          SearchMod::Tag => tag = true,
          SearchMod::Note => desc = true,
          SearchMod::User => user = true,
          SearchMod::File => file = true,
        }
      }
      let field = if desc { "content" } else { "title" };

      if user {
        let user = user_id(conn, &term)?;
        let notstr = match not {
          true => "!",
          false => "",
        };
        (format!("N.user {}= ?", notstr), vec![format!("{}", user)])
      } else {
        if tag {
          let fileclause = if file { "and zkn.file is not null" } else { "" };

          let clause = if exact {
            format!("zkn.{} = ? {}", field, fileclause)
          } else {
            format!("zkn.{}  like ? {}", field, fileclause)
          };

          let notstr = if not { "not" } else { "" };

          (
            // clause
            format!(
              "{} (N.id in (select zklink.toid from zknote as zkn, zklink
             where zkn.id = zklink.fromid
               and {})
            or
                N.id in (select zklink.fromid from zknote as zkn, zklink
             where zkn.id = zklink.toid
               and {}))",
              notstr, clause, clause
            ),
            // args
            if exact {
              vec![term.clone(), term.clone()]
            } else {
              vec![
                format!("%{}%", term).to_string(),
                format!("%{}%", term).to_string(),
              ]
            },
          )
        } else {
          let fileclause = if file { "and N.file is not null" } else { "" };

          let notstr = match (not, exact) {
            (true, false) => "not",
            (false, false) => "",
            (true, true) => "!",
            (false, true) => "",
          };

          (
            // clause
            if exact {
              format!("N.{} {}= ? {}", field, notstr, fileclause)
            } else {
              format!("N.{} {} like ? {}", field, notstr, fileclause)
            },
            // args
            if exact {
              vec![term.clone()]
            } else {
              vec![format!("%{}%", term).to_string()]
            },
          )
        }
      }
    }
    TagSearch::Not { ts } => build_tagsearch_clause(&conn, uid, true, &*ts)?,
    TagSearch::Boolex { ts1, ao, ts2 } => {
      let (cl1, mut arg1) = build_tagsearch_clause(&conn, uid, false, &*ts1)?;
      let (cl2, mut arg2) = build_tagsearch_clause(&conn, uid, false, &*ts2)?;
      let mut cls = String::new();
      let conj = match ao {
        AndOr::Or => " or ",
        AndOr::And => " and ",
      };
      cls.push_str("(");
      cls.push_str(cl1.as_str());
      cls.push_str(conj);
      cls.push_str(cl2.as_str());
      cls.push_str(")");
      arg1.append(&mut arg2);
      (cls, arg1)
    }
  };
  Ok((cls, args))
}
