use clap::{self, Arg};
use reqwest::blocking as rb;
use serde_json;
use std::fmt;
use zkprotocol::search as zs;
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
        .body(serde_json::to_string(&orgauth::data::Login {
          uid: username.clone(),
          pwd: password.clone(),
        })?);

      println!("result: {:?}", rq.send());

      // client.(f)
      // zkprotocol::content::
    }
    "search" => {
      println!("searcH");
    }
    &_ => {
      println!("unsupported command");
    }
  }

  match (
    matches.get_one::<String>("user"),
    matches.get_one::<String>("search"),
  ) {
    (Some(username), Some(search)) => {
      // let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;

      let result_type = match matches.get_one::<String>("search_result_format") {
        Some(s) => {
          let quoted = format!("\"{}\"", s);
          serde_json::from_str::<zs::ResultType>(quoted.as_str())?
        }
        None => zs::ResultType::RtListNote,
      };

      let tag_search = match tag_search_parser(search) {
        Ok(ts) => ts,
        Err(e) => return Err(Box::new(Error::String(e.to_string()))),
      };

      // reqwest::Request::

      println!("search parse: {:?}", tag_search);

      // match (user_id(&conn, username), tag_search_parser(search)) {
      //   (Ok(uid), Ok((_s, tagsearch))) => {
      //     let zns = ZkNoteSearch {
      //       tagsearch: vec![tagsearch],
      //       offset: 0,
      //       limit: None,
      //       what: "".to_string(),
      //       resulttype: result_type,
      //       archives: zs::ArchivesOrCurrent::Current,
      //       deleted: false, // include deleted notes
      //       ordering: None,
      //     };
      //     let res = search::search_zknotes(&conn, &config.file_path, uid, &zns)?;

      //     println!("{}", serde_json::to_string_pretty(&res)?);

      //     return Ok(());
      //   }
      //   (_, Err(e)) => {
      //     println!("search parsing error: {:?}", e);
      //     return Ok(());
      //   }
      //   (Err(e), _) => {
      //     println!("error retrieving user id: {:?}", e);
      //     return Ok(());
      //   }
      // }
      ()
    }
    (Some(_username), None) => {
      println!("search_user and search parameters are both required");
    }
    (None, Some(_search)) => {
      println!("search_user and search parameters are both required");
    }
    (None, None) => (),
  };

  println!("Hello, world!");
  Ok(())
}
