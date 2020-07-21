extern crate actix_files;
extern crate actix_rt;
extern crate actix_web;
extern crate reqwest;
#[macro_use]
extern crate simple_error;
extern crate crypto_hash;
extern crate env_logger;
extern crate futures;
extern crate json;
extern crate lettre;
extern crate lettre_email;
extern crate rand;
extern crate serde_json;
extern crate time;
extern crate toml;
extern crate uuid;
#[macro_use]
extern crate log;
extern crate rusqlite;
#[macro_use]
extern crate serde_derive;
extern crate base64;

mod email;
mod interfaces;
mod sqldata;
mod util;

use actix_files::NamedFile;
use actix_web::http::{Method, StatusCode};
use actix_web::middleware::Logger;
use actix_web::web::JsonConfig;
use actix_web::{
  http, middleware, web, App, FromRequest, HttpMessage, HttpRequest, HttpResponse, HttpServer,
  Responder, Result,
};
use futures::future::Future;
use interfaces::{PublicMessage, ServerResponse, UserMessage};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
  ip: String,
  port: u16,
  pdfdir: String,
  createdirs: bool,
  pdfdb: String,
}

fn files(req: &HttpRequest) -> Result<NamedFile> {
  println!("files!");
  let path: PathBuf = req.match_info().query("tail").parse()?;
  info!("files: {:?}", path);
  let stpath = Path::new("static/").join(path);
  Ok(NamedFile::open(stpath)?)
}

fn pdffiles(state: web::Data<Config>, req: &HttpRequest) -> Result<NamedFile> {
  let uripath = Path::new(req.uri().path());
  uripath
    .strip_prefix("/pdfs")
    .map_err(|e| actix_web::error::ErrorImATeapot(e))
    .and_then(|path| {
      let stpath = Path::new(&state.pdfdir.to_string()).join(path);
      let nf = NamedFile::open(stpath.clone());
      match nf {
        Ok(_) => println!("ef: "),
        Err(e) => println!("err: {}", e),
      }
      let nf = NamedFile::open(stpath);
      nf.map_err(|e| actix_web::error::ErrorImATeapot(e))
    })
}

fn favicon(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/favicon.ico");
  Ok(NamedFile::open(stpath)?)
}

fn sitemap(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/sitemap.txt");
  Ok(NamedFile::open(stpath)?)
}

// simple index handler
fn mainpage(_state: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  println!("mainpage");
  info!(
    "remote ip: {:?}, request:{:?}",
    req.connection_info().remote(),
    req
  );

  match util::load_string("static/index.html") {
    Ok(s) => {
      println!("okaey");
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
  state: web::Data<Config>,
  item: web::Json<PublicMessage>,
  req: HttpRequest,
) -> HttpResponse {
  println!("model: {:?}", &item);

  let pdb = state.pdfdb.clone();

  match interfaces::public_interface(pdb.as_str(), item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("uh oh, 'public' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(e.to_string()),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn user(state: web::Data<Config>, item: web::Json<UserMessage>, req: HttpRequest) -> HttpResponse {
  println!("model: {:?}", &item);

  let pdb = state.pdfdb.clone();

  match interfaces::user_interface(pdb.as_str(), item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("uh oh, 'user' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(e.to_string()),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn register(state: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!("registration: uid: {:?}", req.match_info().get("uid"));
  match (req.match_info().get("uid"), req.match_info().get("key")) {
    (Some(uid), Some(key)) => {
      let pdfdbp = Path::new(&state.pdfdb);
      // read user record.  does the reg key match?
      match sqldata::read_user(pdfdbp, uid) {
        Ok(user) => {
          if user.registration_key == Some(key.to_string()) {
            let mut mu = user;
            mu.registration_key = None;
            match sqldata::update_user(pdfdbp, &mu) {
              Ok(_) => HttpResponse::Ok().body(
                "<h1>You are registered!<h1> <a href=\"https://www.practica.site\">\
                 Proceed to the main site</a>"
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
    pdfdir: "./pdfs".to_string(),
    createdirs: false,
    pdfdb: "./pdf.db".to_string(),
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

fn err_main() -> Result<(), Box<dyn Error>> {
  env_logger::init();

  info!("server init!");

  let config = load_config();

  let pdfdbp = Path::new(&config.pdfdb);

  if !pdfdbp.exists() {
    sqldata::dbinit(pdfdbp)?;
  }

  // if config.createdirs {
  //   std::fs::create_dir_all(config.pdfdir.clone())?;
  // } else {
  //   if !Path::new(&config.pdfdir).exists() {
  //     Err(std::io::Error::new(
  //       std::io::ErrorKind::NotFound,
  //       "pdfdir not found!",
  //     ))?
  //   }
  // }

  println!("config: {:?}", config);

  // let sys = actix_rt::System::new("pdf-server");

  let c = web::Data::new(config.clone());
  HttpServer::new(move || {
    App::new()
      .register_data(c.clone()) // <- create app with shared state
      // enable logger
      .wrap(middleware::Logger::default())
      .route("/", web::get().to(mainpage))
      .service(web::resource("/public").route(web::post().to(public)))
      .service(web::resource("/user").route(web::post().to(user)))
      .service(web::resource(r"/register/{uid}/{key}").route(web::get().to(register)))
      .service(actix_files::Files::new("/", "static/"))
  })
  .bind(format!("{}:{}", config.ip, config.port))?
  .run()?;

  Ok(())
}
