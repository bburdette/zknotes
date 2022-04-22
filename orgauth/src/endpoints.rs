pub fn login_data_for_token(
  session: Session,
  config: &Config,
) -> Result<Option<LoginData>, Box<dyn Error>> {
  let conn = sqldata::connection_open(config.db.as_path())?;

  match session.get("token")? {
    None => Ok(None),
    Some(token) => {
      match sqldata::read_user_by_token(&conn, token, Some(config.login_token_expiration_ms)) {
        Ok(user) => Ok(Some(sqldata::login_data(&conn, user.id)?)),
        Err(_) => Ok(None),
      }
    }
  }
}

pub fn user_interface(
  session: &Session,
  config: &Config,
  msg: UserMessage,
) -> Result<ServerResponse, Box<dyn Error>> {
  info!("got a user message: {}", msg.what);
  let conn = sqldata::connection_open(config.db.as_path())?;
  if msg.what.as_str() == "register" {
    let msgdata = Option::ok_or(msg.data, "malformed registration data")?;
    let rd: RegistrationData = serde_json::from_value(msgdata)?;
    // do the registration thing.
    // user already exists?
    match sqldata::read_user_by_name(&conn, rd.uid.as_str()) {
      Ok(_) => {
        // err - user exists.
        Ok(ServerResponse {
          what: "user exists".to_string(),
          content: serde_json::Value::Null,
        })
      }
      Err(_) => {
        // user does not exist, which is what we want for a new user.
        // get email from 'data'.
        let registration_key = Uuid::new_v4().to_string();
        let salt = util::salt_string();

        // write a user record.
        sqldata::new_user(
          Path::new(&config.db),
          rd.uid.clone(),
          hex_digest(
            Algorithm::SHA256,
            (rd.pwd + salt.as_str()).into_bytes().as_slice(),
          ),
          salt,
          rd.email.clone(),
          registration_key.clone().to_string(),
        )?;

        // send a registration email.
        email::send_registration(
          config.appname.as_str(),
          config.domain.as_str(),
          config.mainsite.as_str(),
          rd.email.as_str(),
          rd.uid.as_str(),
          registration_key.as_str(),
        )?;

        // notify the admin.
        email::send_registration_notification(
          config.appname.as_str(),
          config.domain.as_str(),
          config.admin_email.as_str(),
          rd.email.as_str(),
          rd.uid.as_str(),
          registration_key.as_str(),
        )?;

        Ok(ServerResponse {
          what: "registration sent".to_string(),
          content: serde_json::Value::Null,
        })
      }
    }
  } else if msg.what == "login" {
    let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    let login: Login = serde_json::from_value(msgdata.clone())?;

    let userdata = sqldata::read_user_by_name(&conn, login.uid.as_str())?;
    match userdata.registration_key {
      Some(_reg_key) => Ok(ServerResponse {
        what: "unregistered user".to_string(),
        content: serde_json::Value::Null,
      }),
      None => {
        if hex_digest(
          Algorithm::SHA256,
          (login.pwd.clone() + userdata.salt.as_str())
            .into_bytes()
            .as_slice(),
        ) != userdata.hashwd
        {
          // don't distinguish between bad user id and bad pwd!
          Ok(ServerResponse {
            what: "invalid user or pwd".to_string(),
            content: serde_json::Value::Null,
          })
        } else {
          let ld = sqldata::login_data(&conn, userdata.id)?;
          // new token here, and token date.
          let token = Uuid::new_v4();
          sqldata::add_token(&conn, userdata.id, token)?;
          session.set("token", token)?;
          sqldata::update_user(&conn, &userdata)?;
          info!("logged in, user: {:?}", userdata.name);

          Ok(ServerResponse {
            what: "logged in".to_string(),
            content: serde_json::to_value(ld)?,
          })
        }
      }
    }
  } else if msg.what == "logout" {
    session.remove("token");

    Ok(ServerResponse {
      what: "logged out".to_string(),
      content: serde_json::Value::Null,
    })
  } else if msg.what == "resetpassword" {
    let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    let reset_password: ResetPassword = serde_json::from_value(msgdata.clone())?;

    let userdata = sqldata::read_user_by_name(&conn, reset_password.uid.as_str())?;
    match userdata.registration_key {
      Some(_reg_key) => Ok(ServerResponse {
        what: "unregistered user".to_string(),
        content: serde_json::Value::Null,
      }),
      None => {
        let reset_key = Uuid::new_v4();

        // make 'newpassword' record.
        sqldata::add_newpassword(&conn, userdata.id, reset_key.clone())?;

        // send reset email.
        email::send_reset(
          config.appname.as_str(),
          config.domain.as_str(),
          config.mainsite.as_str(),
          userdata.email.as_str(),
          userdata.name.as_str(),
          reset_key.to_string().as_str(),
        )?;

        Ok(ServerResponse {
          what: "resetpasswordack".to_string(),
          content: serde_json::Value::Null,
        })
      }
    }
  } else if msg.what == "setpassword" {
    let msgdata = Option::ok_or(msg.data.as_ref(), "malformed json data")?;
    let set_password: SetPassword = serde_json::from_value(msgdata.clone())?;

    let mut userdata = sqldata::read_user_by_name(&conn, set_password.uid.as_str())?;
    match userdata.registration_key {
      Some(_reg_key) => Ok(ServerResponse {
        what: "unregistered user".to_string(),
        content: serde_json::Value::Null,
      }),
      None => {
        let npwd = sqldata::read_newpassword(&conn, userdata.id, set_password.reset_key)?;

        if is_token_expired(config.reset_token_expiration_ms, npwd) {
          Ok(ServerResponse {
            what: "password reset failed".to_string(),
            content: serde_json::Value::Null,
          })
        } else {
          userdata.hashwd = hex_digest(
            Algorithm::SHA256,
            (set_password.newpwd + userdata.salt.as_str())
              .into_bytes()
              .as_slice(),
          );
          sqldata::remove_newpassword(&conn, userdata.id, set_password.reset_key)?;
          sqldata::update_user(&conn, &userdata)?;
          Ok(ServerResponse {
            what: "setpasswordack".to_string(),
            content: serde_json::Value::Null,
          })
        }
      }
    }
  } else {
    match session.get::<Uuid>("token")? {
      None => Ok(ServerResponse {
        what: "not logged in".to_string(),
        content: serde_json::Value::Null,
      }),
      Some(token) => {
        match sqldata::read_user_by_token(&conn, token, Some(config.login_token_expiration_ms)) {
          Err(e) => {
            info!("read_user_by_token error: {:?}", e);

            Ok(ServerResponse {
              what: "invalid user or pwd".to_string(),
              content: serde_json::Value::Null,
            })
          }
          Ok(userdata) => {
            // finally!  processing messages as logged in user.
            user_interface_loggedin(&config, userdata.id, &msg)
          }
        }
      }
    }
  }
}

