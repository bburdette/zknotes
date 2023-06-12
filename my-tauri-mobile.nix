{ lib
, stdenv
, rustPlatform
, fetchFromGitHub
, pkg-config
, openssl
, git
, darwin
, makeWrapper
}:

let
  inherit (darwin.apple_sdk.frameworks) CoreServices;
  pname = "tauri-mobile";
  version = "tauri-mobile-v0.5.1";
in
rustPlatform.buildRustPackage {
  inherit pname version;
  src = fetchFromGitHub {
    owner = "bburdette";
    repo = pname;
    rev = "f011d5e53898f36ea64ed2e5ca56035b951a899c";
    sha256 = "sha256-DdOHos7LOMobjzPcwC8F36QeoDfo5ejmfmofAjMEC/k=";
  };

  # Manually specify the sourceRoot since this crate depends on other crates in the workspace. Relevant info at
  # https://discourse.nixos.org/t/difficulty-using-buildrustpackage-with-a-src-containing-multiple-cargo-workspaces/10202
  # sourceRoot = "source/tooling/cli";

  cargoHash = "sha256-mIya7CLhYdWS+K/vLylm1x+1bkYRGtxdY3t1iwjU6oU=";

  preBuild = ''
    export HOME=$(mktemp -d)
  '';

  buildInputs = [ openssl ] ++ lib.optionals stdenv.isDarwin [ CoreServices ];
  nativeBuildInputs = [ pkg-config git makeWrapper ];

  preInstall = ''
    mkdir -p $out/share/
    # the directory created in the build process is .tauri-mobile, a hidden directory
    shopt -s dotglob
    for temp_dir in $HOME/*; do
      cp -R $temp_dir $out/share
    done
  '';

  meta = with lib; {
    description = "Rust on mobile made easy! ";
    homepage = "https://tauri.app/";
    license = with licenses; [ asl20 /* or */ mit ];
    maintainers = with maintainers; [ happysalada ];
  };
}
