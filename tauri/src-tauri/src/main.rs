use log::{error, info};
use zknotes_server_lib::err_main;



// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {

  match err_main() {
    Err(e) => error!("error: {:?}", e),
    Ok(_) => (),
  }
  
  #[cfg(desktop)]
  app_lib::run();
}

