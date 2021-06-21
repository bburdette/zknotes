{ stdenv
, fetchFromGitHub
, rustPlatform
, openssl
, pkgconfig
, sqlite
, lib
, callPackage }:

# , lib
# , packr

rustPlatform.buildRustPackage rec {
  pname = "zknotes-server";
  version = "1.0";

  # ui = callPackage ./ui.nix { };

  src = fetchFromGitHub {
    owner = "bburdette";
    repo = "zknotes";
    rev = "e490934e128cf4a41f087cf7fd8f7a59efe9c01c";
    sha256 = "1cn89iqvbmcjgzbkqigbpx97skds4w7h2y6ib0jbc31c194baynp";
  };

  # preBuild = ''
  #   cp -r ${ui}/libexec/gotify-ui/deps/gotify-ui/build ui/build && packr
  # '';

  # postInstall = ''
  #   echo "postInttall"
  #   ls -l $out
  #   cp -r ${ui}/static $out
  # '';

  # cargo-culting this from the gotify package.
  subPackages = [ "." ];


  sourceRoot = "source/server";
  cargoSha256 = "1krgkkn5550gdyylylljw5zzfmszmcnpcf9k1whg1ylk27bm9q2r";
  # dontMakeSourcesWritable=1;

  buildInputs = [openssl sqlite];

  nativeBuildInputs = [ pkgconfig ];

  meta = with lib; {
    description = "zknotes zettelkasten server.";
    homepage = https://github.com/bburdette/zknotes;
    license = licenses.gpl3;
    maintainers = [ ];
    platforms = platforms.all;
  };
}

