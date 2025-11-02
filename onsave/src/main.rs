use std::{collections::BTreeMap, fmt, fs, io::Cursor, path::Path, process::Command};

use clap::Arg;
use futures_lite::stream::StreamExt;
use lapin::{
  Connection, ConnectionProperties,
  options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
  types::FieldTable,
};
use reqwest::multipart;
use tl::ParserOptions;
use tokio::fs::File;
use tokio_util::codec::BytesCodec;
use tracing::{error, info};
use uuid::Uuid;
use zkprotocol::{
  content::{OnMakeFileNote, OnSavedZkNote, SaveZkLink2, SaveZkLinks, SaveZkNote},
  private::{PrivateReply, PrivateRequest},
  upload::UploadReply,
};

// #[tokio::main(flavor = "multi_thread")]
#[tokio::main]
async fn main() {
  match err_main().await {
    Ok(_) => (),
    Err(e) => {
      error!("error: {:?}", e);
    }
  };
}

async fn err_main() -> Result<(), Box<dyn std::error::Error>> {
  env_logger::init();
  info!("zknotes-onsave");

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
    .arg(
      Arg::new("yt-dlp-path")
        // .short('r')
        .long("yt-dlp-path")
        .value_name("path of yt-dlp")
        .help("full path is needed in nixos module/services"),
    )
    .get_matches();

  let amqp_uri = matches
    .get_one::<String>("amqp_uri")
    .expect("amqp_uri is required");

  let server_uri = matches
    .get_one::<String>("server_uri")
    .expect("server_uri is required");

  let yt_dlp_path = matches
    .get_one::<String>("yt-dlp-path")
    .unwrap_or(&"yt-dlp".to_string())
    .clone();

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
      match serde_json::from_slice::<OnSavedZkNote>(&delivery.data) {
        Ok(szn) => {
          // let res : Result<(), Box<dyn std::err
          info!("savedzkznote: {:?}", szn);
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
                          error!("html parse error: {:?}", e);
                          Vec::new()
                        }
                      };

                      let p = Path::new(".");

                      let mut ed_content = zkn.content.clone();

                      for mut my in yeets {
                        match yeet(&p, my.url.clone(), yt_dlp_path.clone())
                          .map_err(|e| format!("{e:?}"))
                        {
                          Err(e) => {
                            error!("yeet error {e:?}");

                            let nszn = SaveZkNote {
                              id: None,
                              title: "yeet error".to_string(),
                              pubid: None,
                              content: format!("{:?}", e),
                              editable: false,
                              showtitle: false,
                              deleted: false,
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
                                serde_json::to_string(&PrivateRequest::PvqSaveZkNote(nszn))
                                  .unwrap(),
                              )
                              .send()
                              .await;

                            match res {
                              Ok(res) => {
                                println!("res: {:?}", res);

                                if !res.status().is_success() {
                                  error!("error saving yeet error note: {}, {:?}", zkn.id, res);
                                }

                                // async block so we can use ? operator.
                                let blah: Result<(), Box<dyn std::error::Error>> =
                                  async {
                                    let txt = res.text().await?;

                                    let pr = serde_json::from_str::<
                                      zkprotocol::private::PrivateReply,
                                    >(txt.as_str())?;

                                    match pr {
                                      zkprotocol::private::PrivateReply::PvySavedZkNote(szn) => {
                                        // add id of the yeet error note to the <yeet>
                                        my.attribs.insert("id".to_string(), format!("{}", szn.id));
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
                                      }
                                      _ => {
                                        error!("unexpected reply: {:?}", pr);
                                      }
                                    };

                                    Ok(())
                                  }
                                  .await;
                                match blah {
                                  Err(e) => {
                                    error!("{e:?}");
                                  }
                                  _ => (),
                                };
                              }
                              Err(e) => error!("{e:?}"),
                            };
                          }
                          Ok(f) => {
                            let blah: Result<(), Box<dyn std::error::Error>> = async {
                              info!("got file {f:?}");
                              let uploadreply = upload_file(
                                &client,
                                Path::new(&f),
                                &szn.token,
                                onsave_server_uri.as_str(),
                              )
                              .await?;
                              match uploadreply {
                                UploadReply::UrFilesUploaded(notes) => {
                                  fs::remove_file(f.clone())?;
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
                                }
                              };
                              Ok(())
                            }
                            .await;
                            match blah {
                              Err(e) => error!("error: {e:?}"),
                              Ok(_) => (),
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
                            error!("error updating note: {}, {:?}", zkn.id, res);
                          }
                        }
                        Ok(())
                      }
                      .await;
                      match blah {
                        Err(e) => error!("error: {e:?}"),
                        Ok(_) => (),
                      }
                    }
                    x => {
                      error!("unexpected message: {x:?}");
                    }
                  }
                }
                r => {
                  error!("bad result: {r:?}");
                }
              }
            }
            Err(e) => {
              error!("post error: {:?}", e);
            }
          }
        }
        Err(e) => {
          error!("error: {:?}", e);
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
    let delivery = rdelivery.expect("error");
    match serde_json::from_slice::<OnMakeFileNote>(&delivery.data) {
      Ok(omfn) => {
        info!("on_make_file_note: OnMakeFileNote: {:?}", omfn);
        if let Some(suffix) = omfn.title.split('.').last() {
          if !omfn.title.contains("thumb") {
            match suffix.to_lowercase().as_str() {
              "mp4" => resize_video(&server_uri.as_str(), omfn).await?,
              "webm" => resize_video(&server_uri.as_str(), omfn).await?,
              "mkv" => resize_video(&server_uri.as_str(), omfn).await?,
              "jpg" => resize_image(&server_uri.as_str(), omfn).await?,
              "gif" => resize_image(&server_uri.as_str(), omfn).await?,
              "png" => resize_image(&server_uri.as_str(), omfn).await?,
              // ignore these.
              "mp3" => (),
              "m4a" => (),
              "opus" => (),
              _ => {
                error!("unsupported file suffix: {}", suffix);
              }
            }
          }
        }
      }
      Err(e) => {
        error!("error: {:?}", e);
      }
    }

    delivery
      .ack(BasicAckOptions::default())
      .await
      .expect("ack error");
  }

  info!("Goodbye, world!");

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

