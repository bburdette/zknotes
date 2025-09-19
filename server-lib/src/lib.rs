pub mod config;
pub mod error;
pub mod interfaces;
pub mod jobs;
mod migrations;
pub mod search;
pub mod sqldata;
mod sqltest;
pub mod state;
mod sync;
mod synctest;
use crate::{error as zkerr, state::State};
use actix_cors::Cors;
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_session::{
  config::PersistentSession, storage::CookieSessionStore, Session, SessionMiddleware,
};
use actix_web::{
  cookie::{self, Key},
  dev::Server,
  web, App, HttpRequest, HttpResponse, HttpServer, Result,
};
use chrono;
use clap::Arg;
use config::Config;
use futures_util::TryStreamExt as _;
use girlboss::Girlboss;
use lapin::{self, options::QueueDeclareOptions, types::FieldTable};
use log::{error, info};
pub use orgauth;
use orgauth::{data::UserId, util};
pub use orgauth::{
  data::{AdminResponse, UserResponse},
  endpoints::ActixTokener,
};
pub use rusqlite;
use rusqlite::Connection;
use serde_json;
use simple_error::simple_error;
use sqldata::{get_single_value, local_server_id, LapinInfo};
use std::fs::File;
use std::io::{stdin, Write};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, sync::RwLock};
use std::{error::Error, sync::Arc};
use timer;
use tokio_util::io::StreamReader;
use tracing_actix_web::TracingLogger;
use uuid::Uuid;
pub use zkprotocol;
pub use zkprotocol::content as zc;
pub use zkprotocol::messages::PrivateStreamingMessage;
pub use zkprotocol::search as zs;
use zkprotocol::{
  private::{PrivateError, PrivateReply, PrivateRequest},
  public::{PublicError, PublicReply, PublicRequest},
  upload::UploadReply,
};

/*
use actix_files::NamedFile;

TODO doni't hardcode these paths.  Use config.static_path
fn sitemap(_req: &HttpRequest) -> Result<NamedFile> {
  let stpath = Path::new("static/sitemap.txt");
  Ok(NamedFile::open(stpath)?)
}
*/

async fn favicon(data: web::Data<State>, req: HttpRequest) -> HttpResponse {
  let staticpath = data
    .config
    .static_path
    .clone()
    .unwrap_or(PathBuf::from("static/"));
  let icopath = staticpath.join("favicon.ico");
  match NamedFile::open(&icopath) {
    Ok(f) => f.into_response(&req),
    Err(e) => HttpResponse::from_error(actix_web::error::ErrorInternalServerError(e)),
  }
}

// simple index handler
async fn mainpage(session: Session, data: web::Data<State>, req: HttpRequest) -> HttpResponse {
  info!("remote ip: {:?}, request:{:?}", req.connection_info(), req);

  // logged in?
  let logindata = match interfaces::login_data_for_token(session, &data.config) {
    Ok(optld) => match optld {
      Some(logindata) => serde_json::to_value(logindata).unwrap_or(serde_json::Value::Null),
      _ => serde_json::Value::Null,
    },
    _ => serde_json::Value::Null,
  };

  // TODO: hardcode the UUID of this?
  let errorid = match data.config.error_index_note {
    Some(eid) => serde_json::to_value(eid).unwrap_or(serde_json::Value::Null),
    None => serde_json::Value::Null,
  };

  let adminsettings =
    serde_json::to_value(orgauth::data::admin_settings(&data.config.orgauth_config))
      .unwrap_or(serde_json::Value::Null);

  let mut staticpath = data
    .config
    .static_path
    .clone()
    .unwrap_or(PathBuf::from("static/"));
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

pub fn to_public_error(zke: zkerr::Error, pr: PublicRequest) -> PublicError {
  match zke {
    error::Error::NoteNotFound => PublicError::PbeNoteNotFound(pr),
    error::Error::NoteIsPrivate => PublicError::PbeNoteIsPrivate(pr),
    _ => PublicError::PbeString(zke.to_string()),
  }
}

async fn public(
  data: web::Data<State>,
  item: web::Json<PublicRequest>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "public msg: {:?} connection_info: {:?}",
    &item,
    req.connection_info()
  );

  let public_request = item.into_inner();

  match interfaces::public_interface(
    &data.config,
    &public_request,
    req.connection_info().realip_remote_addr(),
  ) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'public' err: {:?}", e);
      let se = PublicReply::PbyServerError(to_public_error(e, public_request));
      HttpResponse::Ok().json(se)
    }
  }
}

