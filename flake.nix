{
  description = "zknotes, a web based zettelkasten";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # elm-language-server = {
    #   url = "github:WhileTruu/elm-language-server";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
  };


  outputs = { self, nixpkgs, flake-utils, naersk, fenix
  # , elm-language-server
}:
    let
      makeElmPkg = { pkgs, additionalInputs ? [ ], pythonPackages ? (ps: [ ]) }:
        pkgs.stdenv.mkDerivation {
          name = "zknotes-elm";
          src = ./.;
          buildPhase = pkgs.elmPackages.fetchElmDeps
            {
              elmPackages = import ./elm/elm-srcs.nix;
              elmVersion = "0.19.1";
              registryDat = ./elm/registry.dat;
            } + ''
            cd elm
            elm-optimize-level-2 src/Main.elm --output=dist/main.js
          '';
          installPhase = ''
            mkdir $out
            cp -r dist/* $out
          '';
          buildInputs = with pkgs;
            [
              elmPackages.elm
              elmPackages.elm-optimize-level-2
            ] ++ additionalInputs;
        };
    in
    flake-utils.lib.eachDefaultSystem
      (
        system:
        let
          toolchain = fenix.packages.${system}.latest;
          rs_compiler = (with toolchain; [ rustc cargo rust-analyzer rust-src ]);

          pname = "zknotes";
          pkgs = nixpkgs.legacyPackages."${system}";
          elm-stuff = makeElmPkg { inherit pkgs; };
          naersk-lib = naersk.lib."${system}";
          rust-stuff = (naersk-lib.override {
            rustc = toolchain.rustc;
            cargo = toolchain.cargo;
          }).buildPackage {
            pname = pname;
            root = ./.;
            buildInputs = with pkgs; [
              # rs_compiler
              yt-dlp
              sqlite
              pkg-config
              openssl.dev
            ];
          };
        in
        rec {
          inherit pname;
          # `nix build`
          packages.${pname} = pkgs.stdenv.mkDerivation {
            nativeBuildInputs = [ pkgs.makeWrapper ];
            name = pname;
            src = ./.;
            # building the 'out' folder
            installPhase = ''
              mkdir -p $out/share/zknotes/static
              mkdir $out/bin
              cp -r $src/server/static $out/share/zknotes
              cp ${elm-stuff}/main.js $out/share/zknotes/static
              cp -r ${rust-stuff}/bin $out
              mv $out/bin/zknotes-server $out/bin/.zknotes-server
              makeWrapper $out/bin/.zknotes-server $out/bin/zknotes-server --set ZKNOTES_STATIC_PATH $out/share/zknotes/static;
            '';
          };
          defaultPackage = packages.${pname};

          # `nix run`
          apps.${pname} = flake-utils.lib.mkApp {
            drv = packages.${pname};
          };
          defaultApp = apps.${pname};

          # `nix develop`
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              rs_compiler
              cargo-watch
              rustfmt
              sqlite
              pkg-config
              openssl.dev
              elm2nix
              elmPackages.elm
              elmPackages.elm-analyse
              elmPackages.elm-doc-preview
              elmPackages.elm-format
              elmPackages.elm-live
              elmPackages.elm-test
              elmPackages.elm-upgrade
              elmPackages.elm-xref
              elmPackages.elm-language-server
              # elm-language-server.defaultPackage.${system}
              elmPackages.elm-verify-examples
              # elmPackages.elmi-to-json
              elmPackages.elm-optimize-level-2
            ];

            shellHook = ''
              export RUST_SRC_PATH=${toolchain.rust-src}/lib/rustlib/src/rust/library
            '';

          };
        }
      ) // {
      nixosModules = { zknotes = import ./module.nix; zknotes-onsave = import ./onsave-module.nix; };
    };
}

