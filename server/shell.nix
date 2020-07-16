let 
  nixpkgs = import <nixpkgs> {};
in
  with nixpkgs;
  stdenv.mkDerivation {
    name = "pdfq-env";
    buildInputs = [ 
      cargo
      rustc
      sqlite
      pkgconfig
      openssl.dev 
      nix
      ];
    OPENSSL_DEV=openssl.dev;
  }
