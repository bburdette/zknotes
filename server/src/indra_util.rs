use errors;
use indradb::Transaction;
use simple_error::SimpleError;
use uuid::Uuid;

pub fn getprop<T: indradb::Transaction, R>(
  itr: &T,
  vq: &indradb::VertexQuery,
  x: &str,
) -> Result<R, errors::Error>
where
  R: serde::de::DeserializeOwned,
{
  Ok(serde_json::from_value::<R>(
    itr
      .get_vertex_properties(indradb::VertexPropertyQuery::new(vq.clone(), x))?
      .first()
      .ok_or(SimpleError::new(
        format!("property not found: {}", x).as_str(),
      ))?
      .value
      .clone(),
  )?)
}

pub fn getoptprop<T: indradb::Transaction, R>(
  itr: &T,
  vq: &indradb::VertexQuery,
  x: &str,
) -> Result<Option<R>, errors::Error>
where
  R: serde::de::DeserializeOwned,
{
  let r = match itr
    .get_vertex_properties(indradb::VertexPropertyQuery::new(vq.clone(), x))?
    .first()
  {
    Some(vp) => Some(serde_json::from_value::<R>(vp.value.clone())?),
    None => None,
  };
  Ok(r)
}

pub fn getoptedgeprop<T: indradb::Transaction, R>(
  itr: &T,
  eq: &indradb::EdgeQuery,
  x: &str,
) -> Result<Option<R>, errors::Error>
where
  R: serde::de::DeserializeOwned,
{
  let r = match itr
    .get_edge_properties(indradb::EdgePropertyQuery::new(eq.clone(), x))?
    .first()
  {
    Some(ep) => Some(serde_json::from_value::<R>(ep.value.clone())?),
    None => None,
  };
  Ok(r)
}

// first first in a query that matches the test.
pub fn find_first<T: indradb::Transaction, F>(
  itr: &T,
  vpq: indradb::VertexPropertyQuery,
  test: F,
) -> Result<Option<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool,
{
  let vps = itr.get_vertex_properties(vpq)?;
  let ret = vps.iter().find(test);
  match ret {
    Some(d) => Ok(Some(d.clone())),
    None => Ok(None),
  }
}

// first first in a query that matches the test.
pub fn find_all<T: indradb::Transaction, F>(
  itr: &T,
  vpq: indradb::VertexPropertyQuery,
  test: F,
) -> Result<Vec<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool,
{
  let vps = itr.get_vertex_properties(vpq)?;
  let mut ret = Vec::new();
  for vp in vps.iter().filter(test) {
    ret.push(vp.clone());
  }
  Ok(ret)
  // Ok(ret)
  // match ret {
  //   Some(d) => Ok(Some(d.clone())),
  //   None => Ok(None),
  // }
}

// like find_first, but keeps going.
pub fn find_first_q<T: indradb::Transaction, F, Q>(
  itr: &T,
  vpqf: Q,
  test: F,
) -> Result<Option<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool + Copy,
  Q: Fn(Option<Uuid>) -> Result<indradb::VertexPropertyQuery, errors::Error>,
{
  let mut ret = None;
  let uuid = None;
  let mut vpq = vpqf(uuid)?;
  let mut vps = itr.get_vertex_properties(vpq)?;
  while (!vps.is_empty()) {
    match vps.iter().find(test) {
      Some(r) => {
        ret = Some(r.clone());
        break;
      }
      None => match vps.last() {
        Some(l) => {
          vpq = vpqf(Some(l.id))?;
          vps = itr.get_vertex_properties(vpq)?;
        }
        None => break,
      },
    }
  }
  match ret {
    Some(r) => Ok(Some(r.clone())),
    None => Ok(None),
  }
}

// like find_first_q, but finds multiple matches.
pub fn find_all_q<T: indradb::Transaction, F, Q>(
  itr: &T,
  vpqf: Q,
  test: F,
  max: Option<usize>,
) -> Result<Vec<indradb::VertexProperty>, errors::Error>
where
  F: FnMut(&&indradb::VertexProperty) -> bool + Copy,
  Q: Fn(Option<Uuid>) -> Result<indradb::VertexPropertyQuery, errors::Error>,
{
  let mut ret = Vec::new();
  let uuid = None;
  let mut vpq = vpqf(uuid)?;
  let mut vps = itr.get_vertex_properties(vpq)?;
  while (!vps.is_empty()) {
    match vps.iter().find(test) {
      Some(r) => {
        ret.push(r.clone());
        // if we've reached max records, bail
        if max.map(|m| ret.len() >= m).unwrap_or(false) {
          break;
        }
      }
      None => match vps.last() {
        Some(l) => {
          vpq = vpqf(Some(l.id))?;
          vps = itr.get_vertex_properties(vpq)?;
        }
        // no more records to search.
        None => break,
      },
    }
  }
  Ok(ret)
}
