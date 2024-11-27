// design goals:
//   - don't share jobs between users.  A user can only see their own jobs.
//

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct JobId {
  pub uid: i64,
  pub jobno: i64,
}
