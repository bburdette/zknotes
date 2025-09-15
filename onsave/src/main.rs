use std::{collections::BTreeMap, fmt, io::Cursor, path::Path, process::Command};

use clap::Arg;
use futures_lite::stream::StreamExt;
use lapin::{
  options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
  types::FieldTable,
  Connection, ConnectionProperties,
};
use reqwest::{
  header::{HeaderMap, HeaderValue, CONTENT_DISPOSITION},
  multipart,
};
use tl::ParserOptions;
use tokio::fs::File;
use tokio_util::codec::BytesCodec;
use tracing::{error, info};
use uuid::Uuid;
use zkprotocol::{
  content::{
    OnMakeFileNote, OnSavedZkNote, SaveZkLink, SaveZkLink2, SaveZkLinks, SaveZkNote,
    SaveZkNoteAndLinks,
  },
  private::{PrivateReply, PrivateRequest},
  upload::UploadReply,
};

// #[tokio::main(flavor = "multi_thread")]
#[tokio::main]
async fn main() {
  match err_main().await {
    Ok(_) => (),
    Err(e) => {
      println!("error: {:?}", e);
    }
  };
}

async fn err_main() -> Result<(), Box<dyn std::error::Error>> {
  println!("welcome to err main");

  let matches = clap::Command::new("zknotes onsave")
    .version("1.0")
    .author("Ben Burdette")
    .about("process notes and file post-save")
    .arg(
      Arg::new("amqp_uri")
        .short('a')
        .long("amqp_uri")
        .value_name("uri of amqp server")
        .help("example: 'amqp://localhost:5672'"),
    )
    .arg(
      Arg::new("server_uri")
        .short('r')
        .long("server_uri")
        .value_name("zknotes server uri")
        .help("server to connect to"),
    )
    .get_matches();

  let amqp_uri = matches
    .get_one::<String>("amqp_uri")
    .expect("amqp_uri is required");

  let server_uri = matches
    .get_one::<String>("server_uri")
    .expect("server_uri is required");

  let conn = Connection::connect(&amqp_uri, ConnectionProperties::default()).await?;

  info!("CONNECTED");

  let chan = conn.create_channel().await?;

  chan
    .queue_declare(
      "on_save_zknote",
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await?;

  let mut consumer = chan
    .basic_consume(
      "on_save_zknote",
      "onsave",
      BasicConsumeOptions::default(),
      FieldTable::default(),
    )
    .await?;

  let onsave_server_uri = server_uri.clone();

  tokio::spawn(async move {
    let client = reqwest::Client::builder()
      .build()
      .expect("error building reqwest client");
    let private_uri = String::from(onsave_server_uri.clone()) + "/private";
    while let Some(rdelivery) = consumer.next().await {
      let delivery = rdelivery.expect("error");
      // println!("onsaved dilvery.data: {:?}", delivery.data);
      match serde_json::from_slice::<OnSavedZkNote>(&delivery.data) {
        Ok(szn) => {
          // let res : Result<(), Box<dyn std::err
          println!("savedskznote: {:?}", szn);
          // yeet processing.
          // retrieve the note.

          let rq = zkprotocol::private::PrivateRequest::PvqGetZkNote(szn.id);

          let rs = serde_json::to_string(&rq).expect("serde error");

          let res = client
            .post(&private_uri)
            .header(
              reqwest::header::COOKIE,
              format!("id={}", szn.token.as_str()),
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(rs)
            .send()
            .await;

          match res {
            Ok(r) => {
              match (
                r.status().is_success(),
                r.text()
                  .await
                  .map_err(|e| format!("error: {e:?}"))
                  .and_then(|t| {
                    serde_json::from_str::<zkprotocol::private::PrivateReply>(t.as_str())
                      .map_err(|e| format!("error: {e:?}"))
                  }),
              ) {
                (true, Ok(m)) => {
                  match m {
                    zkprotocol::private::PrivateReply::PvyZkNote(zkn) => {
                      // search for yeet text.  have to make an intermediate yeet list because Node is not Send.
                      struct MahYeet {
                        url: String,
                        attribs: BTreeMap<String, String>,
                        raw: String,
                      }

                      let yeets = match tl::parse(zkn.content.as_str(), ParserOptions::default()) {
                        Ok(vdom) => {
                          let yeets: Vec<MahYeet> = vdom
                            .nodes()
                            .iter()
                            .filter_map(|n| match n {
                              tl::Node::Tag(t) => {
                                if t.name().as_utf8_str() == "yeet" {
                                  if let Some(Some(_)) = t.attributes().get("id") {
                                    None // don't process yeets that already have an id.
                                  } else if let Some(Some(u)) = t.attributes().get("url") {
                                    Some(MahYeet {
                                      url: u.as_utf8_str().to_string(),
                                      attribs: t.attributes().iter().fold(
                                        BTreeMap::new(),
                                        |mut acc: BTreeMap<String, String>, (name, mbvalue)| {
                                          if let Some(v) = mbvalue {
                                            acc.insert(name.to_string(), v.to_string());
                                            acc
                                          } else {
                                            acc
                                          }
                                        },
                                      ),
                                      raw: t.raw().as_utf8_str().to_string(),
                                    })
                                  } else {
                                    None // if no url, can't process.
                                  }
                                } else {
                                  None
                                }
                              }
                              _ => None,
                            })
                            .collect();
                          yeets
                        }
                        Err(e) => {
                          println!("html parse error: {:?}", e);
                          Vec::new()
                        }
                      };

                      let p = Path::new(".");

                      let mut ed_content = zkn.content.clone();

                      for mut my in yeets {
                        match yeet(&p, my.url.clone()).map_err(|e| format!("{e:?}")) {
                          Err(e) => {
                            println!("error {e:?}");
                          }
                          Ok(f) => {
                            println!("got fileeeee {f:?}");

                            // upload the file to zknotes.
                            if let Ok(file) = File::open(f.clone()).await {
                              let blah: Result<(), Box<dyn std::error::Error>> = async {
                                // upload the file to zknotes.
                                let bytes_stream =
                                  tokio_util::codec::FramedRead::new(file, BytesCodec::new());
                                let form = reqwest::multipart::Form::new().part(
                                  f.clone().replace(" ", "_"), // no spaces allowed.
                                  multipart::Part::stream(reqwest::Body::wrap_stream(bytes_stream))
                                    .file_name(f),
                                );
                                let res = client
                                  .post(String::from(onsave_server_uri.clone()) + "/upload")
                                  .multipart(form)
                                  .header(
                                    reqwest::header::COOKIE,
                                    format!("id={}", szn.token.as_str()),
                                  )
                                  .send()
                                  .await?;
                                println!("upload result: {res:?}");

                                // if the result was good, add the id to the yeet tag, and search-and-replace.
                                match (
                                  res.status().is_success(),
                                  res
                                    .text()
                                    .await
                                    .map_err(|e| format!("error: {e:?}"))
                                    .and_then(|t| {
                                      println!("t : {}", t);
                                      serde_json::from_str::<zkprotocol::upload::UploadReply>(
                                        t.as_str(),
                                      )
                                      .map_err(|e| format!("error: {e:?}"))
                                    }),
                                ) {
                                  (true, Ok(pr)) => match pr {
                                    UploadReply::UrFilesUploaded(notes) => {
                                      let id = notes
                                        .first()
                                        .ok_or(StringError {
                                          s: "no note uploaded".to_string(),
                                        })?
                                        .id;
                                      my.attribs.insert("id".to_string(), format!("{}", id));
                                      let newyeet = format!("<yeet ")
                                        + my
                                          .attribs
                                          .into_iter()
                                          .map(|(n, v)| format!(" {}=\"{}\"", n, v))
                                          .collect::<Vec<String>>()
                                          .concat()
                                          .as_str()
                                        + "/>";

                                      ed_content =
                                        ed_content.replace(my.raw.as_str(), newyeet.as_str());

                                      println!("ed_content {}", ed_content);
                                    }
                                  },
                                  x => {
                                    println!("upload failure: {x:?}");
                                  }
                                }
                                Ok(())
                              }
                              .await;

                              match blah {
                                Err(e) => println!("error: {e:?}"),
                                Ok(_) => (),
                              }
                            }
                          }
                        }
                      }

                      // if ed_content has changed, update the note.
                      let blah: Result<(), Box<dyn std::error::Error>> = async {
                        if ed_content != zkn.content {
                          let nszn = SaveZkNote {
                            id: Some(zkn.id),
                            title: zkn.title,
                            pubid: zkn.pubid,
                            content: ed_content,
                            editable: zkn.editable,
                            showtitle: zkn.showtitle,
                            deleted: zkn.deleted,
                            what: None,
                          };
                          let res = client
                            .post(String::from(onsave_server_uri.clone()) + "/private")
                            .header(
                              reqwest::header::COOKIE,
                              format!("id={}", szn.token.as_str()),
                            )
                            .header(reqwest::header::CONTENT_TYPE, "application/json")
                            .body(
                              serde_json::to_string(&PrivateRequest::PvqSaveZkNote(nszn)).unwrap(),
                            )
                            .send()
                            .await?;

                          if !res.status().is_success() {
                            println!("error updating note: {}, {:?}", zkn.id, res);
                          }
                        }
                        Ok(())
                      }
                      .await;
                      match blah {
                        Err(e) => println!("error: {e:?}"),
                        Ok(_) => (),
                      }
                    }
                    x => {
                      println!("unexpected message: {x:?}");
                    }
                  }
                }
                r => {
                  println!("bad result: {r:?}");
                }
              }
            }
            Err(e) => {
              println!("post error: {:?}", e);
            }
          }
        }
        Err(e) => {
          println!("error: {:?}", e);
        }
      }

      delivery
        .ack(BasicAckOptions::default())
        .await
        .expect("ack error");
    }
  });

  let chan = conn.create_channel().await?;

  chan
    .queue_declare(
      "on_make_file_note",
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await?;

  let mut consumer = chan
    .basic_consume(
      "on_make_file_note",
      "onmakefile",
      BasicConsumeOptions::default(),
      FieldTable::default(),
    )
    .await?;

  while let Some(rdelivery) = consumer.next().await {
    // println!("on_make_file_note: rdeliver: {:?}", rdelivery);
    let delivery = rdelivery.expect("error");
    match serde_json::from_slice::<OnMakeFileNote>(&delivery.data) {
      Ok(omfn) => {
        println!("on_make_file_note: OnMakeFileNote: {:?}", omfn);
        if let Some(suffix) = omfn.title.split('.').last() {
          if !omfn.title.contains("thumb") {
            match suffix.to_lowercase().as_str() {
              "mp4" => resize_video(&server_uri.as_str(), omfn).await?,
              "webm" => resize_video(&server_uri.as_str(), omfn).await?,
              "mkv" => resize_video(&server_uri.as_str(), omfn).await?,
              "jpg" => resize_image(&server_uri.as_str(), omfn).await?,
              "gif" => resize_image(&server_uri.as_str(), omfn).await?,
              "png" => resize_image(&server_uri.as_str(), omfn).await?,
              // "mp3" ->
              // "m4a" ->
              // "opus" ->
              _ => {
                println!("unsupported file suffix: {}", suffix);
              }
            }
          }
        }
      }
      Err(e) => {
        println!("error: {:?}", e);
      }
    }

    println!("pre ack");

    delivery
      .ack(BasicAckOptions::default())
      .await
      .expect("ack error");
  }

  println!("Goodbye, world!");

  Ok(())
}

#[derive(Debug)]
struct StringError {
  s: String,
}

impl fmt::Display for StringError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.s)
  }
}

