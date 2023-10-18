mod commands;
use commands::{greet, pimsg, uimsg, zimsg, ZkState};
use zknotes_server_lib::sqldata;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let config = zknotes_server_lib::defcon();
  let ret = zknotes_server_lib::sqldata::dbinit(
    config.orgauth_config.db.as_path(),
    config.orgauth_config.login_token_expiration_ms,
  );

  println!("dbinit ret: {:?}", ret);

  tauri::Builder::default()
    .manage(ZkState {
      config: zknotes_server_lib::defcon(),
      uid: None,
    })
    .invoke_handler(tauri::generate_handler![greet, zimsg, pimsg, uimsg])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
