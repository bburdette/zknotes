use clap::Arg;
use lapin::{
  options::{BasicConsumeOptions, BasicGetOptions, QueueDeclareOptions},
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

  // tokio::spawn(async move {
  while let Some(delivery) = chan
    .basic_get("on_save_zknote", BasicGetOptions::default())
    .await
    .expect("error in consumer")
  {
    match serde_json::from_slice::<SavedZkNote>(&delivery.data) {
      Ok(szn) => {
        println!("savedskznote: {:?}", szn);
      }
      Err(e) => {
        println!("error: {:?}", e);
      }
    }
  }
  // });

  let chan = conn.create_channel().await?;

  chan
    .queue_declare(
      "on_make_file_note",
      QueueDeclareOptions::default(),
      FieldTable::default(),
    )
    .await?;

  // tokio::spawn(async move {
  while let Some(delivery) = chan
    .basic_get("on_make_file_note", BasicGetOptions::default())
    .await
    .expect("error in consumer")
  {
    match serde_json::from_slice::<SavedZkNote>(&delivery.data) {
      Ok(szn) => {
        println!("savedskznote: {:?}", szn);
      }
      Err(e) => {
        println!("error: {:?}", e);
      }
    }
  }
  // });

  //   onsave_consumer.or_else(f)
  // while let Some(delivery) =  onsave_consumer.next().await {
  //   delivery.ack(BasicAckOptions::default()).await.expect("ack")
  // } }).detach();

  println!("Hello, world!");

  Ok(())
}
