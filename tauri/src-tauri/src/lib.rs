mod commands;
use commands::{greet, pimsg, uimsg, zimsg, ZkState};
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(ZkState {
      config: Mutex::new(zknotes_server_lib::defcon()),
      uid: Mutex::new(None),
    })
    .setup(|app| {
      let mut dbpath = app.path().data_dir().unwrap();
      dbpath.push("zknotes.db");
      println!("dbpath: {:?}", dbpath);
      match app.state::<ZkState>().config.lock() {
        Ok(mut config) => {
          config.orgauth_config.db = dbpath;
          zknotes_server_lib::sqldata::dbinit(
            config.orgauth_config.db.as_path(),
            config.orgauth_config.login_token_expiration_ms,
          );
        }
        Err(_) => (),
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![greet, zimsg, pimsg, uimsg])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
