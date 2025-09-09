use std::io::Cursor;

use clap::Arg;
use futures_lite::stream::StreamExt;
use lapin::{
  options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
  types::FieldTable,
  Connection, ConnectionProperties,
};
use tokio::main;
use tracing::{error, info};
use uuid::Uuid;
use zkprotocol::content::{OnMakeFileNote, OnSavedZkNote, ZkNoteId};

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

  tokio::spawn(async move {
    while let Some(rdelivery) = consumer.next().await {
      let delivery = rdelivery.expect("error");
      match serde_json::from_slice::<OnSavedZkNote>(&delivery.data) {
        Ok(szn) => {
          println!("savedskznote: {:?}", szn);
        }
        Err(e) => {
          println!("error: {:?}", e);
        }
      }
      // yeet processing.

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
    println!("on_make_file_note: rdeliver: {:?}", rdelivery);
    let delivery = rdelivery.expect("error");
    match serde_json::from_slice::<OnMakeFileNote>(&delivery.data) {
      Ok(omfn) => {
        println!("on_make_file_note: OnMakeFileNote: {:?}", omfn);
        if let Some(suffix) = omfn.title.split('.').last() {
          match suffix.to_lowercase().as_str() {
            "mp4" => resize_video(server_uri.as_str(), omfn).await?,
            "webm" => resize_video(server_uri.as_str(), omfn).await?,
            "mkv" => resize_video(server_uri.as_str(), omfn).await?,
            "jpg" => resize_image(server_uri.as_str(), omfn).await?,
            "gif" => resize_image(server_uri.as_str(), omfn).await?,
            "png" => resize_image(server_uri.as_str(), omfn).await?,
            // "mp3" ->
            // "m4a" ->
            // "opus" ->
            _ => {
              println!("unsupported file suffix: {}", suffix);
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
      format!("id={}", uuid.to_string().as_str()),
    )
    .send()
    .await?;

  // let res = reqwest::(s).await?;
  let mut file = std::fs::File::create(idstring.as_str())?;
  let mut content = Cursor::new(res.bytes().await?);
  std::io::copy(&mut content, &mut file)?;

  // call out to ffmpeg to resize.

  Ok(())
}

async fn resize_image(
  server_uri: &str,
  omfn: OnMakeFileNote,
) -> Result<(), Box<dyn std::error::Error>> {
  println!("resize_image {:?}", omfn);
  // download the file.
  // download the file.i
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
      format!("id={}", uuid.to_string().as_str()),
    )
    .send()
    .await?;

  let mut file = std::fs::File::create(idstring.as_str())?;
  let mut content = Cursor::new(res.bytes().await?);
  std::io::copy(&mut content, &mut file)?;

  Ok(())

  // run imagemagick to resize.

  // upload resized with thumb prefix.
}
