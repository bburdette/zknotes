mod config;
mod email;
mod interfaces;
mod search;
mod sqldata;
mod util;

use actix_files::NamedFile;
use clap::{Arg, SubCommand};
use actix_session::{CookieSession, Session};
use log::{debug, error, log_enabled, info, Level};
use actix_web::middleware::Logger;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use config::Config;
use std::error::Error;
use std::path::{Path, PathBuf};
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
use std::sync::{Arc, Mutex};

fn favicon(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/favicon.ico");
  Ok(NamedFile::open(stpath)?)
}

fn sitemap(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/sitemap.txt");
  Ok(NamedFile::open(stpath)?)
}

// simple index handler
fn mainpage(session: Session, data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!(
    "remote ip: {:?}, request:{:?}",
    req.connection_info(),
    req
  );

  if let Ok(v) = serde_json::to_value(5) {
    session.set("test", v);
  }

  match util::load_string("static/index.html") {
    Ok(s) => {
      // response
      HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(s)
    }
    Err(e) => {
      println!("err");
      HttpResponse::from_error(actix_web::error::ErrorImATeapot(e))
    }
  }
}

fn public(
  data: web::Data<Config>,
  item: web::Json<PublicMessage>,
  _req: HttpRequest,
) -> HttpResponse {
  println!("public msg: {:?}", &item);

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

fn user(session: Session, data: web::Data<Config>, item: web::Json<UserMessage>, _req: HttpRequest) -> HttpResponse {
  println!("user msg: {}, {:?}", &item.what, &item.data);

  let s = session.get::<i64>("test");
  println!("session val: {:?}", s);

  match interfaces::user_interface(&session, &data, item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(e.to_string()),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn register(data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!("registration: uid: {:?}", req.match_info().get("uid"));
  match (req.match_info().get("uid"), req.match_info().get("key")) {
    (Some(uid), Some(key)) => {
      // read user record.  does the reg key match?
      match sqldata::read_user(data.db.as_path(), uid) {
        Ok(user) => {
          println!("user {:?}", user);
          println!("user.registration_key {:?}", user.registration_key);
          println!("key {}", key);
          if user.registration_key == Some(key.to_string()) {
            let mut mu = user;
            mu.registration_key = None;
            match sqldata::update_user(data.db.as_path(), &mu) {
              Ok(_) => HttpResponse::Ok().body(
                format!(
                  "<h1>You are registered!<h1> <a href=\"{}\">\
                 Proceed to the main site</a>",
                  data.mainsite
                )
                .to_string(),
              ),
              Err(_e) => HttpResponse::Ok().body("<h1>registration failed</h1>".to_string()),
            }
          } else {
            HttpResponse::Ok().body("<h1>registration failed</h1>".to_string())
          }
        }
        Err(_e) => HttpResponse::Ok().body("registration key or user doesn't match".to_string()),
      }
    }
    _ => HttpResponse::Ok().body("Uid, key not found!".to_string()),
  }
}

fn defcon() -> Config {
  Config {
    ip: "127.0.0.1".to_string(),
    port: 8000,
    createdirs: false,
    db: PathBuf::from("./mahbloag.db"),
    mainsite: "https:://mahbloag.practica.site/".to_string(),
    appname: "mahbloag".to_string(),
    domain: "practica.site".to_string(),
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
async fn err_main() -> Result<(), Box<dyn Error>> {
  let matches = clap::App::new("zknotes server")
    .version("1.0")
    .author("Ben Burdette")
    .about("Does awesome things")
    .arg(
      Arg::with_name("export")
        .short("e")
        .long("export")
        .value_name("FILE")
        .help("Export database to json")
        .takes_value(true),
    )
    .get_matches();

  // are we exporting the DB?
  match matches.value_of("export") {
    Some(exportfile) => {
      // do that exporting...
      let config = load_config();

      sqldata::dbinit(config.db.as_path())?;

      util::write_string(
        exportfile,
        serde_json::to_string_pretty(&sqldata::export_db(config.db.as_path())?)?.as_str(),
      )?;

      Ok(())
    }
    None => {
      // normal server ops
      env_logger::init();

      info!("server init!");

      let config = load_config();

      println!("config: {:?}", config);

      sqldata::dbinit(config.db.as_path())?;

      let c = config.clone();
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
