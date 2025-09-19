use clap::{self, Arg};
use serde_json;
use std::io::Read;
use std::{fmt, io};
use zkprotocol::content::SaveZkNote;
use zkprotocol::search::{self as zs, ZkNoteSearch};
use zkprotocol::search_util::tag_search_parser;

pub enum Error {
  String(String),
}

impl std::error::Error for Error {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self {
      Error::String(e) => write!(f, "{}", e),
    }
  }
}

impl fmt::Debug for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &self {
      Error::String(e) => write!(f, "{}", e),
    }
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let matches = clap::Command::new("zknotes cli")
    .version("1.0")
    .author("Ben Burdette")
    .about("zettelkasten web server")
    .arg(
      Arg::new("command")
        .short('c')
        .long("command")
        .value_name("command to execute")
        .help("search, savenote, login"),
    )
    .arg(
      Arg::new("server_url")
        .short('r')
        .long("url")
        .value_name("zknotes server url")
        .help("server to connect to"),
    )
    .arg(
      Arg::new("user")
        .short('u')
        .long("user")
        .value_name("user name")
        .help("name of user to search with"),
    )
    .arg(
      Arg::new("password")
        .short('p')
        .long("password")
        .value_name("password")
        .help("password"),
    )
    .arg(
      Arg::new("cookie")
        .short('k')
        .long("cookie")
        .value_name("cookie"),
    )
    .arg(
      Arg::new("search")
        .short('s')
        .long("search")
        .value_name("search expression"),
    )
    .arg(
      Arg::new("search_result_format")
        .short('f')
        .long("search_format")
        .value_name("search result format")
        .help("RtId, RtListNote, RtNote, RtNoteAndLinks"),
    )
    .get_matches();

  let url = matches
    .get_one::<String>("server_url")
    .ok_or(Error::String("'server_url' is required!".to_string()))?;

  match matches
    .get_one::<String>("command")
    .ok_or(Error::String("'command' is required!".to_string()))?
    .as_str()
  {
    "login" => {
      let client = reqwest::blocking::Client::builder().build()?;

      let username = matches
        .get_one::<String>("user")
        .ok_or(Error::String("'user' is required!".to_string()))?;
      let password = matches
        .get_one::<String>("password")
        .ok_or(Error::String("'password' is required!".to_string()))?;

      let rq = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(serde_json::to_string(
          &orgauth::data::UserRequest::UrqLogin(orgauth::data::Login {
            uid: username.clone(),
            pwd: password.clone(),
          }),
        )?);

      let res = rq.send();

      match res {
        Ok(r) => {
          if r.status().is_success() {
            if let Some(cookie) = r.headers().get("set-cookie") {
              // let x: &reqwest::header::HeaderValue = cookie;
              println!("cookie: {}", cookie.to_str()?);
            } else {
              println!("cookie not found!");
            }
          } else {
            println!("bad response: {:?}", r);
          }
        }
        Err(e) => {
          println!("result: {:?}", e);
        }
      };

      return Ok(());
    }
    "search" => {
      match (
        matches.get_one::<String>("cookie"),
        matches.get_one::<String>("search"),
      ) {
        (Some(cookie), Some(search)) => {
          let result_type = match matches.get_one::<String>("search_result_format") {
            Some(s) => {
              let quoted = format!("\"{}\"", s);
              serde_json::from_str::<zs::ResultType>(quoted.as_str())?
            }
            None => zs::ResultType::RtListNote,
          };

          let (_extra, tag_search) = match tag_search_parser(search) {
            Ok(ts) => ts,
            Err(e) => return Err(Box::new(Error::String(e.to_string()))),
          };

          let zns = ZkNoteSearch {
            tagsearch: vec![tag_search],
            offset: 0,
            limit: None,
            what: "".to_string(),
            resulttype: result_type,
            archives: zs::ArchivesOrCurrent::Current,
            deleted: false, // include deleted notes
            ordering: None,
          };

          let client = reqwest::blocking::Client::builder().build()?;
          let rq = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::COOKIE, cookie)
            .body(serde_json::to_string(
              &zkprotocol::private::PrivateRequest::PvqSearchZkNotes(zns),
            )?);

          let mut res = rq.send()?;

          let mut buf = String::new();
          res.read_to_string(&mut buf)?;

          println!("{}", buf);

          ()
        }
        _ => {
          println!("cookie, user and search parameters are required");
        }
      };
    }
    "savenote" => {
      match matches.get_one::<String>("cookie") {
        Some(cookie) => {
          let stdin = io::stdin();

          // turns out this can read a serialized zknote!
          let sn: SaveZkNote = serde_json::from_reader(stdin)?;

          let client = reqwest::blocking::Client::builder().build()?;
          let rq = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::COOKIE, cookie)
            .body(serde_json::to_string(
              &zkprotocol::private::PrivateRequest::PvqSaveZkNote(sn),
            )?);

          let mut res = rq.send()?;

          let mut buf = String::new();
          res.read_to_string(&mut buf)?;

          println!("{}", buf);

          ()
        }
        _ => {
          println!("cookie, user and search parameters are required");
        }
      };
    }
    &_ => {
      println!("unsupported command");
    }
  }

  Ok(())
}