async fn user(
  session: Session,
  data: web::Data<State>,
  item: web::Json<orgauth::data::UserRequest>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "user msg: {:?}  \n connection_info: {:?}",
    &item,
    req.connection_info()
  );
  match async {
    let conn = sqldata::connection_open(data.config.orgauth_config.db.as_path())?;

    interfaces::user_interface(
      &conn,
      &mut ActixTokener { session: &session },
      &data.config,
      item.into_inner(),
    )
    .await
  }
  .await
  {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = UserResponse::UrpServerError(e.to_string());
      HttpResponse::Ok().json(se)
    }
  }
}

async fn admin(
  session: Session,
  data: web::Data<State>,
  item: web::Json<orgauth::data::AdminRequest>,
  req: HttpRequest,
) -> HttpResponse {
  info!(
    "admin msg: {:?}  \n connection_info: {:?}",
    &item,
    req.connection_info()
  );
  let mut cb = sqldata::zknotes_callbacks();
  match orgauth::endpoints::admin_interface_check(
    &mut ActixTokener { session: &session },
    &data.config.orgauth_config,
    &mut cb,
    item.into_inner(),
  ) {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'user' err: {:?}", e);
      let se = AdminResponse::ArpServerError(e.to_string());
      HttpResponse::Ok().json(se)
    }
  }
}

fn session_user(
  conn: &Connection,
  session: Session,
  state: &web::Data<State>,
) -> Result<orgauth::data::User, zkerr::Error> {
  match session.get::<Uuid>("token")? {
    None => Err(zkerr::Error::NotLoggedIn)?,
    Some(token) => orgauth::dbfun::read_user_by_token_api(
      &conn,
      token,
      state.config.orgauth_config.login_token_expiration_ms,
      state.config.orgauth_config.regen_login_tokens,
    )
    .map_err(|e| zkerr::Error::Orgauth(e)),
  }
}

pub fn get_tauri_uid(conn: &Connection) -> Result<Option<UserId>, zkerr::Error> {
  get_single_value(&conn, "last_login").and_then(|x| {
    Ok(x.and_then(|s| {
      serde_json::from_str::<i64>(s.as_str())
        .ok()
        .map(|x| UserId::Uid(x))
    }))
  })
}

async fn file(session: Session, state: web::Data<State>, req: HttpRequest) -> HttpResponse {
  let conn = match sqldata::connection_open(state.config.orgauth_config.db.as_path()) {
    Ok(c) => c,
    Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
  };

  let uid: Option<UserId> = if state.config.tauri_mode == true && state.config.ip == "127.0.0.1" {
    let rs = {
      get_single_value(&conn, "last_login")
        .and_then(|x| Ok(x.and_then(|s| serde_json::from_str::<i64>(s.as_str()).ok())))
    };

    match rs {
      Ok(uid) => uid.map(|id| UserId::Uid(id)),
      Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
    }
  } else {
    match session_user(&conn, session, &state) {
      Ok(u) => Some(u.id),

      Err(e) => match e {
        zkerr::Error::NotLoggedIn => None,
        _ => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
      },
    }
  };

  match req.match_info().get("id") {
    Some(noteid) => {
      let uuid = match Uuid::parse_str(noteid) {
        Ok(id) => id,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
      };
      let nid = match sqldata::note_id_for_uuid(&conn, &uuid) {
        Ok(id) => id,
        Err(e) => return HttpResponse::NotFound().body(e.to_string()),
      };
      let hash = match sqldata::read_zknote_filehash(&conn, uid, nid) {
        Ok(Some(hash)) => hash,
        Ok(None) => {
          return HttpResponse::NotFound().body(format!("note {} not found, or private", uuid))
        }

        Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
      };

      let zkln = match sqldata::read_zklistnote(&conn, &state.config.file_path, uid, nid) {
        Ok(zkln) => zkln,
        Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
      };

      let stpath = state.config.file_path.join(hash);

      match File::open(stpath.clone())
        .and_then(|f| NamedFile::from_file(f, Path::new(zkln.title.as_str())))
      {
        Ok(f) => f.into_response(&req),
        Err(e) => HttpResponse::NotFound().body(format!("{:?}", e)),
      }
    }
    None => HttpResponse::BadRequest().body("file id required: /file/<id>"),
  }
}

