mod commands;
use commands::{greet, pimsg, uimsg, zimsg, ZkState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(ZkState {
      config: zknotes_server_lib::defcon(),
      uid: None,
    })
    .invoke_handler(tauri::generate_handler![greet, zimsg, pimsg, uimsg])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