pub fn yeet(
  savedir: &Path,
  url: String,
  yt_dlp_path: String,
) -> Result<String, Box<dyn std::error::Error>> {
  // parse 'url'
  let uri: reqwest::Url = match url.parse() {
    Ok(uri) => uri,
    Err(e) => {
      return Err(Box::new(StringError {
        s: format!("yeet err {:?}", e),
      }));
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

  let mut child = Command::new(yt_dlp_path.as_str())
    .arg("-x")
    .arg(format!("-o{}/%(title)s-%(id)s.%(ext)s", savedir.display()))
    .arg(url.clone())
    .arg("--extractor-args")
    .arg("youtube:player-client=default,-tv_simply")
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
                  }));
                }
              },
              None => {
                return Err(Box::new(StringError {
                  s: format!("yeet file not found {:?}", v),
                }));
              }
            },
            Err(e) => {
              return Err(Box::new(StringError {
                s: format!("glob error {:?}", e),
              }));
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
  info!("resize_video {:?}", omfn);
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

  fs::remove_file(thumbfile)?;

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
  info!("resize_image {:?}", omfn);
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

  fs::remove_file(thumbfile)?;

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
    .arg("800x800^")
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
      s: "filename unicode error".to_string(),
    })?
    .to_string();

  let form = reqwest::multipart::Form::new().part(
    "whatever_name".to_string(),
    multipart::Part::stream(reqwest::Body::wrap_stream(bytes_stream)).file_name(utf_fname),
  );
  let res = client
    .post(String::from(server_uri) + "/upload")
    .multipart(form)
    .header(reqwest::header::COOKIE, format!("id={}", token))
    .send()
    .await?;

  if !res.status().is_success() {
    return Err(Box::new(StringError {
      s: format!("upload failure: {:?}", res),
    }));
  }

  let txt = res.text().await?;

  let ur = serde_json::from_str::<zkprotocol::upload::UploadReply>(txt.as_str())?;

  Ok(ur)
}