async fn receive_files(
  session: Session,
  config: web::Data<State>,
  mut payload: Multipart,
  req: HttpRequest,
) -> HttpResponse {
  let li = match (&config.lapin_channel.as_ref(), get_cookie_id(&req)) {
    (Some(channel), Some(token)) => Some(LapinInfo {
      channel: &channel,
      token,
    }),
    _ => None,
  };
  match make_file_notes(session, &config, &li, &mut payload).await {
    Ok(r) => HttpResponse::Ok().json(r),
    Err(e) => return HttpResponse::InternalServerError().body(format!("{:?}", e)),
  }
}

async fn make_file_notes(
  session: Session,
  state: &web::Data<State>,
  lapin_info: &Option<LapinInfo<'_>>,
  payload: &mut Multipart,
) -> Result<UploadReply, Box<dyn Error>> {
  let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
  let uid = if state.config.tauri_mode {
    get_tauri_uid(&conn)?.ok_or(zkerr::Error::NotLoggedIn)?
  } else {
    session_user(&conn, session, &state)?.id
  };

  let server = local_server_id(&conn)?;

  // Save the files to our temp path.
  let tp = state.config.file_tmp_path.clone();
  let saved_files = save_files(&tp, payload).await?;

  let mut zklns = Vec::new();

  for (name, fp) in saved_files {
    // compute hash.
    let fpath = Path::new(&fp);

    let (nid64, _noteid, _fid) = sqldata::make_file_note(
      &conn,
      &server,
      &lapin_info,
      &state.config.file_path,
      uid,
      &name,
      fpath,
      false,
    )
    .await?;

    // return zknoteedit.
    let listnote = sqldata::read_zklistnote(&conn, &state.config.file_path, Some(uid), nid64)?;

    zklns.push(listnote);
  }
  Ok(UploadReply::UrFilesUploaded(zklns))
}

async fn save_files(
  to_dir: &Path,
  payload: &mut Multipart,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
  // iterate over multipart stream

  let mut rv = Vec::new();

  while let Some(mut field) = payload.try_next().await? {
    // A multipart/form-data stream has to contain `content_disposition`
    let content_disposition = field.content_disposition().clone();
    // .ok_or(simple_error::SimpleError::new("bad"))?;

    let filename = content_disposition
      .get_filename()
      .ok_or(zkerr::Error::String(
        "filename not found in content_disposition".to_string(),
      ))?;

    // can't add this in actix_multipart_rfc7578
    // let mbuuid = content_disposition.get_unknown("noteuuid");

    let wkfilename = Uuid::new_v4().to_string();

    let mut filepath = to_dir.to_path_buf();
    filepath.push(wkfilename);

    let rf = filepath.clone();

    // File::create is blocking operation, use threadpool
    let mut f = web::block(|| std::fs::File::create(rf)).await??;

    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.try_next().await? {
      // filesystem operations are blocking, we have to use threadpool
      f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
    }

    let ps = filepath
      .into_os_string()
      .into_string()
      .map_err(|osstr| simple_error!("couldn't convert filename to string: {:?}", osstr));

    rv.push((filename.to_string(), ps?));
  }

  Ok(rv)
}

fn get_cookie_id(req: &HttpRequest) -> Option<String> {
  req.cookie("id").map(|c| c.value().to_string())
}

async fn private(
  session: Session,
  data: web::Data<State>,
  item: web::Json<PrivateRequest>,
  req: HttpRequest,
) -> HttpResponse {
  let mut state = data.clone();

  let token = get_cookie_id(&req);

  match zk_interface_check(&session, &mut state, token, item.into_inner()).await {
    Ok(sr) => HttpResponse::Ok().json(sr),
    Err(e) => {
      error!("'private' err: {:?}", e);
      HttpResponse::Ok().json(PrivateReply::PvyServerError(PrivateError::PveString(
        e.to_string(),
      )))
    }
  }
}

