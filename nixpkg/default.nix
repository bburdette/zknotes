{ stdenv
, fetchFromGitHub
, rustPlatform
, openssl
, pkgconfig
, sqlite
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
    rev = "464d7c8214b2c62fffbac447c5ae1aa3e1e64b8c";
    sha256 = "1c127n3shn14293fy45pamb8zx92q4cplgb8i7f99sfampn8kwhx";
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
  cargoSha256 = "1qrifqgaymmp8l7rg18nqi2rk10sr52d4m0pclwbc6fnnalvk3kf";
  # dontMakeSourcesWritable=1;

  buildInputs = [openssl sqlite];

  nativeBuildInputs = [ pkgconfig ];

  meta = with stdenv.lib; {
    description = "zknotes zettelkasten server.";
    homepage = https://github.com/bburdette/zknotes;
    license = with licenses; [ gpl ];
    maintainers = [ ];
    platforms = platforms.all;
  };
}