impl std::error::Error for StringError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
  }
}

pub fn yeet(savedir: &Path, url: String) -> Result<String, Box<dyn std::error::Error>> {
  // parse 'url'
  let uri: reqwest::Url = match url.parse() {
    Ok(uri) => uri,
    Err(e) => {
      return Err(Box::new(StringError {
        s: format!("yeet err {:?}", e),
      }))
    }
  };

  // get 'v' parameter.
  let qps = uri.query_pairs();
  let mut vid: Option<String> = None;
  for (name, value) in qps {
    if name == "v" {
      vid = Some(value.to_string());
    }
  }

  let v = vid.ok_or(StringError {
    s: "missing 'v' from url".to_string(),
  })?;

  let mut child = Command::new("yt-dlp")
    .arg("-x")
    .arg(format!("-o{}/%(title)s-%(id)s.%(ext)s", savedir.display()))
    .arg(url.clone())
    .spawn()
    .expect("yt-dlp failed to execute");

  match child.wait() {
    Ok(exit_code) => {
      if exit_code.success() {
        // find the yeeted file by 'v'.
        let file: std::path::PathBuf =
          match glob::glob(format!("{}/*{}*", savedir.display(), v).as_str()) {
            Ok(mut paths) => match paths.next() {
              Some(rpb) => match rpb {
                Ok(pb) => pb,
                Err(e) => {
                  return Err(Box::new(StringError {
                    s: format!("glob error {:?}", e),
                  }))
                }
              },
              None => {
                return Err(Box::new(StringError {
                  s: format!("yeet file not found {:?}", v),
                }))
              }
            },
            Err(e) => {
              return Err(Box::new(StringError {
                s: format!("glob error {:?}", e),
              }))
            }
          };
        let filename = file
          .as_path()
          .file_name()
          .and_then(|x| x.to_str())
          .unwrap_or("meh.txt")
          .to_string();

        Ok(filename)
      } else {
        Err(Box::new(StringError {
          s: format!("yeet err {:?}", exit_code),
        }))
      }
    }
    Err(e) => Err(Box::new(StringError {
      s: format!("yeet err {:?}", e),
    })),
  }
}

