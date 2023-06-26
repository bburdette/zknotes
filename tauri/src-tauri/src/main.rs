#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use log::{error, info};
use std::thread;
use zknotes_server_lib::err_main;

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
fn main() {
  let handler = thread::spawn(|| match err_main() {
    Err(e) => error!("error: {:?}", e),
    Ok(_) => (),
  });

  #[cfg(desktop)]
  app_lib::run();
}