async fn private_streaming(
  session: Session,
  data: web::Data<State>,
  item: web::Json<PrivateStreamingMessage>,
  _req: HttpRequest,
) -> HttpResponse {
  match zk_interface_check_streaming(&session, &data.config, item.into_inner()).await {
    Ok(hr) => hr,
    Err(e) => {
      error!("'private_streaming err: {:?}", e);
      let se = PrivateReply::PvyServerError(PrivateError::PveString(e.to_string()));
      HttpResponse::Ok().json(se)
    }
  }
}

async fn zk_interface_check(
  session: &Session,
  state: &State,
  token: Option<String>,
  msg: PrivateRequest,
) -> Result<PrivateReply, zkerr::Error> {
  match session.get::<Uuid>("token")? {
    None => Ok(PrivateReply::PvyServerError(PrivateError::PveNotLoggedIn)),
    Some(token_uuid) => {
      let conn = sqldata::connection_open(state.config.orgauth_config.db.as_path())?;
      match orgauth::dbfun::read_user_by_token_api(
        &conn,
        token_uuid,
        state.config.orgauth_config.login_token_expiration_ms,
        state.config.orgauth_config.regen_login_tokens,
      ) {
        Err(e) => {
          info!("read_user_by_token_api error2: {:?}, {:?}", token_uuid, e);

          Ok(PrivateReply::PvyServerError(PrivateError::PveLoginError(
            e.to_string(),
          )))
        }
        Ok(userdata) => {
          // finally!  processing messages as logged in user.
          interfaces::zk_interface_loggedin(state, &conn, token, userdata.id, &msg).await
        }
      }
    }
  }
}

async fn zk_interface_check_streaming(
  session: &Session,
  config: &Config,
  msg: PrivateStreamingMessage,
) -> Result<HttpResponse, Box<dyn Error>> {
  match session.get::<Uuid>("token")? {
    None => Ok(HttpResponse::Ok().json(PrivateReply::PvyServerError(PrivateError::PveNotLoggedIn))),

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
          Ok(
            HttpResponse::Ok().json(PrivateReply::PvyServerError(PrivateError::PveLoginError(
              format!("{:?}", e),
            ))),
          )
        }
        Ok(userdata) => {
          // finally!  processing messages as logged in user.
          interfaces::zk_interface_loggedin_streaming(&config, userdata.id, &msg).await
        }
      }
    }
  }
}

async fn private_upstreaming(
  session: Session,
  data: web::Data<State>,
  body: web::Payload,
  req: HttpRequest,
) -> HttpResponse {
  let li = match (data.lapin_channel.as_ref(), get_cookie_id(&req)) {
    (Some(channel), Some(token)) => Some(LapinInfo {
      channel: &channel,
      token,
    }),
    _ => None,
  };
  match zk_interface_check_upstreaming(&session, &data.config, &li, body).await {
    Ok(hr) => hr,
    Err(e) => {
      error!("'private' err: {:?}", e);
      let se = PrivateReply::PvyServerError(PrivateError::PveString(e.to_string()));
      HttpResponse::Ok().json(se)
    }
  }
}

fn convert_bodyerr(err: actix_web::error::PayloadError) -> std::io::Error {
  error!("convert_err {:?}", err);
  todo!()
}

