let 
  nixpkgs = import <nixpkgs> {};
in
  with nixpkgs;
  stdenv.mkDerivation {
    name = "rust-env";
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
