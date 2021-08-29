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
    rev = "a3702c442a71912067ba6fbf423b6ff126ebb3db";
    sha256 = "1a1cj70576wax2drflksafah66fgjz5xm6cg2f7ycp2q2z8ppjny";
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
  cargoSha256 = "01wmb4rxahhx767msb0nzvwqk3shkr28b6zr3d1mgv6zjj3qhrd8";
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