async fn zk_interface_check_upstreaming(
  session: &Session,
  config: &Config,
  lapin_info: &Option<LapinInfo<'_>>,
  // lapin_channel: &Option<lapin::Channel>,
  body: web::Payload,
  // req: &HttpRequest,
) -> Result<HttpResponse, Box<dyn Error>> {
  match session.get::<Uuid>("token")? {
    None => Ok(HttpResponse::Ok().json(PrivateReply::PvyServerError(PrivateError::PveNotLoggedIn))),

    Some(token) => {
      let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;

      let server = sqldata::local_server_id(&conn)?;

      match orgauth::dbfun::read_user_by_token_api(
        &conn,
        token,
        config.orgauth_config.login_token_expiration_ms,
        config.orgauth_config.regen_login_tokens,
      ) {
        Err(e) => {
          info!("read_user_by_token_api error2: {:?}, {:?}", token, e);

          Ok(
            HttpResponse::Ok().json(PrivateReply::PvyServerError(PrivateError::PveLoginError(
              format!("{:?}", e),
            ))),
          )
        }
        Ok(user) => {
          // finally!  processing messages as logged in user.
          let rstream =
            body.map_err(convert_bodyerr as fn(actix_web::error::PayloadError) -> std::io::Error);

          let mut br = StreamReader::new(rstream);

          // let cid = get_cookie_id(&req);

          let sr = sync::sync_from_stream(
            &conn,
            &lapin_info,
            &server,
            &user,
            &config.file_path,
            None,
            None,
            None,
            &mut sqldata::zknotes_callbacks(),
            &mut br,
          )
          .await?;

          Ok(HttpResponse::Ok().json(sr))
        }
      }
    }
  }
}

// TODO: fns for mobile app default, web server default, I guess desktop too.
pub fn defcon() -> Config {
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
    send_emails: false,
    non_admin_invite: true,
    remote_registration: true,
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
    tauri_mode: false,
    aqmp_uri: None,
    orgauth_config: oc,
  }
}

pub fn load_config(filename: &str) -> Result<Config, Box<dyn Error>> {
  info!("loading config: {}", filename);
  let c = toml::from_str(
    util::load_string(filename)
      .map_err(|e| {
        zkerr::annotate_string(
          format!("failed to load config: '{}'", filename),
          zkerr::Error::String(e.to_string()),
        )
      })?
      .as_str(),
  )?;
  Ok(c)
}

async fn register(data: web::Data<State>, req: HttpRequest) -> HttpResponse {
  orgauth::endpoints::register(&data.config.orgauth_config, req)
}
async fn new_email(data: web::Data<State>, req: HttpRequest) -> HttpResponse {
  orgauth::endpoints::new_email(&data.config.orgauth_config, req)
}

#[actix_web::main]
pub async fn err_main(
  oconfig: Option<Config>,
  logfile: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
  match logfile {
    Some(lf) => {
      let target = Box::new(File::create(lf).expect("Can't create file"));
      env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, log::LevelFilter::Debug)
        // .format(|buf, record| {
        //   writeln!(
        //     buf,
        //     "[{} {} {}:{}] {}",
        //     Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
        //     record.level(),
        //     record.file().unwrap_or("unknown"),
        //     record.line().unwrap_or(0),
        //     record.args()
        //   )
        // })
        .init();
    }
    None => env_logger::init(),
  };

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
  let config = match oconfig {
    // passed in config gets priority
    Some(c) => c, // load_config(c.as_str())?,
    None => match matches.value_of("config") {
      Some(filename) => load_config(filename)?,
      None => load_config("config.toml")?,
    },
  };

  // verify/create file directories.
  if config.createdirs {
    if !std::path::Path::exists(&config.file_tmp_path) {
      std::fs::create_dir_all(&config.file_tmp_path)?
    }
    if !std::path::Path::exists(&config.file_path) {
      std::fs::create_dir_all(&config.file_path)?
    }
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

  // promoting a user to admin?
  if let Some(uid) = matches.value_of("promote_to_admin") {
    let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    let mut user = orgauth::dbfun::read_user_by_name(&conn, uid)?;
    user.admin = true;
    orgauth::dbfun::update_user(&conn, &user)?;

    info!("promoted user {} to admin", uid);
    return Ok(());
  }

  // creating an admin user?
  if let Some(username) = matches.value_of("create_admin_user") {
    // prompt for password.
    println!("Enter password for admin user '{}':", username);
    let mut pwd = String::new();
    stdin().read_line(&mut pwd)?;
    let mut cb = sqldata::zknotes_callbacks();

    let conn = sqldata::connection_open(config.orgauth_config.db.as_path())?;
    // make new registration i
    let rd = orgauth::data::RegistrationData {
      uid: username.to_string(),
      pwd: pwd.trim().to_string(),
      email: "".to_string(),
      remote_url: "".to_string(),
    };

    orgauth::dbfun::new_user(
      &conn,
      &rd,
      None,
      None,
      true,
      None,
      None,
      None,
      None,
      None,
      &mut cb.on_new_user,
    )?;

    println!("admin user created: {}", username);
    return Ok(());
  }

  // Web server is the default.
  let server = init_server(config).await?;
  server.await?;

  Ok(())
}

