use rand;
use rand::Rng;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::string::*;
use std::time::SystemTime;

pub fn load_string(file_name: &str) -> Result<String, Box<dyn Error>> {
  let path = &Path::new(&file_name);
  let mut inf = File::open(path)?;
  let mut result = String::new();
  inf.read_to_string(&mut result)?;
  Ok(result)
}

pub fn write_string(file_name: &str, text: &str) -> Result<usize, Box<dyn Error>> {
  let path = &Path::new(&file_name);
  let mut outf = File::create(path)?;
  Ok(outf.write(text.as_bytes())?)
}

pub fn salt_string() -> String {
  get_rand_string(10)
}

pub fn get_rand_string(len: usize) -> String {
  let mut rng = rand::thread_rng();
  let mut rstr = String::with_capacity(len);

  for _ in 0..len {
    let c = rng.gen::<char>();
    rstr.push(c);
  }

  rstr
}

pub fn now() -> Result<i64, Box<dyn Error>> {
  let nowsecs = SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .map(|n| n.as_secs())?;
  let s: i64 = nowsecs.try_into()?;
  Ok(s * 1000)
}

pub fn is_token_expired(token_expiration_ms: i64, tokendate: i64) -> bool {
  match now() {
    Ok(now) => now < tokendate || (now - tokendate) > token_expiration_ms,
    _ => true,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_expiration() {
    match now() {
      Ok(now) => {
        assert_eq!(is_token_expired(100, now), false);
        assert_eq!(is_token_expired(100, now - 200), true);
        assert_eq!(is_token_expired(100, now + 200), true);
      }
      Err(_) => assert_eq!(2, 4),
    }
  }
}
