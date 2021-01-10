use rand;
use rand::Rng;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::string::*;
// use std::time::Duration;
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
  let mut inf = File::create(path)?;
  Ok(inf.write(text.as_bytes())?)
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
