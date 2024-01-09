use crate::appdata::AppData;
use crate::appdata::Config;
use crate::appdata::TokenInfo;
use crate::error as zkerr;
use crate::search;
use crate::sqldata;
use crate::sync;
use actix::{ActorContext, StreamHandler};
use actix_session::Session;
use actix_web::HttpResponse;
use actix_web_actors::ws;
use async_stream::try_stream;
use bytes::Bytes;
use bytestring::ByteString;
use futures::{Stream, TryStream};
use futures_util::TryStreamExt;
use log::info;
use orgauth;
use orgauth::endpoints::{Callbacks, Tokener};
use rusqlite::Connection;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use zkprotocol::constants::PrivateReplies;
use zkprotocol::constants::PublicReplies;
use zkprotocol::constants::{PrivateRequests, PrivateStreamingRequests, PublicRequests};
use zkprotocol::content::{
  GetArchiveZkLinks, GetArchiveZkNote, GetZkLinksSince, GetZkNoteAndLinks, GetZkNoteArchives,
  GetZkNoteComments, GetZnlIfChanged, ImportZkNote, SaveZkNote, SaveZkNoteAndLinks, ZkLinks,
  ZkListNote, ZkNoteAndLinks, ZkNoteAndLinksWhat, ZkNoteArchives, ZkNoteId,
};
use zkprotocol::messages::PublicReplyMessage;
use zkprotocol::messages::{
  PrivateMessage, PrivateReplyMessage, PrivateStreamingMessage, PublicMessage,
};
use zkprotocol::search::{
  AndOr, OrderDirection, OrderField, ResultType, SearchMod, TagSearch, ZkIdSearchResult,
  ZkListNoteSearchResult, ZkNoteAndLinksSearchResult, ZkNoteSearch, ZkNoteSearchResult,
  ZkSearchResultHeader,
};
// use zkprotocol::search::{TagSearch, ZkListNoteSearchResult, ZkNoteSearch};

// TODO: have the socket close on timeout as in example?
pub struct StreamingWebSocket {
  pub config: Config,
  pub uid: i64,
}

impl actix::Actor for StreamingWebSocket {
  type Context = ws::WebsocketContext<Self>;

  // /// Method is called on actor start. We start the heartbeat process here.
  // fn started(&mut self, ctx: &mut Self::Context) {
  //   self.hb(ctx);
  // }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for StreamingWebSocket {
  fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
    // process websocket messages
    println!("WS: {msg:?}");
    match msg {
      Ok(ws::Message::Ping(msg)) => {
        // self.hb = Instant::now();
        ctx.pong(&msg);
      }
      Ok(ws::Message::Pong(_)) => {
        // self.hb = Instant::now();
      }
      Ok(ws::Message::Text(text)) => match std::str::from_utf8(text.as_bytes()) {
        Ok(utf8) => match serde_json::from_str(utf8) {
          Ok(psm) => match zk_interface_loggedin_wsstreaming(&self.config, self.uid, &psm, ctx) {
            Ok(_) => {}
            Err(e) => {}
          },
          Err(_) => {}
        },
        Err(_) => {}
      },
      Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
      Ok(ws::Message::Close(reason)) => {
        ctx.close(reason);
        ctx.stop();
      }
      _ => ctx.stop(),
    }
  }
}

