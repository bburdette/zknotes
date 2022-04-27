extern crate serde_derive;

pub mod data;
pub mod dbfun;
pub mod email;
pub mod endpoints;
pub mod migrations;
pub mod util;

#[cfg(test)]

mod tests {
  #[test]
  fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
  }
}
