mod config;
mod email;
mod errors;
mod icontent;
mod importdb;
mod indra;
mod indratest;
use indradb::Datastore;
mod indra_util;
mod interfaces;
mod isearch;
mod search;
mod sqldata;
mod user;
mod util;
use actix_files::NamedFile;
use clap::{Arg, SubCommand};
use user::ZkDatabase;
// use actix_web::http::{Method, StatusCode};
use actix_session::{CookieSession, Session};
use log::{debug, error, log_enabled, info, Level};
use actix_web::middleware::Logger;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use config::{Config, State};
use indra as I;
use std::error::Error;
use std::path::{Path, PathBuf};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use std::sync::{Arc, Mutex};
use serde_json;
use uuid::Uuid;

fn favicon(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/favicon.ico");
  Ok(NamedFile::open(stpath)?)
}

fn sitemap(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/sitemap.txt");
  Ok(NamedFile::open(stpath)?)
}


// simple index handler
fn mainpage(session: Session, state: web::Data<State>, req: HttpRequest) -> HttpResponse {
  info!(
    "remote ip: {:?}, request:{:?}",
    req.connection_info(),
    req
  );

  // logged in?
  let logindata = match interfaces::login_data_for_token(session, &state.config) {
    Ok(Some(logindata)) => serde_json::to_value(logindata).unwrap_or(serde_json::Value::Null),
    _ => serde_json::Value::Null,
  };

  match util::load_string("static/index.html") {
    Ok(s) => {
      // search and replace with logindata!
      HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(s.replace("{{logindata}}", logindata.to_string().as_str()))
    }
    Err(e) => {
      println!("err");
      HttpResponse::from_error(actix_web::error::ErrorImATeapot(e))
    }
  }
}

fn public(
  state: web::Data<State>,
  item: web::Json<PublicMessage>,
  _req: HttpRequest,
) -> HttpResponse {
  info!("public msg: {:?}", &item);

  match interfaces::public_interface(&data, item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'public' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(e.to_string()),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn user(session: Session, state: web::Data<State>, item: web::Json<UserMessage>, _req: HttpRequest) -> HttpResponse {
  info!("user msg: {}, {:?}", &item.what, &item.data);
  match interfaces::user_interface(&session, &state, item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(format!("{:?}", e)),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn register(state: web::Data<State>, req: HttpRequest) -> HttpResponse {
  info!("registration: uid: {:?}", req.match_info().get("uid"));

  match state.db.transaction() {
    Err(e) => HttpResponse::Ok().body(format!("{:?}", e)),
    Ok(itr) => {
      match (req.match_info().get("uid"), req.match_info().get("key")) {
        (Some(uid), Some(key)) => {
          // read user record.  does the reg key match?
          match I::read_user(&itr, &uid.to_string()) {
            Ok(user) => {
              println!("user {:?}", user);
              println!("user.registration_key {:?}", user.registration_key);
              println!("key {}", key);
              if user.registration_key == Some(key.to_string()) {
                let mut mu = user;
                mu.registration_key = None;
                match I::save_user(&itr, &mu) {
                  Ok(_) => HttpResponse::Ok().body(
                    format!(
                      "<h1>You are registered!<h1> <a href=\"{}\">\
                     Proceed to the main site</a>",
                      state.config.mainsite
                    )
                    .to_string(),
                  ),
                  Err(_e) => HttpResponse::Ok().body("<h1>registration failed</h1>".to_string()),
                }
              } else {
                HttpResponse::Ok().body("<h1>registration failed</h1>".to_string())
              }
            }
            Err(_e) => {
              HttpResponse::Ok().body("registration key or user doesn't match".to_string())
           }
          }
        }
        _ => HttpResponse::Ok().body("Uid, key not found!".to_string()),
      }
    }
  }
}

fn defcon() -> Config {
  Config {
    ip: "127.0.0.1".to_string(),
    port: 8000,
    createdirs: false,
    db: PathBuf::from("./mahbloag.db"),
    indradb: PathBuf::from("./indra2"),
    mainsite: "https:://mahbloag.practica.site/".to_string(),
    appname: "mahbloag".to_string(),
    domain: "practica.site".to_string(),
    admin_email: "admin@practica.site".to_string(),
    token_expiration_ms: 7*24*60*60*1000,  // 7 days in milliseconds
  }
}

fn load_config() -> Config {
  match util::load_string("config.toml") {
    Err(e) => {
      error!("error loading config.toml: {:?}", e);
      defcon()
    }
    Ok(config_str) => match toml::from_str(config_str.as_str()) {
      Ok(c) => c,
      Err(e) => {
        error!("error loading config.toml: {:?}", e);
        defcon()
      }
    },
  }
}

fn main() {
  match err_main() {
    Err(e) => println!("error: {:?}", e),
    Ok(_) => (),
  }
}

#[actix_web::main]
async fn err_main() -> Result<(), errors::Error> {
  let matches = clap::App::new("zknotes server")
    .version("1.0")
    .author("Ben Burdette")
    .arg(
      Arg::with_name("export")
        .short("e")
        .long("export")
        .value_name("FILE")
        .help("Export database to json")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("import")
        .short("i")
        .long("import")
        .value_name("FILE")
        .help("Import database from json")
        .takes_value(true),
    )
    .get_matches();

  // are we exporting the DB?
  match (
    matches.value_of("import"),
    matches.value_of("export"),
    matches.is_present("indra-test"),
  ) {
    (Some(importfile), _, _) => {
      // do that exporting...
      let config = load_config();

      let db = util::load_string(importfile)?;

      let zd: ZkDatabase = serde_json::from_str(db.as_str())?;

      importdb::import_db(&zd, &config.indradb)?;

      Ok(())
    }
    (_, Some(exportfile), _) => {
      // do that exporting...
      let config = load_config();

      sqldata::dbinit(config.db.as_path())?;

      util::write_string(
        exportfile,
        serde_json::to_string_pretty(&sqldata::export_db(config.db.as_path())?)?.as_str(),
      )?;

      Ok(())
    }
    _ => {
      // normal server ops
      env_logger::init();

      info!("server init!");

      let config = load_config();

      println!("config: {:?}", config);

      sqldata::dbinit(config.db.as_path())?;
      let sc = indradb::SledConfig::with_compression(None);
      println!("got sc");

      let ids = sc.open(config.indradb.clone())?;
      println!("got ids");

      let svs = I::get_systemvs(&ids.transaction()?)?;

      let c = web::Data::new(State {
        config: config.clone(),
        db: ids,
        svs: svs,
      });
      HttpServer::new(move || {
        App::new()
          .data(c.clone()) // <- create app with shared state
          .wrap(middleware::Logger::default())
          .wrap(
            CookieSession::signed(&[0; 32]) // <- create cookie based session middleware
              .secure(true),
          )
          .service(web::resource("/public").route(web::post().to(public)))
          .service(web::resource("/user").route(web::post().to(user)))
          .service(web::resource(r"/register/{uid}/{key}").route(web::get().to(register)))
          .service(actix_files::Files::new("/static/", "static/"))
          .service(web::resource("/{tail:.*}").route(web::get().to(mainpage)))
      })
      .bind(format!("{}:{}", config.ip, config.port))?
      .run()
      .await?;

      Ok(())
    }
  }
}