async fn resize_video(
  server_uri: &str,
  omfn: OnMakeFileNote,
) -> Result<(), Box<dyn std::error::Error>> {
  println!("resize_video {:?}", omfn);
  // download the file.
  let mut s = String::from(server_uri);
  s.push_str("/file/");
  let uuid: Uuid = omfn.id.into();
  let idstring = uuid.to_string();
  s.push_str(idstring.as_str());

  let client = reqwest::Client::builder().build()?;
  let res = client
    .get(s)
    .header(
      reqwest::header::COOKIE,
      format!("id={}", omfn.token.as_str()),
    )
    .send()
    .await?;

  // let res = reqwest::(s).await?;
  let mut file = std::fs::File::create(idstring.as_str())?;
  let mut content = Cursor::new(res.bytes().await?);
  std::io::copy(&mut content, &mut file)?;

  // call out to ffmpeg to resize.
  // `ffmpeg -i newphonepix/Camera/VID_20250730_191257.mp4 -x264-params keyint=240:bframes=6:ref=4:me=umh:subme=9:no-fast-pskip=1:b-adapt=2:aq-mode=2 alamo.mp4`

  let thumbfile = video_resize(idstring).await?;

  // upload resized thumb
  let zkprotocol::upload::UploadReply::UrFilesUploaded(uploadreply) = upload_file(
    &client,
    Path::new(thumbfile.as_str()),
    &omfn.token,
    server_uri,
  )
  .await?;

  println!("made it here");
  // ---------- link thumb to original. -------------

  // for each zklistnote make a link.  should be only one.
  let szl = SaveZkLinks {
    links: uploadreply
      .into_iter()
      .map(|zln| SaveZkLink2 {
        from: zln.id,
        to: omfn.id,
        linkzknote: None,
        delete: None,
      })
      .collect(),
  };

  let client = reqwest::Client::builder().build()?;
  let res = client
    .post(String::from(server_uri) + "/private")
    .header(
      reqwest::header::COOKIE,
      format!("id={}", omfn.token.as_str()),
    )
    .header(reqwest::header::CONTENT_TYPE, "application/json")
    .body(serde_json::to_string(&PrivateRequest::PvqSaveZkLinks(szl))?)
    .send()
    .await?;

  let txt = res.text().await?;

  println!("savezklinkes reply: {txt}");

  let reply = serde_json::from_str::<zkprotocol::private::PrivateReply>(txt.as_str())?;

  match reply {
    PrivateReply::PvySavedZkLinks => Ok(()),
    x => Err(Box::new(StringError {
      s: format!("unexpected reply to savezklinks: {:?}", x),
    })),
  }
}

