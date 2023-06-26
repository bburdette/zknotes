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
      mytaurimobile = { pkgs }: pkgs.callPackage ./my-tauri-mobile.nix {};
    in
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pname = "zknotes";
        # pkgs = nixpkgs.legacyPackages."${system}"
        pkgs = import nixpkgs {
          config.android_sdk.accept_license = true;
          config.allowUnfree = true;
          system = "${system}"; };
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
        my-tauri-mobile = mytaurimobile { inherit pkgs; };

        # fenix stuff for adding other compile targets
        mkToolchain = fenix.packages.${system}.combine;
        toolchain = fenix.packages.${system}.stable;
        target1 = fenix.packages.${system}.targets."aarch64-linux-android".stable;
        target2 = fenix.packages.${system}.targets."armv7-linux-androideabi".stable;
        target3 = fenix.packages.${system}.targets."i686-linux-android".stable;
        target4 = fenix.packages.${system}.targets."x86_64-linux-android".stable;

        mobileTargets = mkToolchain (with toolchain; [
          cargo
          rustc
          target1.rust-std
          target2.rust-std
          target3.rust-std
          target4.rust-std
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

          # meh = pkgs.androidenv.androidPkgs_9_0 // { android_sdk.accept_license = true; };
          # androidComposition = pkgs.androidenv.androidPkgs_9_0 // { includeNDK = true; };

          androidEnv = pkgs.androidenv.override { licenseAccepted = true; };
          androidComposition = androidEnv.composeAndroidPackages {
            includeNDK = true;
            platformToolsVersion = "33.0.3";
            buildToolsVersions = [ "30.0.3" ];
            platformVersions  = ["33"];
            extraLicenses = [
              "android-googletv-license"
              "android-sdk-arm-dbt-license"
              "android-sdk-license"
              "android-sdk-preview-license"
              "google-gdk-license"
              "intel-android-extra-license"
              "intel-android-sysimage-license"
              "mips-android-sysimage-license"            ];
          };
          # `nix develop`
          devShell = pkgs.mkShell {

            NIX_LD= "${pkgs.stdenv.cc.libc}/lib/ld-linux-x86-64.so.2";
            ANDROID_HOME = "${androidComposition.androidsdk}/libexec/android-sdk";
            NDK_HOME = "${androidComposition.androidsdk}/libexec/android-sdk/ndk/${builtins.head (pkgs.lib.lists.reverseList (builtins.split "-" "${androidComposition.ndk-bundle}"))}";

            nativeBuildInputs = with pkgs; [
              androidComposition.androidsdk
              androidComposition.ndk-bundle
              # cargo
              # rustc
              cargo-watch
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
              # for tauti-mobile
              librsvg
              webkitgtk_4_1
              # tauri-mobile
              my-tauri-mobile
              lldb
              nodejs
              # rustup # `cargo tauri android init` wants this, even though targets already installed.
                     # should be fixed though, https://github.com/tauri-apps/tauri/issues/7044
              alsa-lib
              mobileTargets
              # they suggest using the jbr (jetbrains runtime?) from android-studio, but that is not accessible.
              jetbrains.jdk
              ];
          };
        }
    );
}

