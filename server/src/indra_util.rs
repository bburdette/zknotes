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
