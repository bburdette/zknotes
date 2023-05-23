{
  description = "zknotes, a web based zettelkasten";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, naersk, fenix }:
    let
      makeElmPkg = { pkgs, additionalInputs ? [ ], pythonPackages ? (ps: [ ]) }:
        pkgs.stdenv.mkDerivation {
          name = "zknotes-elm";
          src = ./.;
          buildPhase = pkgs.elmPackages.fetchElmDeps {
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
      mytauri = { pkgs }: pkgs.callPackage ./my-tauri.nix {};
    in
    flake-utils.lib.eachDefaultSystem (
      system: 
      let
        pname = "zknotes";
        pkgs = nixpkgs.legacyPackages."${system}";
        # aarch64-linux-android-pkgs = nixpkgs.legacyPackages."aarch64-linux-android";
        # aarch64-linux-android-pkgs = nixpkgs.legacyPackages."aarch64-linux";
        naersk-lib = naersk.lib."${system}";
        elm-stuff = makeElmPkg { inherit pkgs; };
        rust-stuff = naersk-lib.buildPackage {
            pname = pname;
            root = ./.;
            buildInputs = with pkgs; [
              cargo
              rustc
              sqlite
              pkgconfig
              openssl.dev 
              ];
          };

        my-tauri = mytauri { inherit pkgs; };

        # fenix stuff for adding other compile targets
        mkToolchain = fenix.packages.${system}.combine;
        toolchain = fenix.packages.${system}.stable;
        target1 = fenix.packages.${system}.targets."aarch64-linux-android".stable;
        target2 = fenix.packages.${system}.targets."armv7-linux-androideabi".stable;
        target3 = fenix.packages.${system}.targets."i686-linux-android".stable;
        target4 = fenix.packages.${system}.targets."x86_64-linux-android".stable;

        mobileTargets = mkToolchain (with toolchain; [
          cargo
          # clippy
          # rust-src
          rustc
          # target.rust-std
          target1.rust-std
          target2.rust-std
          target3.rust-std
          target4.rust-std

          # Always use nightly rustfmt because most of its options are unstable
          # fenix.packages.${system}.latest.rustfmt
        ]);


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
              # cargo
              cargo-watch
              # rustc
              rustfmt
              rust-analyzer
              sqlite
              openssl.dev
              # aarch64-linux-android-pkgs.sqlite
              # aarch64-linux-android-pkgs.openssl.dev
              pkgconfig
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
              elmPackages.elm-verify-examples
              elmPackages.elmi-to-json
              elmPackages.elm-optimize-level-2
              # extra stuff for tauri
              my-tauri
              # cargo-tauri
              libsoup
              cairo
              atk
              webkitgtk
              gst_all_1.gstreamer
              gst_all_1.gst-plugins-base
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad
              # for tauti-mobile (?)
              tauri-mobile
              lldb
              nodejs
              # vscode-extensions.vadimcn.vscode-lldb   #  added this but still not found by tauri mobile template init.
              alsa-lib
              mobileTargets 
              # they suggest using the jbr (jetbrains runtime?) from android-studio, but that is not accessible.
              jetbrains.jdk
              ];
          };
        }
    );
}

