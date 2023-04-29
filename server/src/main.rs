mod config;
mod interfaces;
mod migrations;
mod search;
mod sqldata;
mod sqltest;
use actix_cors::Cors;
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_session::{CookieSession, Session};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use chrono;
use clap::Arg;
use config::Config;
use either::Either;
use futures_util::TryStreamExt as _;
use log::{error, info};
use orgauth::endpoints::Callbacks;
use orgauth::util;
use rusqlite::{params, Connection};
use serde_json;
use sha256;
use simple_error::simple_error;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{stdin, Write};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use timer;
use util::now;
use uuid::Uuid;
use zkprotocol::content as zc;
use zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};

/*
use actix_files::NamedFile;

TODO don't hardcode these paths.  Use config.static_path
fn favicon(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/favicon.ico");
  Ok(NamedFile::open(stpath)?)
}

fn sitemap(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/sitemap.txt");
  Ok(NamedFile::open(stpath)?)
}
*/

// simple index handler
fn mainpage(session: Session, data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!("remote ip: {:?}, request:{:?}", req.connection_info(), req);

  // logged in?
  let logindata = match interfaces::login_data_for_token(session, &data) {
    Ok(Some(logindata)) => serde_json::to_value(logindata).unwrap_or(serde_json::Value::Null),
    _ => serde_json::Value::Null,
  };

  let errorid = match data.error_index_note {
    Some(eid) => serde_json::to_value(eid).unwrap_or(serde_json::Value::Null),
    None => serde_json::Value::Null,
  };

  let adminsettings = serde_json::to_value(orgauth::data::admin_settings(&data.orgauth_config))
    .unwrap_or(serde_json::Value::Null);

  let mut staticpath = data.static_path.clone().unwrap_or(PathBuf::from("static/"));
  staticpath.push("index.html");
  match staticpath.to_str() {
    Some(path) => match util::load_string(path) {
      Ok(s) => {
        // search and replace with logindata!
        HttpResponse::Ok()
          .content_type("text/html; charset=utf-8")
          .body(
            s.replace("{{logindata}}", logindata.to_string().as_str())
              .replace("{{errorid}}", errorid.to_string().as_str())
              .replace("{{adminsettings}}", adminsettings.to_string().as_str()),
          )
      }
      Err(e) => HttpResponse::from_error(actix_web::error::ErrorImATeapot(e)),
    },
    None => HttpResponse::from_error(actix_web::error::ErrorImATeapot("bad static path")),
  }
}

