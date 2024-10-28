use log::error;
use zknotes_server_lib::err_main;

fn main() {
  match err_main(None, None) {
    Err(e) => error!("error: {:?}", e),
    Ok(_) => (),
  }
}