pub fn zk_interface_loggedin_wsstreaming(
  config: &Config,
  uid: i64,
  msg: &PrivateStreamingMessage,
  ctx: &mut ws::WebsocketContext<StreamingWebSocket>,
) -> Result<(), zkerr::Error> {
  // Ok(HttpResponse::Ok().into())
  match msg.what {
    PrivateStreamingRequests::SearchZkNotes => {
      let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
      let search: ZkNoteSearch = serde_json::from_value(msgdata.clone())?;
      let conn = Arc::new(sqldata::connection_open(
        config.orgauth_config.db.as_path(),
      )?);

      // search_zknotes_wsstream_sync(conn, uid, search, ctx)?;
      let znsstream = search_zknotes_wsstream_sync(conn, uid, search, ctx);
      // let q = znsstream.map_err(|x| ws::ProtocolError::Io(std::io::Error::other(x)));

      StreamingWebSocket::add_stream(
        // znsstream,
        // znsstream.map_err(|x| ws::ProtocolError::Io(std::io::Error::other(x))),
        znsstream.map_err(|x| {
          // info!("errah {:?}", x);
          ws::ProtocolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "wups"))
        }),
        ctx,
      );
      Ok(())
    }
    _ => Ok(()),
  }
  //   PrivateStreamingRequests::GetArchiveZkLinks => {
  //     let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
  //     let rq: GetArchiveZkLinks = serde_json::from_value(msgdata.clone())?;
  //     let conn = Arc::new(sqldata::connection_open(
  //       config.orgauth_config.db.as_path(),
  //     )?);
  //     let bstream = sqldata::read_archivezklinks_stream(conn, uid, rq.createddate_after);
  //     Ok(HttpResponse::Ok().streaming(bstream))
  //   }
  //   PrivateStreamingRequests::GetZkLinksSince => {
  //     let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
  //     let rq: GetZkLinksSince = serde_json::from_value(msgdata.clone())?;
  //     let conn = Arc::new(sqldata::connection_open(
  //       config.orgauth_config.db.as_path(),
  //     )?);
  //     let bstream = sqldata::read_zklinks_since_stream(conn, uid, rq.createddate_after);
  //     Ok(HttpResponse::Ok().streaming(bstream))
  //   } // wat => Err(format!("invalid 'what' code:'{}'", wat).into()),
  // }
}

pub fn search_zknotes_wsstream_sync(
  conn: Arc<Connection>,
  user: i64,
  search: ZkNoteSearch,
  ctx: &mut ws::WebsocketContext<StreamingWebSocket>,
) -> impl TryStream<Item = Result<ws::Message, zkerr::Error>, Ok = actix_web_actors::ws::Message> + 'static
{
  // ) -> Result<(), zkerr::Error> {
  // uncomment for formatting, lsp
  // {
  try_stream! {
    // let sysid = user_id(&conn, "system")?;
    let s_user = if search.archives {
      orgauth::dbfun::user_id(&conn, "system")?
    } else {
      user
    };
    let (sql, args) = search::build_sql(&conn, user, search.clone())?;
    let mut stmt = conn.prepare(sql.as_str())?;

    let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;

    let mut header = serde_json::to_value(PrivateReplyMessage {
      what: match search.resulttype {
        ResultType::RtId => PrivateReplies::ZkNoteIdSearchResult,
        ResultType::RtListNote => PrivateReplies::ZkListNoteSearchResult,
        ResultType::RtNote => PrivateReplies::ZkNoteSearchResult,
        ResultType::RtNoteAndLinks => PrivateReplies::ZkNoteAndLinksSearchResult,
      },
      content: serde_json::to_value(ZkSearchResultHeader {
        what: search.what,
        offset: search.offset,
      })?,
    })?
    .to_string();

    // ctx.text(header);
    yield ws::Message::Text(ByteString::from(header));

    // header.push_str("\n");
    // yield Bytes::from(ByteString::from(header);

    while let Some(row) = rows.next()? {
      println!("got row: {:?}", row);
      match search.resulttype {
        ResultType::RtId => {
          let mut s = serde_json::to_value(row.get::<usize, String>(1)?.as_str())?
            .to_string()
            .to_string();
          //       ctx.text(s);
          yield ws::Message::Text(ByteString::from(s));
          // s.push_str("\n");
          //  Bytes::from(s);
        }
        ResultType::RtListNote => {
          let zln = ZkListNote {
            id: Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?,
            title: row.get(2)?,
            is_file: {
              let wat: Option<i64> = row.get(3)?;
              wat.is_some()
            },
            user: row.get(4)?,
            createdate: row.get(5)?,
            changeddate: row.get(6)?,
            sysids: Vec::new(),
          };

          let mut s = serde_json::to_value(zln)?.to_string().to_string();
          //       ctx.text(s);
          yield ws::Message::Text(ByteString::from(s));
          // s.push_str("\n");
          // yield Bytes::from(ByteString::from(s);
        }
        ResultType::RtNote => {
          let zn = sqldata::read_zknote_i64(&conn, Some(s_user), row.get(0)?)?;
          let mut s = serde_json::to_value(zn)?.to_string().to_string();
          //       ctx.text(s);
          yield ws::Message::Text(ByteString::from(s));
          // s.push_str("\n");
          // yield Bytes::from(ByteString::from(s);
        }
        ResultType::RtNoteAndLinks => {
          // TODO: i64 version
          let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
          let zn = sqldata::read_zknoteandlinks(&conn, Some(s_user), &uuid)?;
          let mut s = serde_json::to_value(zn)?.to_string().to_string();
          //       ctx.text(s);
          yield ws::Message::Text(ByteString::from(s));
          // s.push_str("\n");
          // yield Bytes::from(ByteString::from(s);
        }
      }
    }
  }
}