fn public(
  data: web::Data<Config>,
  item: web::Json<PublicMessage>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "public msg: {:?} connection_info: {:?}",
    &item,
    req.connection_info()
  );

  match interfaces::public_interface(&data, item.into_inner(), req) {
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

fn user(
  session: Session,
  data: web::Data<Config>,
  item: web::Json<orgauth::data::WhatMessage>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "user msg: {}, {:?}  \n connection_info: {:?}",
    &item.what,
    &item.data,
    req.connection_info()
  );
  match interfaces::user_interface(&session, &data, item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = orgauth::data::WhatMessage {
        what: "server error".to_string(),
        data: Some(serde_json::Value::String(e.to_string())),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn admin(
  session: Session,
  data: web::Data<Config>,
  item: web::Json<orgauth::data::WhatMessage>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "admin msg: {}, {:?}  \n connection_info: {:?}",
    &item.what,
    &item.data,
    req.connection_info()
  );
  let mut cb = Callbacks {
    on_new_user: Box::new(sqldata::on_new_user),
    extra_login_data: Box::new(sqldata::extra_login_data_callback),
    on_delete_user: Box::new(sqldata::on_delete_user),
  };
  match orgauth::endpoints::admin_interface_check(
    &session,
    &data.orgauth_config,
    &mut cb,
    item.into_inner(),
  ) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = orgauth::data::WhatMessage {
        what: "server error".to_string(),
        data: Some(serde_json::Value::String(e.to_string())),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn session_user(
  conn: &Connection,
  session: Session,
  config: &web::Data<Config>,
) -> Result<Either<orgauth::data::User, ServerResponse>, Box<dyn Error>> {
  match session.get::<Uuid>("token")? {
    None => Ok(Either::Right(ServerResponse {
      what: "not logged in".to_string(),
      content: serde_json::Value::Null,
    })),
    Some(token) => {
      match orgauth::dbfun::read_user_by_token_api(
        &conn,
        token,
        config.orgauth_config.login_token_expiration_ms,
        config.orgauth_config.regen_login_tokens,
      ) {
        Err(e) => {
          info!("read_user_by_token_api error: {:?}", e);

          Ok(Either::Right(ServerResponse {
            what: "login error".to_string(),
            content: serde_json::to_value(format!("{:?}", e).as_str())?,
          }))
        }
        Ok(userdata) => Ok(Either::Left(userdata)),
      }
    }
  }
}

async fn file(session: Session, config: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  let conn = match sqldata::connection_open(config.orgauth_config.db.as_path()) {
    Ok(c) => c,
    Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
  };

  let suser = match session_user(&conn, session, &config) {
    Ok(Either::Left(user)) => Some(user),
    Ok(Either::Right(_sr)) => None,
    Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
  };

  let uid = suser.map(|user| user.id);

  match req
    .match_info()
    .get("id")
    .and_then(|s| s.parse::<i64>().ok())
  {
    Some(noteid) => {
      let hash = match sqldata::read_zknote_filehash(&conn, uid, &noteid) {
        Ok(Some(hash)) => hash,
        Ok(None) => return HttpResponse::NotFound().body("not found"),
        Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
      };

      let zkln = match sqldata::read_zklistnote(&conn, uid, noteid) {
        Ok(zkln) => zkln,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
      };

      let pstr = format!("files/{}", hash);
      let stpath = Path::new(pstr.as_str());

      // Self::from_file(File::open(&path)?, path)
      match File::open(stpath).and_then(|f| NamedFile::from_file(f, Path::new(zkln.title.as_str())))
      {
        Ok(f) => f
          .into_response(&req)
          .unwrap_or(HttpResponse::NotFound().json(())),
        Err(e) => HttpResponse::NotFound().body(format!("{:?}", e)),
      }
    }
    None => HttpResponse::BadRequest().body("file id required: /files/<id>"),
  }
}

async fn receive_files(
  session: Session,
  config: web::Data<Config>,
  mut payload: Multipart,
) -> HttpResponse {
  match make_file_notes(session, config, &mut payload).await {
    Ok(r) => HttpResponse::Ok().json(r),
    Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
  }
}

// TODO: move out of main.rs
async fn make_file_notes(
  session: Session,
  config: web::Data<Config>,
  payload: &mut Multipart,
  // ) -> Result<ServerResponse, Box<dyn Error>> {
) -> Result<ServerResponse, Box<dyn Error>> {
  let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
  let userdata = match session_user(&conn, session, &config)? {
    Either::Left(ud) => ud,
    Either::Right(sr) => return Ok(sr),
  };

  // Save the files to our temp path.
  let tp = config.file_tmp_path.clone();
  let saved_files = save_files(&tp, payload).await?;

  let mut zklns = Vec::new();

  for (name, fp) in saved_files {
    // compute hash.
    let fpath = Path::new(&fp);
    let noteid = make_file_note(&conn, userdata.id, name, fpath)?;
    /*
    let fh = sha256::try_digest(fpath)?;
    let size = std::fs::metadata(fpath)?.len();
    let fhp = format!("files/{}", fh);
    let hashpath = Path::new(&fhp);

    // file exists?
    if hashpath.exists() {
      // new file already exists.
      std::fs::remove_file(fpath)?;
    } else {
      // move into hashed-files dir.
      std::fs::rename(fpath, hashpath)?;
    }

    // table entry exists?
    let oid: Option<i64> =
      match conn.query_row("select id from file where hash = ?1", params![fh], |row| {
        Ok(row.get(0)?)
      }) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(x) => Err(Box::new(x)),
      }?;

    // use existing id, or create new
    let fid = match oid {
      Some(id) => id,
      None => {
        let now = now()?;

        // add table entry
        conn.execute(
          "insert into file (hash, createdate, size)
                 values (?1, ?2, ?3)",
          params![fh, now, size],
        )?;
        conn.last_insert_rowid()
      }
    };

    // now make a new note.
    let sn = sqldata::save_zknote(
      &conn,
      userdata.id,
      &zc::SaveZkNote {
        id: None,
        title: name,
        pubid: None,
        content: "".to_string(),
        editable: false,
        showtitle: false,
        deleted: false,
      },
    )?;

    // set the file id in that note.
    sqldata::set_zknote_file(&conn, sn.id, fid)?;i
    */

    // return zknoteedit.
    let listnote = sqldata::read_zklistnote(&conn, Some(userdata.id), noteid)?;
    info!(
      "user#filer_uploaded-zknote: {} - {}",
      listnote.id, listnote.title
    );

    zklns.push(listnote);
  }
  Ok(ServerResponse {
    what: "savedfiles".to_string(),
    content: serde_json::to_value(zklns)?,
  })
}

fn make_file_note(
  conn: &Connection,
  uid: i64,
  name: String,
  fpath: &Path,
) -> Result<i64, orgauth::error::Error> {
  // compute hash.
  // let fpath = Path::new(&filepath);
  let fh = sha256::try_digest(fpath)?;
  let size = std::fs::metadata(fpath)?.len();
  let fhp = format!("files/{}", fh);
  let hashpath = Path::new(&fhp);

  // file exists?
  if hashpath.exists() {
    // new file already exists.
    std::fs::remove_file(fpath)?;
  } else {
    // move into hashed-files dir.
    std::fs::rename(fpath, hashpath)?;
  }

  // table entry exists?
  let oid: Option<i64> =
    match conn.query_row("select id from file where hash = ?1", params![fh], |row| {
      Ok(row.get(0)?)
    }) {
      Ok(v) => Ok(Some(v)),
      Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
      Err(x) => Err(x),
    }?;

  // use existing id, or create new
  let fid = match oid {
    Some(id) => id,
    None => {
      let now = now()?;

      // add table entry
      conn.execute(
        "insert into file (hash, createdate, size)
                 values (?1, ?2, ?3)",
        params![fh, now, size],
      )?;
      conn.last_insert_rowid()
    }
  };

  // now make a new note.
  let sn = sqldata::save_zknote(
    &conn,
    uid,
    &zc::SaveZkNote {
      id: None,
      title: name,
      pubid: None,
      content: "".to_string(),
      editable: false,
      showtitle: false,
      deleted: false,
    },
  )?;

  // set the file id in that note.
  sqldata::set_zknote_file(&conn, sn.id, fid)?;

  Ok(sn.id)
}

async fn save_files(
  to_dir: &Path,
  payload: &mut Multipart,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
  // iterate over multipart stream

  let mut rv = Vec::new();

  while let Some(mut field) = payload.try_next().await? {
    // A multipart/form-data stream has to contain `content_disposition`
    let content_disposition = field
      .content_disposition()
      .ok_or(simple_error::SimpleError::new("bad"))?;

    let filename = content_disposition
      .get_filename()
      .unwrap_or("filename not found");

    let wkfilename = Uuid::new_v4().to_string();

    let mut filepath = to_dir.to_path_buf();
    filepath.push(wkfilename);

    let rf = filepath.clone();

    // File::create is blocking operation, use threadpool
    let mut f = web::block(|| std::fs::File::create(filepath)).await?;

    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.try_next().await? {
      // filesystem operations are blocking, we have to use threadpool
      f = web::block(move || f.write_all(&chunk).map(|_| f)).await?;
    }

    let ps = rf
      .into_os_string()
      .into_string()
      .map_err(|osstr| simple_error!("couldn't convert filename to string: {:?}", osstr));

    rv.push((filename.to_string(), ps?));
  }

  Ok(rv)
}

fn private(
  session: Session,
  data: web::Data<Config>,
  item: web::Json<UserMessage>,
  _req: HttpRequest,
) -> HttpResponse {
  match zk_interface_check(&session, &data, item.into_inner()) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'private' err: {:?}", e);
      let se = ServerResponse {
        what: "server error".to_string(),
        content: serde_json::Value::String(e.to_string()),
      };
      HttpResponse::Ok().json(se)
    }
  }
}

fn zk_interface_check(
  session: &Session,
  config: &Config,
  msg: UserMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  match session.get::<Uuid>("token")? {
    None => Ok(ServerResponse {
      what: "not logged in".to_string(),
      content: serde_json::Value::Null,
    }),
    Some(token) => {
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
      match orgauth::dbfun::read_user_by_token_api(
        &conn,
        token,
        config.orgauth_config.login_token_expiration_ms,
        config.orgauth_config.regen_login_tokens,
      ) {
        Err(e) => {
          info!("read_user_by_token_api error2: {:?}, {:?}", token, e);

          Ok(ServerResponse {
            what: "login error".to_string(),
            content: serde_json::to_value(format!("{:?}", e).as_str())?,
          })
        }
        Ok(userdata) => {
          // finally!  processing messages as logged in user.
          interfaces::zk_interface_loggedin(&config, userdata.id, &msg)
        }
      }
    }
  }
}

fn defcon() -> Config {
  let oc = orgauth::data::Config {
    db: PathBuf::from("./zknotes.db"),
    mainsite: "http://localhost:8000".to_string(),
    appname: "zknotes".to_string(),
    emaildomain: "zknotes.com".to_string(),
    admin_email: "admin@admin.admin".to_string(),
    regen_login_tokens: true,
    login_token_expiration_ms: None, // Some(7 * 24 * 60 * 60 * 1000), // 7 days in milliseconds
    email_token_expiration_ms: 1 * 24 * 60 * 60 * 1000, // 1 day in milliseconds
    reset_token_expiration_ms: 1 * 24 * 60 * 60 * 1000, // 1 day in milliseconds
    invite_token_expiration_ms: 7 * 24 * 60 * 60 * 1000, // 7 day in milliseconds
    open_registration: false,
    non_admin_invite: true,
  };
  Config {
    ip: "127.0.0.1".to_string(),
    port: 8000,
    createdirs: false,
    altmainsite: [].to_vec(),
    static_path: None,
    file_tmp_path: Path::new("./temp").to_path_buf(),
    file_path: Path::new("./files").to_path_buf(),
    error_index_note: None,
    orgauth_config: oc,
  }
}

fn load_config(filename: &str) -> Result<Config, Box<dyn Error>> {
  info!("loading config: {}", filename);
  let c = toml::from_str(util::load_string(filename)?.as_str())?;
  Ok(c)
}

fn main() {
  match err_main() {
    Err(e) => error!("error: {:?}", e),
    Ok(_) => (),
  }
}

fn register(data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  orgauth::endpoints::register(&data.orgauth_config, req)
}
fn new_email(data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  orgauth::endpoints::new_email(&data.orgauth_config, req)
}

#[actix_web::main]
async fn err_main() -> Result<(), Box<dyn Error>> {
  env_logger::init();

  let matches = clap::App::new("zknotes server")
    .version("1.0")
    .author("Ben Burdette")
    .about("zettelkasten web server")
    .arg(
      Arg::with_name("export")
        .short("e")
        .long("export")
        .value_name("FILE")
        .help("Export database to json")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("config")
        .short("c")
        .long("config")
        .value_name("FILE")
        .help("specify config file")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("write_config")
        .short("w")
        .long("write_config")
        .value_name("FILE")
        .help("write default config file")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("promote_to_admin")
        .short("p")
        .long("promote_to_admin")
        .value_name("user name")
        .help("grant admin privileges to user")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("create_admin_user")
        .short("a")
        .long("create_admin_user")
        .value_name("user name")
        .help("create new admin user")
        .takes_value(true),
    )
    .get_matches();

  // writing a config file?
  if let Some(filename) = matches.value_of("write_config") {
    util::write_string(filename, toml::to_string_pretty(&defcon())?.as_str())?;
    info!("default config written to file: {}", filename);
    return Ok(());
  }

  // specifying a config file?  otherwise try to load the default.
  let mut config = match matches.value_of("config") {
    Some(filename) => load_config(filename)?,
    None => load_config("config.toml")?,
  };

  // verify/create file directories.

  // TODO upgrade when stable
  // if !std::fs::try_exists(config.file_tmp_path)? {
  if !std::path::Path::exists(&config.file_tmp_path) {
    std::fs::create_dir_all(&config.file_tmp_path)?
  }
  // TODO upgrade when stable
  // if !std::fs::try_exists(config.file_path)? {
  if !std::path::Path::exists(&config.file_path) {
    std::fs::create_dir_all(&config.file_path)?
  }

  // are we exporting the DB?
  if let Some(exportfile) = matches.value_of("export") {
    // do that exporting...

    sqldata::dbinit(
      config.orgauth_config.db.as_path(),
      config.orgauth_config.login_token_expiration_ms,
    )?;

    util::write_string(
      exportfile,
      serde_json::to_string_pretty(&sqldata::export_db(config.orgauth_config.db.as_path())?)?
        .as_str(),
    )?;

    return Ok(());
  }

  // normal server ops
  info!("server init!");
  if config.static_path == None {
    for (key, value) in env::vars() {
      if key == "ZKNOTES_STATIC_PATH" {
        config.static_path = PathBuf::from_str(value.as_str()).ok();
      }
    }
  }

  info!("config parameters:\n\n{}", toml::to_string_pretty(&config)?);

  sqldata::dbinit(
    config.orgauth_config.db.as_path(),
    config.orgauth_config.login_token_expiration_ms,
  )?;

  // promoting a user to admin?
  if let Some(uid) = matches.value_of("promote_to_admin") {
    let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    let mut user = orgauth::dbfun::read_user_by_name(&conn, uid)?;
    user.admin = true;
    orgauth::dbfun::update_user(&conn, &user)?;

    println!("promoted user {} to admin", uid);
    return Ok(());
  }

  // creating an admin user?
  if let Some(username) = matches.value_of("create_admin_user") {
    // prompt for password.
    println!("Enter password for admin user '{}':", username);
    let mut pwd = String::new();
    stdin().read_line(&mut pwd)?;
    let mut cb = Callbacks {
      on_new_user: Box::new(sqldata::on_new_user),
      extra_login_data: Box::new(sqldata::extra_login_data_callback),
      on_delete_user: Box::new(sqldata::on_delete_user),
    };

    let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    // make new registration i
    let rd = orgauth::data::RegistrationData {
      uid: username.to_string(),
      pwd: pwd.trim().to_string(),
      email: "".to_string(),
    };

    println!("rd: {:?}", rd);

    orgauth::dbfun::new_user(
      &conn,
      &rd,
      None,
      None,
      true,
      None,
      // &mut Box::new(sqldata::on_new_user),
      &mut cb.on_new_user,
    )?;

    println!("admin user created: {}", username);
    return Ok(());
  }

  let timer = timer::Timer::new();

  let ptconfig = config.clone();

  let _guard =
    timer.schedule_repeating(
      chrono::Duration::days(1),
      move || match orgauth::dbfun::purge_tokens(&ptconfig.orgauth_config) {
        Err(e) => error!("purge_tokens error: {}", e),
        Ok(_) => (),
      },
    );

  let c = config.clone();
  HttpServer::new(move || {
    let staticpath = c.static_path.clone().unwrap_or(PathBuf::from("static/"));
    let d = c.clone();
    let cors = Cors::default()
      .allowed_origin_fn(move |rv, rh| {
        if *rv == d.orgauth_config.mainsite {
          true
        } else if d.altmainsite.iter().any(|am| *rv == am) {
          true
        } else if rv == "https://29a.ch"
          && rh.method == "GET"
          && (rh.uri.to_string().starts_with("/static/")
            || (rh.uri.to_string().starts_with("/file/")))
        {
          true
        } else {
          info!("cors denied: {:?}, {:?}", rv, rh);
          false
        }
      })
      .allow_any_header()
      .allow_any_method()
      .max_age(3600);

    App::new()
      .data(c.clone()) // <- create app with shared state
      .wrap(cors)
      .wrap(middleware::Logger::default())
      .wrap(
        CookieSession::signed(&[0; 32]) // <- create cookie based session middleware
          .secure(false) // allows for dev access
          .max_age(10 * 24 * 60 * 60), // 10 days
      )
      .service(web::resource("/upload").route(web::post().to(receive_files)))
      .service(web::resource("/public").route(web::post().to(public)))
      .service(web::resource("/private").route(web::post().to(private)))
      .service(web::resource("/user").route(web::post().to(user)))
      .service(web::resource("/admin").route(web::post().to(admin)))
      // .service(web::resource(r"/files/{hash}").route(web::get().to(files)))
      .service(web::resource(r"/file/{id}").route(web::get().to(file)))
      .service(web::resource(r"/register/{uid}/{key}").route(web::get().to(register)))
      .service(web::resource(r"/newemail/{uid}/{token}").route(web::get().to(new_email)))
      .service(actix_files::Files::new("/static/", staticpath))
      .service(web::resource("/{tail:.*}").route(web::get().to(mainpage)))
  })
  .bind(format!("{}:{}", config.ip, config.port))?
  .run()
  .await?;

  Ok(())
}
