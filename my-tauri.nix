{ lib
, stdenv
, rustPlatform
, fetchFromGitHub
, pkg-config
, glibc
, libsoup
, cairo
, gtk3
, webkitgtk
, darwin
}:

let
  inherit (darwin.apple_sdk.frameworks) CoreServices Security;
in
rustPlatform.buildRustPackage rec {
  pname = "tauri";
  version = "2.0.0-alpha.10";

  src = fetchFromGitHub {
    owner = "tauri-apps";
    repo = pname;
    rev = "tauri-v${version}";
    sha256 = "sha256-WOxl+hgzKmKXQryD5tH7SJ9YvZb9QA4R+YUYnarnhKA=";
  };

  # Manually specify the sourceRoot since this crate depends on other crates in the workspace. Relevant info at
  # https://discourse.nixos.org/t/difficulty-using-buildrustpackage-with-a-src-containing-multiple-cargo-workspaces/10202
  sourceRoot = "source/tooling/cli";

  cargoHash = "sha256-yZ6PSonM6vSfMfi6FgjDW592lN3fh4kfFyx3toZjseo=";

  buildInputs = lib.optionals stdenv.isLinux [ glibc libsoup cairo gtk3 webkitgtk ]
    ++ lib.optionals stdenv.isDarwin [ CoreServices Security ];
  nativeBuildInputs = [ pkg-config ];

  meta = with lib; {
    description = "Build smaller, faster, and more secure desktop applications with a web frontend";
    homepage = "https://tauri.app/";
    license = with licenses; [ asl20 /* or */ mit ];
    maintainers = with maintainers; [ dit7ya ];
  };
}
