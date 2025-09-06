use clap::Arg;
use futures_lite::stream::StreamExt;
use lapin::{
  options::{BasicAckOptions, BasicConsumeOptions, BasicGetOptions, QueueDeclareOptions},
  types::FieldTable,
  Connection, ConnectionProperties,
};
use tokio::main;
use tracing::info;
use zkprotocol::content::SavedZkNote;

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
      Arg::new("server_url")
        .short('r')
        .long("url")
        .value_name("zknotes server url")
        .help("server to connect to"),
    )
    .arg(
      Arg::new("user")
        .short('u')
        .long("user")
        .value_name("user name")
        .help("name of user to search with"),
    )
    .arg(
      Arg::new("password")
        .short('p')
        .long("password")
        .value_name("password")
        .help("password"),
    )
    .arg(
      Arg::new("cookie")
        .short('k')
        .long("cookie")
        .value_name("cookie"),
    )
    .arg(
      Arg::new("search")
        .short('s')
        .long("search")
        .value_name("search expression"),
    )
    .arg(
      Arg::new("search_result_format")
        .short('f')
        .long("search_format")
        .value_name("search result format")
        .help("RtId, RtListNote, RtNote, RtNoteAndLinks"),
    )
    .get_matches();

  let uri = matches
    .get_one::<String>("amqp_uri")
    .expect("amqp_uri is required");

  let conn = Connection::connect(&uri, ConnectionProperties::default()).await?;

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
      match serde_json::from_slice::<SavedZkNote>(&delivery.data) {
        Ok(szn) => {
          println!("savedskznote: {:?}", szn);
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
    let delivery = rdelivery.expect("error");
    match serde_json::from_slice::<SavedZkNote>(&delivery.data) {
      Ok(szn) => {
        println!("on_make_file_note: savedskznote: {:?}", szn);
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

  println!("Goodbye, world!");

  Ok(())
}