pub async fn init_server(mut config: Config) -> Result<Server, Box<dyn Error>> {
  // ------------------------------------------------------
  // normal server ops
  info!("server init!");
  if config.static_path == None {
    for (key, value) in env::vars() {
      if key == "ZKNOTES_STATIC_PATH" {
        config.static_path = PathBuf::from_str(value.as_str()).ok();
      }
    }
  }

  match std::fs::create_dir(config.file_tmp_path.clone()) {
    Ok(()) => (),
    Err(e) => {
      if e.kind() == std::io::ErrorKind::AlreadyExists {
        ()
      } else {
        Err(e)?
      }
    }
  }

  info!("config parameters:\n\n{}", toml::to_string_pretty(&config)?);

  let server = sqldata::dbinit(
    config.orgauth_config.db.as_path(),
    config.orgauth_config.login_token_expiration_ms,
  )?;

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

  let lapin_channel = match config.aqmp_uri {
    Some(ref uri) => {
      let conn =
        lapin::Connection::connect(uri.as_str(), lapin::ConnectionProperties::default()).await?;
      let chan = conn.create_channel().await?;
      info!("lapin channel created {:?}", chan);
      chan
        .queue_declare(
          "on_save_zknote",
          QueueDeclareOptions::default(),
          FieldTable::default(),
        )
        .await?;
      chan
        .queue_declare(
          "on_make_file_note",
          QueueDeclareOptions::default(),
          FieldTable::default(),
        )
        .await?;
      Some(chan)
    }
    None => None,
  };

  // create here, not in the HttpServer::new() call,
  // to prevent multiple copies of jobcounter etc.
  let state = web::Data::new(State {
    config: config.clone(),
    girlboss: { Arc::new(RwLock::new(Girlboss::new())) },
    jobcounter: { RwLock::new(0 as i64) },
    server,
    lapin_channel,
  });

  let c = config.clone();
  let server = HttpServer::new(move || {
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
          // tauri all allowed!
          if d.tauri_mode {
            info!("cors allowed {:?}, {:?}", rv, rh);

            true
          } else {
            info!("cors denied: {:?}, {:?}", rv, rh);

            false
          }
        }
      })
      .allow_any_header()
      .allow_any_method()
      .max_age(3600);

    App::new()
      .app_data(state.clone()) // <- create app with shared state
      .wrap(cors)
      .wrap(TracingLogger::default())
      .wrap(
        SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
          .cookie_secure(false)
          // customize session and cookie expiration
          .session_lifecycle(
            PersistentSession::default().session_ttl(cookie::time::Duration::weeks(52)),
          )
          .build(),
      )
      .service(web::resource("/upload").route(web::post().to(receive_files)))
      .service(web::resource("/public").route(web::post().to(public)))
      .service(web::resource("/private").route(web::post().to(private)))
      .service(web::resource("/stream").route(web::post().to(private_streaming)))
      .service(web::resource("/upstream").route(web::post().to(private_upstreaming)))
      .service(web::resource("/user").route(web::post().to(user)))
      .service(web::resource("/admin").route(web::post().to(admin)))
      .service(web::resource(r"/file/{id}").route(web::get().to(file)))
      .service(web::resource(r"/register/{uid}/{key}").route(web::get().to(register)))
      .service(web::resource(r"/newemail/{uid}/{token}").route(web::get().to(new_email)))
      .service(web::resource(r"/favicon.ico").route(web::get().to(favicon)))
      .service(actix_files::Files::new("/static/", staticpath))
      .service(web::resource("/{tail:.*}").route(web::get().to(mainpage)))
  })
  .bind(format!("{}:{}", config.ip, config.port))?
  .run();

  Ok(server)
}
