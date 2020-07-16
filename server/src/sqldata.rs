use rusqlite::{params, Connection};
use serde_json;
use std::collections::{BTreeMap};
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Debug, Clone)]
pub struct PdfInfo {
  pub last_read: Option<i64>,
  pub filename: String,
  pub state: Option<serde_json::Value>,
}

pub fn pdflist(dbfile: &Path) -> rusqlite::Result<Vec<PdfInfo>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare("SELECT name, last_read, persistentState FROM pdfinfo")?;
  let pdfinfo_iter = pstmt.query_map(params![], |row| {
    let ss: Option<String> = row.get(2)?;
    // we don't get the json parse error if there is one!
    let state: Option<serde_json::Value> = ss.and_then(|s| serde_json::from_str(s.as_str()).ok());

    Ok(PdfInfo {
      filename: row.get(0)?,
      last_read: row.get(1)?,
      state: state,
    })
  })?;

  let mut pv = Vec::new();

  for rspdfinfo in pdfinfo_iter {
    match rspdfinfo {
      Ok(pdfinfo) => {
        pv.push(pdfinfo);
      }
      Err(_) => (),
    }
  }

  Ok(pv)
}

pub fn dbinit(dbfile: &Path) -> rusqlite::Result<()> {
  let conn = Connection::open(dbfile)?;

  // create the pdfinfo table.
  println!(
    "pdfinfo create: {:?}",
    conn.execute(
      "CREATE TABLE pdfinfo (
                  name            TEXT NOT NULL PRIMARY KEY,
                  last_read       INTEGER,
                  persistentState BLOB,
                  notes           TEXT NOT NULL
                  )",
      params![],
    )?
  );
  conn.execute(
    "CREATE TABLE uistate (
                id          INTEGER NOT NULL PRIMARY KEY,
                state       TEXT NOT NULL
                )",
    params![],
  )?;
  Ok(())
}

// create entries in the db for pdfs that aren't in there yet.
// then return a list of entries for pdfs that are in the dir.
pub fn pdfupret(
  dbfile: &Path,
  filepdfs: std::vec::Vec<PdfInfo>,
  dbpdfs: std::vec::Vec<PdfInfo>,
) -> rusqlite::Result<std::vec::Vec<PdfInfo>> {
  let conn = Connection::open(dbfile)?;

  let mut dbmap: BTreeMap<String, PdfInfo> = BTreeMap::new();
  for pi in dbpdfs {
    dbmap.insert(pi.filename.clone(), pi);
  }

  let mut out = Vec::new();

  for pi in filepdfs {
    match dbmap.get(&pi.filename) {
      Some(dbpi) => {
        out.push(dbpi.clone());
      }
      None => {
        println!("adding pdf: {}", pi.filename);
        conn.execute(
          "INSERT INTO pdfinfo (name, last_read, persistentState, notes)
                      VALUES (?1, ?2, ?3, '')",
          params![pi.filename, pi.last_read, ""],
        )?;
        out.push(pi.clone());
      }
    }
  }

  Ok(out)
}

// create an entry in the db if the pdf isn't there already.
pub fn addpdfentry(dbfile: &Path, filename: &str) -> Result<PdfInfo, Box<dyn Error>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt =
    conn.prepare("SELECT name, last_read, persistentState FROM pdfinfo WHERE name = ?1")?;

  let mut pdfinfo_iter = pstmt.query_map(params![filename], |row| {
    let ss: Option<String> = row.get(2)?;
    // TODO: we don't get the json parse error if there is one!
    let state: Option<serde_json::Value> = ss.and_then(|s| serde_json::from_str(s.as_str()).ok());

    Ok(PdfInfo {
      filename: row.get(0)?,
      last_read: row.get(1)?,
      state: state,
    })
  })?;

  match pdfinfo_iter.next() {
    Some(pi) => {
      println!("addpdfentry, ret existing: {}", filename);
      pi.map_err(|e| e.into())
    }
    None => {
      println!("addpdfentry, inserting: {}", filename);
      let nowsecs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|n| n.as_secs())?;
      let s: i64 = nowsecs.try_into()?;
      let nowi64secs = s * 1000;

      conn.execute(
        "INSERT INTO pdfinfo (name, last_read, persistentState, notes)
                      VALUES (?1, ?2, ?3, '')",
        params![filename, nowi64secs, ""],
      )?;

      Ok(PdfInfo {
        filename: filename.to_string(),
        last_read: Some(nowi64secs),
        state: None,
      })
    }
  }
}

pub fn save_ui_state(dbfile: &Path, state: &str) -> rusqlite::Result<()> {
  let conn = Connection::open(dbfile)?;

  conn.execute(
    "INSERT INTO uistate(id,state) VALUES(1, ?1)
    ON CONFLICT(id) DO UPDATE SET state=excluded.state",
    params![state],
  )?;

  Ok(())
}

pub fn last_ui_state(dbfile: &Path) -> rusqlite::Result<Option<String>> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare("SELECT state FROM uistate WHERE id = 1")?;
  let mut rows = pstmt.query(params![])?;

  match rows.next() {
    Ok(Some(row)) => row.get(0),
    Ok(None) => Ok(None),
    Err(e) => Err(e),
  }
}

pub fn save_pdf_state(
  dbfile: &Path,
  pdfname: &str,
  pdfstate: &str,
  last_read: i64,
) -> rusqlite::Result<()> {
  let conn = Connection::open(dbfile)?;

  println!("save_pdf_state {} {}", pdfname, pdfstate);

  conn.execute(
    "update pdfinfo set persistentState = ?2, last_read = ?3 where name = ?1",
    params![pdfname, pdfstate, last_read],
  )?;

  Ok(())
}

pub fn get_pdf_notes(dbfile: &Path, pdfname: &str) -> rusqlite::Result<String> {
  let conn = Connection::open(dbfile)?;

  let mut pstmt = conn.prepare("SELECT notes FROM pdfinfo WHERE name = ?1")?;
  let mut rows = pstmt.query(params![pdfname])?;

  match rows.next() {
    Ok(Some(row)) => row.get(0),
    Ok(None) => Err(rusqlite::Error::QueryReturnedNoRows),
    Err(e) => Err(e),
  }
}

pub fn save_pdf_notes(dbfile: &Path, pdfname: &str, pdfnotes: &str) -> rusqlite::Result<()> {
  let conn = Connection::open(dbfile)?;

  conn.execute(
    "update pdfinfo set notes = ?2 where name = ?1",
    params![pdfname, pdfnotes],
  )?;

  Ok(())
}

// scan the pdf dir and return a pdfinfo for each file.
pub fn pdfscan(pdfdir: &str) -> Result<std::vec::Vec<PdfInfo>, Box<dyn Error>> {
  let p = Path::new(pdfdir);

  let mut v = Vec::new();

  if p.exists() {
    for fr in p.read_dir()? {
      let f = fr?;

      v.push(PdfInfo {
        filename: f
          .file_name()
          .into_string()
          .unwrap_or("non utf filename".to_string()),
        last_read: f
          .metadata()
          .and_then(|f| {
            f.accessed().and_then(|t| {
              let dur = t.duration_since(UNIX_EPOCH).expect("unix-epoch-error");
              let meh: i64 = dur
                .as_millis()
                .try_into()
                .expect("i64 insufficient for posix date!");
              Ok(meh)
            })
          })
          .ok(),
        state: None,
      });
    }
  } else {
    error!("pdf directory not found: {}", pdfdir);
  }

  Ok(v)
}