async fn resize_image(
  server_uri: &str,
  omfn: OnMakeFileNote,
) -> Result<(), Box<dyn std::error::Error>> {
  println!("resize_image {:?}", omfn);
  // download the file.
  // download the file.
  let mut s = String::from(server_uri);
  s.push_str("/file/");
  let uuid: Uuid = omfn.id.into();
  let idstring = uuid.to_string();
  s.push_str(idstring.as_str());

  let client = reqwest::Client::builder().build()?;
  let res = client
    .get(s)
    .header(
      reqwest::header::COOKIE,
      format!("id={}", omfn.token.as_str()),
    )
    .send()
    .await?;

  let mut file = std::fs::File::create(idstring.as_str())?;
  let mut content = Cursor::new(res.bytes().await?);
  std::io::copy(&mut content, &mut file)?;

  // run imagemagick to resize.
  // resize to area of 400x400, but retaining aspect ratio:
  // `magick IMG_20230121_101742.jpg -resize 400x400^ out2.jpg`
  let thumbfile = image_resize(idstring).await?;

  // upload resized thumb
  let zkprotocol::upload::UploadReply::UrFilesUploaded(uploadreply) = upload_file(
    &client,
    Path::new(thumbfile.as_str()),
    &omfn.token,
    server_uri,
  )
  .await?;

  println!("made it here");
  // ---------- link thumb to original. -------------

  // for each zklistnote make a link.  should be only one.
  let szl = SaveZkLinks {
    links: uploadreply
      .into_iter()
      .map(|zln| SaveZkLink2 {
        from: zln.id,
        to: omfn.id,
        linkzknote: None,
        delete: None,
      })
      .collect(),
  };

  let client = reqwest::Client::builder().build()?;
  let res = client
    .post(String::from(server_uri) + "/private")
    .header(
      reqwest::header::COOKIE,
      format!("id={}", omfn.token.as_str()),
    )
    .header(reqwest::header::CONTENT_TYPE, "application/json")
    .body(serde_json::to_string(&PrivateRequest::PvqSaveZkLinks(szl))?)
    .send()
    .await?;

  let txt = res.text().await?;

  println!("savezklinkes reply: {txt}");

  let reply = serde_json::from_str::<zkprotocol::private::PrivateReply>(txt.as_str())?;

  match reply {
    PrivateReply::PvySavedZkLinks => Ok(()),
    x => Err(Box::new(StringError {
      s: format!("unexpected reply to savezklinks: {:?}", x),
    })),
  }

  // TODO: delete file.
}

