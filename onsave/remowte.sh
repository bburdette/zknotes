
RUST_LOG=debug ../target/debug/zknotes-onsave \
  --amqp_uri "amqps://zknotes.com:5671" \
  --server_uri "https://zknotes.com" \
  --yt-dlp-path "yt-dlp" \
  --yeet-service "true" \
  --thumb-service "false" \
  --amqp-uid-file "../../../yeetwds/uid" \
  --amqp-pwd-file "../../../yeetwds/pwd"
