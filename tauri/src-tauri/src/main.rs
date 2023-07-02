#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info};
use serde;
use serde_json;
use serde_json::Value;
use std::thread;
use tauri::State;
use zknotes_server_lib::err_main;
use zknotes_server_lib::orgauth::data::WhatMessage;
use zknotes_server_lib::zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};

struct ZkState {
  config: zknotes_server_lib::config::Config,
  uid: Option<i64>,
}

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
fn main() {
  // spawn the web server in a separate thread.
  let handler = thread::spawn(|| {
    println!("meh here");
    match err_main() {
      Err(e) => error!("error: {:?}", e),
      Ok(_) => (),
    }
  });

  // #[cfg(desktop)]
  // app_lib::run();
  tauri::Builder::default()
    .manage(ZkState {
      config: zknotes_server_lib::defcon(),
      uid: None,
    })
    .invoke_handler(tauri::generate_handler![greet, zimsg, pimsg, uimsg])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
  println!("greeet");
  format!("Hello, {}!", name)
}

#[tauri::command]
fn zimsg(msg: UserMessage) -> ServerResponse {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("zimsg");

  let c = zknotes_server_lib::defcon();

  match zknotes_server_lib::interfaces::zk_interface_loggedin(&c, 2, &msg) {
    Ok(sr) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      sr
    }
    Err(e) => ServerResponse {
      what: "erro".to_string(),
      content: Value::String("erro".to_string()),
    },
  }
}

#[tauri::command]
fn pimsg(state: State<ZkState>, msg: PublicMessage) -> ServerResponse {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("pimsg");

  let c = zknotes_server_lib::defcon();

  match zknotes_server_lib::interfaces::public_interface(&state.config, msg, None) {
    Ok(sr) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      sr
    }
    Err(e) => ServerResponse {
      what: "erro".to_string(),
      content: Value::String("erro".to_string()),
    },
  }
}

#[tauri::command]
fn uimsg(state: State<ZkState>, msg: WhatMessage) -> WhatMessage {
  // gonna need config obj, uid.
  // uid could be passed from elm maybe.

  println!("uimsg");

  let c = zknotes_server_lib::defcon();

  match zknotes_server_lib::interfaces::user_interface(&state.config, msg) {
    Ok(sr) => {
      println!("sr: {}", sr.what);
      // serde_json::to_value(&sr).unwrap());
      sr
    }
    Err(e) => WhatMessage {
      what: "erro".to_string(),
      data: Some(Value::String("erro".to_string())),
    },
  }
}

// #[tauri::command]
// fn aimsg(msg: UserMessage) -> ServerResponse {
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
