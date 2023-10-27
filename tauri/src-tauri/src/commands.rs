use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::State;
use zknotes_server_lib::err_main;
use zknotes_server_lib::orgauth::data::WhatMessage;
use zknotes_server_lib::orgauth::endpoints::{Callbacks, Tokener, UuidTokener};
use zknotes_server_lib::zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};

pub struct ZkState {
  pub config: Mutex<zknotes_server_lib::config::Config>,
  pub uid: Mutex<Option<i64>>,
}

#[tauri::command]
pub fn greet(name: &str) -> String {
  println!("greeet");
  format!("Hello, {}!", name)
}

#[derive(Serialize, Deserialize)]
pub struct TimedData {
  utcmillis: u128,
  data: ServerResponse,
}

#[tauri::command]
pub fn zimsg(state: State<ZkState>, msg: UserMessage) -> TimedData {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("zimsg");

  match (
    zknotes_server_lib::interfaces::zk_interface_loggedin(&&state.config.lock().unwrap(), 2, &msg),
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|n| n.as_millis()),
  ) {
    (Ok(sr), Ok(t)) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      TimedData {
        utcmillis: t,
        data: sr,
      }
    }
    (Err(e), _) => TimedData {
      utcmillis: 0,
      data: ServerResponse {
        what: "server error".to_string(),
        content: Value::String(e.to_string()),
      },
    },
    (_, Err(e)) => TimedData {
      utcmillis: 0,
      data: ServerResponse {
        what: "server error".to_string(),
        content: Value::String(e.to_string()),
      },
    },
  }
}

#[tauri::command]
pub fn pimsg(state: State<ZkState>, msg: PublicMessage) -> TimedData {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("pimsg");

  match (
    zknotes_server_lib::interfaces::public_interface(&state.config.lock().unwrap(), msg, None),
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|n| n.as_millis()),
  ) {
    (Ok(sr), Ok(t)) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      TimedData {
        utcmillis: t,
        data: sr,
      }
    }
    (Err(e), _) => TimedData {
      utcmillis: 0,
      data: ServerResponse {
        what: "server error".to_string(),
        content: Value::String(e.to_string()),
      },
    },
    (_, Err(e)) => TimedData {
      utcmillis: 0,
      data: ServerResponse {
        what: "server error".to_string(),
        content: Value::String(e.to_string()),
      },
    },
  }
}

#[tauri::command]
pub fn uimsg(state: State<ZkState>, msg: WhatMessage) -> WhatMessage {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("uimsg");

  let mut ut = UuidTokener { uuid: None };

  let sr = match zknotes_server_lib::interfaces::user_interface(
    &mut ut,
    &state.config.lock().unwrap(),
    msg,
  ) {
    Ok(sr) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      sr
    }
    Err(e) => WhatMessage {
      what: "server error".to_string(),
      data: Some(Value::String(e.to_string())),
    },
  };

  println!("ut {:?}", ut.uuid);

  sr
}

// #[tauri::command]
// pub fn aimsg(msg: UserMessage) -> ServerResponse {
//   // gonna need config obj, uid.
//   // uid could be passed from elm maybe.

//   println!("aimsg");

//   let c = zknotes_server_lib::defcon();

//   match zknotes_server_lib::interfaces::admin_interface(&c, 2, &msg) {
//     Ok(sr) => {
//       println!("sr: {}", sr.what);
//       // serde_json::to_value(&sr).unwrap());
//       sr
//     }
//     Err(e) => ServerResponse {
//       what: "erro".to_string(),
//       content: Value::String("erro".to_string()),
//     },
//   }
// }
