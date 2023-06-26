use log::{error, info};
use zknotes_server_lib::err_main;

fn main() {
  match err_main() {
    Err(e) => error!("error: {:?}", e),
    Ok(_) => (),
  }
}
