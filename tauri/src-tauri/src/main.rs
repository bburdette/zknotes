#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info};
use serde;
use serde_json;
use serde_json::Value;
use std::thread;
use tauri::State;
use zknotes_server_lib::err_main;
use zknotes_server_lib::orgauth::data::WhatMessage;
use zknotes_server_lib::orgauth::endpoints::{Callbacks, Tokener, UuidTokener};
use zknotes_server_lib::zkprotocol::messages::{PublicMessage, ServerResponse, UserMessage};
mod commands;
use commands::{greet, pimsg, uimsg, zimsg, ZkState};

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
fn main() {
  // spawn the web server in a separate thread.
  // let handler = thread::spawn(|| {
  //   println!("meh here");
  //   match err_main() {
  //     Err(e) => error!("error: {:?}", e),
  //     Ok(_) => (),
  //   }
  // });

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
