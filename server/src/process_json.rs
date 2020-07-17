extern crate serde_json;
use base64;
use serde_json::Value;
use simple_error;
use sqldata;
use sqldata::PdfInfo;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct PublicMessage {
  what: String,
  data: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct Message {
  pub uid: String,
  pwd: String,
  what: String,
  data: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerResponse {
  pub what: String,
  pub content: Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct PdfNotes {
  pdf_name: String,
  notes: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct PersistentState {
  pdf_name: String,
  last_read: i64,
}

#[derive(Deserialize, Serialize, Debug)]
struct SavePdf {
  pdf_name: String,
  pdf_string: String, // base64
}

#[derive(Deserialize, Serialize, Debug)]
struct GetPdf {
  pdf_name: String,
  pdf_url: String,
}

#[derive(Serialize, Debug)]
pub struct PdfList {
  pdfs: Vec<PdfInfo>,
}

// public json msgs don't require login.
pub fn public_interface(
  pdfdir: &str,
  pdfdb: &str,
  _ip: &Option<&str>,
  msg: PublicMessage,
) -> Result<Option<ServerResponse>, Box<dyn Error>> {
  let pdbp = Path::new(&pdfdb);
  match msg.what.as_str() {
    /*
    "getfilelist" => {
      // get db record info for all pdfs in the pdf dir.

      // pdfs in the db.
      let sqlpdfs = sqldata::pdflist(pdbp)?;
      // pdfs in the dir.
      let filepdfs = sqldata::pdfscan(&pdfdir)?;

      // write records for unknown pdfs into the db, and return a list of
      // records for only the pdfs that are in the dir.
      let refpdfs = sqldata::pdfupret(pdbp, filepdfs, sqlpdfs)?;

      Ok(Some(ServerResponse {
        what: "filelist".to_string(),
        content: serde_json::to_value(PdfList { pdfs: refpdfs })?,
      }))
    }
    "savepdfstate" => {
      // save the pdf viewer state.
      let json = msg
        .data
        .ok_or(simple_error::SimpleError::new("pdfstate data not found!"))?;
      let ps: PersistentState = serde_json::from_value(json.clone())?;
      sqldata::save_pdf_state(
        pdbp,
        ps.pdf_name.as_str(),
        json.to_string().as_str(),
        ps.last_read,
      )?;
      Ok(Some(ServerResponse {
        what: "pdfstatesaved".to_string(),
        content: serde_json::Value::Null,
      }))
    }
    "savepdf" => {
      println!("savepdf");
      // save the pdf to a file.
      let json = msg
        .data
        .ok_or(simple_error::SimpleError::new("pdf data not found!"))?;
      let ps: SavePdf = serde_json::from_value(json.clone())?;
      let bytes = base64::decode(ps.pdf_string.as_str())?;
      let path = &Path::new(pdfdir).join(ps.pdf_name.as_str());
      let mut inf = File::create(path)?;
      inf.write(&bytes)?;
      println!("saved PDF: {}", ps.pdf_name);

      let pi = sqldata::addpdfentry(pdbp, ps.pdf_name.as_str())?;

      Ok(Some(ServerResponse {
        what: "pdfsaved".to_string(),
        content: serde_json::to_value(pi)?,
      }))
    }
    "getpdf" => {
      println!("getpdf");
      // save the pdf to a file.
      let json = msg
        .data
        .ok_or(simple_error::SimpleError::new("getpdf data not found!"))?;
      let gp: GetPdf = serde_json::from_value(json.clone())?;
      {
        let mut res = reqwest::get(gp.pdf_url.as_str())?;
        let path = &Path::new(pdfdir).join(gp.pdf_name.as_str());
        let mut inf = File::create(path)?;
        res.copy_to(&mut inf)?;
      }

      let pi = sqldata::addpdfentry(pdbp, gp.pdf_name.as_str())?;

      println!("saved url PDF: {}", pi.filename);

      Ok(Some(ServerResponse {
        what: "pdfgotten".to_string(),
        content: serde_json::to_value(pi)?,
      }))
    }
    "getnotes" => {
      let json = msg
        .data
        .ok_or(simple_error::SimpleError::new("getnotes data not found!"))?;
      let pdfname: String = serde_json::from_value(json.clone())?;
      let notes = sqldata::get_pdf_notes(pdbp, pdfname.as_str())?;
      let data = serde_json::to_value(PdfNotes {
        pdf_name: pdfname,
        notes: notes,
      })?;

      Ok(Some(ServerResponse {
        what: "notesresponse".to_string(),
        content: data,
      }))
    }
    "savenotes" => {
      let json = msg
        .data
        .ok_or(simple_error::SimpleError::new("savenotes data not found!"))?;
      let pdfnotes: PdfNotes = serde_json::from_value(json.clone())?;
      sqldata::save_pdf_notes(pdbp, pdfnotes.pdf_name.as_str(), pdfnotes.notes.as_str())?;
      Ok(Some(ServerResponse {
        what: "notesaved".to_string(),
        content: serde_json::Value::Null,
      }))
    }
    // get app state.
    "getlaststate" => {
      let nullstate = || {
        Some(ServerResponse {
          what: "laststate".to_string(),
          content: serde_json::Value::Null,
        })
      };
      sqldata::last_ui_state(pdbp)
        .map(|opss| {
          opss
            .and_then(|statestring| {
              println!("statestring: {}", statestring);
              match serde_json::from_str(statestring.as_str()) {
                Ok(v) => {
                  println!("json success {}", v);
                  Some(ServerResponse {
                    what: "laststate".to_string(),
                    content: v,
                  })
                }
                Err(e) => {
                  println!("json fail {}", e);
                  nullstate()
                }
              }
            })
            .or(nullstate())
        })
        .or_else(|e| {
          println!("not found {}", e);
          Ok(nullstate())
        })
    }
    // save app state.
    "savelaststate" => {
      msg.data.map_or(Ok(()), |json| {
        sqldata::save_ui_state(pdbp, json.to_string().as_str())
      })?;
      Ok(Some(ServerResponse {
        what: "laststatesaved".to_string(),
        content: serde_json::Value::Null,
      }))
    } */
    // error for unsupported whats
    wat => bail!(format!("invalid 'what' code:'{}'", wat)),
  }
}
