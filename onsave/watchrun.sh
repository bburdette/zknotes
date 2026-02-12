cargo watch -c \
  -w src \
  -w ../orgauth/rustlib/src/ \
  -w ../zkprotocol/src/ \
  -w Cargo.toml \
  -w ../zkprotocol/Cargo.toml \
  -x build -s ./run.sh