pub async fn image_resize(
  imagefile: String,
  // sizeparm: String,
) -> Result<String, Box<dyn std::error::Error>> {
  let outfile = imagefile.clone() + "-thumb.jpg";

  let mut child = Command::new("magick")
    .arg(imagefile)
    .arg("-resize")
    .arg("400x400^")
    .arg(outfile.clone())
    .spawn()
    .expect("magick failed to execute");

  match child.wait() {
    Ok(exit_code) => {
      if exit_code.success() {
        Ok(outfile)
      } else {
        Err(Box::new(StringError {
          s: format!("magick resize error {:?}", exit_code),
        }))
      }
    }
    Err(e) => Err(Box::new(StringError {
      s: format!("magick resize error {:?}", e),
    })),
  }
}

pub async fn video_resize(
  videofile: String,
  // sizeparm: String,
) -> Result<String, Box<dyn std::error::Error>> {
  let outfile = videofile.clone() + "-thumb.mp4";

  let mut child = Command::new("ffmpeg")
    .arg("-i")
    .arg(videofile)
    .arg("-x264-params")
    .arg("keyint=240:bframes=6:ref=4:me=umh:subme=9:no-fast-pskip=1:b-adapt=2:aq-mode=2")
    .arg(outfile.clone())
    .spawn()
    .expect("magick failed to execute");

  match child.wait() {
    Ok(exit_code) => {
      if exit_code.success() {
        Ok(outfile)
      } else {
        Err(Box::new(StringError {
          s: format!("ffmpeg resize error {:?}", exit_code),
        }))
      }
    }
    Err(e) => Err(Box::new(StringError {
      s: format!("ffmpeg resize error {:?}", e),
    })),
  }
}

pub async fn upload_file(
  client: &reqwest::Client,
  filename: &Path,
  token: &str,
  server_uri: &str,
) -> Result<zkprotocol::upload::UploadReply, Box<dyn std::error::Error>> {
  let file = File::open(filename).await?;

  // upload the file to zknotes.
  let bytes_stream = tokio_util::codec::FramedRead::new(file, BytesCodec::new());
  let utf_fname = filename
    .to_str()
    .ok_or(StringError {
      // s: format!("filename unicode error: {:?}", filename.clone()),
      s: "filename unicode error".to_string(),
    })?
    .to_string();
  let form = reqwest::multipart::Form::new().part(
    utf_fname.to_string().replace(" ", "_"), // no spaces allowed.
    multipart::Part::stream(reqwest::Body::wrap_stream(bytes_stream)).file_name(utf_fname),
  );
  let res = client
    .post(String::from(server_uri) + "/upload")
    .multipart(form)
    .header(reqwest::header::COOKIE, format!("id={}", token))
    .send()
    .await?;
  println!("upload result: {res:?}");

  if !res.status().is_success() {
    return Err(Box::new(StringError {
      s: format!("upload failure: {}", res.status()),
    }));
  }

  let txt = res.text().await?;

  let ur = serde_json::from_str::<zkprotocol::upload::UploadReply>(txt.as_str())?;

  Ok(ur)
}
