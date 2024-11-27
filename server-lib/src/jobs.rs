#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct JobId {
  pub uid: i64,
  pub jobno: i64,
}