pub fn search_zknotes_wsstream(
  conn: Arc<Connection>,
  user: i64,
  search: ZkNoteSearch,
  ctx: &mut ws::WebsocketContext<StreamingWebSocket>,
  // ) -> impl Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> + 'static {
) -> Result<(), zkerr::Error> {
  // uncomment for formatting, lsp
  // {
  // try_stream! {
  // let sysid = user_id(&conn, "system")?;
  let s_user = if search.archives {
    orgauth::dbfun::user_id(&conn, "system")?
  } else {
    user
  };
  let (sql, args) = search::build_sql(&conn, user, search.clone())?;
  let mut stmt = conn.prepare(sql.as_str())?;

  let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;

  let mut header = serde_json::to_value(PrivateReplyMessage {
    what: match search.resulttype {
      ResultType::RtId => PrivateReplies::ZkNoteIdSearchResult,
      ResultType::RtListNote => PrivateReplies::ZkListNoteSearchResult,
      ResultType::RtNote => PrivateReplies::ZkNoteSearchResult,
      ResultType::RtNoteAndLinks => PrivateReplies::ZkNoteAndLinksSearchResult,
    },
    content: serde_json::to_value(ZkSearchResultHeader {
      what: search.what,
      offset: search.offset,
    })?,
  })?
  .to_string();

  ctx.text(header);

  // header.push_str("\n");
  // yield Bytes::from(header);

  while let Some(row) = rows.next()? {
    println!("got row: {:?}", row);
    match search.resulttype {
      ResultType::RtId => {
        let mut s = serde_json::to_value(row.get::<usize, String>(1)?.as_str())?
          .to_string()
          .to_string();
        ctx.text(s);
        // s.push_str("\n");
        //  Bytes::from(s);
      }
      ResultType::RtListNote => {
        let zln = ZkListNote {
          id: Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?,
          title: row.get(2)?,
          is_file: {
            let wat: Option<i64> = row.get(3)?;
            wat.is_some()
          },
          user: row.get(4)?,
          createdate: row.get(5)?,
          changeddate: row.get(6)?,
          sysids: Vec::new(),
        };

        let mut s = serde_json::to_value(zln)?.to_string().to_string();
        ctx.text(s);
        // s.push_str("\n");
        // yield Bytes::from(s);
      }
      ResultType::RtNote => {
        let zn = sqldata::read_zknote_i64(&conn, Some(s_user), row.get(0)?)?;
        let mut s = serde_json::to_value(zn)?.to_string().to_string();
        ctx.text(s);
        // s.push_str("\n");
        // yield Bytes::from(s);
      }
      ResultType::RtNoteAndLinks => {
        // TODO: i64 version
        let uuid = Uuid::parse_str(row.get::<usize, String>(1)?.as_str())?;
        let zn = sqldata::read_zknoteandlinks(&conn, Some(s_user), &uuid)?;
        let mut s = serde_json::to_value(zn)?.to_string().to_string();
        ctx.text(s);
        // s.push_str("\n");
        // yield Bytes::from(s);
      }
    }
  }

  Ok(())
  // }
}