fn register(data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!("registration: uid: {:?}", req.match_info().get("uid"));
  match sqldata::connection_open(data.db.as_path()) {
    Ok(conn) => match (req.match_info().get("uid"), req.match_info().get("key")) {
      (Some(uid), Some(key)) => {
        // read user record.  does the reg key match?
        match sqldata::read_user_by_name(&conn, uid) {
          Ok(user) => {
            if user.registration_key == Some(key.to_string()) {
              let mut mu = user;
              mu.registration_key = None;
              match sqldata::update_user(&conn, &mu) {
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
    },

    Err(_e) => HttpResponse::Ok().body("<h1>registration failed</h1>".to_string()),
  }
}

fn new_email(data: web::Data<Config>, req: HttpRequest) -> HttpResponse {
  info!("new email: uid: {:?}", req.match_info().get("uid"));
  match sqldata::connection_open(data.db.as_path()) {
    Ok(conn) => match (req.match_info().get("uid"), req.match_info().get("token")) {
      (Some(uid), Some(tokenstr)) => {
        match Uuid::from_str(tokenstr) {
          Err(_e) => HttpResponse::BadRequest().body("invalid token".to_string()),
          Ok(token) => {
            // read user record.  does the reg key match?
            match sqldata::read_user_by_name(&conn, uid) {
              Ok(user) => {
                match sqldata::read_newemail(&conn, user.id, token) {
                  Ok((email, tokendate)) => {
                    match now() {
                      Err(_e) => HttpResponse::InternalServerError()
                        .body("<h1>'now' failed!</h1>".to_string()),

                      Ok(now) => {
                        if (now - tokendate) > data.email_token_expiration_ms {
                          // TODO token expired?
                          HttpResponse::UnprocessableEntity()
                            .body("<h1>email change failed - token expired</h1>".to_string())
                        } else {
                          // put the email in the user record and update.
                          let mut mu = user.clone();
                          mu.email = email;
                          match sqldata::update_user(&conn, &mu) {
                            Ok(_) => {
                              // delete the change email token record.
                              match sqldata::remove_newemail(&conn, user.id, token) {
                                Ok(_) => (),
                                Err(e) => error!("error removing newemail record: {:?}", e),
                              }
                              HttpResponse::Ok().body(
                                format!(
                                  "<h1>Email address changed!<h1> <a href=\"{}\">\
                                   Proceed to the main site</a>",
                                  data.mainsite
                                )
                                .to_string(),
                              )
                            }
                            Err(_e) => HttpResponse::InternalServerError()
                              .body("<h1>email change failed</h1>".to_string()),
                          }
                        }
                      }
                    }
                  }
                  Err(_e) => HttpResponse::InternalServerError()
                    .body("<h1>email change failed</h1>".to_string()),
                }
              }
              Err(_e) => HttpResponse::BadRequest()
                .body("email change token or user doesn't match".to_string()),
            }
          }
        }
      }
      _ => HttpResponse::BadRequest().body("username or token not found!".to_string()),
    },

    Err(_e) => {
      HttpResponse::InternalServerError().body("<h1>database connection failed</h1>".to_string())
    }
  }
}
