{
  description = "zknotes, a web based zettelkasten";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
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
    in
    flake-utils.lib.eachDefaultSystem (
      system: let
        pname = "zknotes";
        pkgs = nixpkgs.legacyPackages."${system}";
        naersk-lib = naersk.lib."${system}";
        elm-stuff = makeElmPkg { inherit pkgs; };
        rust-stuff = naersk-lib.buildPackage {
            naersk.inputs.nixpkgs.follows = "nixpkgs";
            pname = pname;
            root = ./.;
            buildInputs = with pkgs; [
              cargo
              rustc
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
              cargo
              cargo-watch
              rustc
              rustfmt
              rust-analyzer
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
              elmPackages.elm-verify-examples
              elmPackages.elmi-to-json
              elmPackages.elm-optimize-level-2
            ];
          };
        }
    ) // {
      nixosModules = { zknotes = import ./module.nix; };
    };
}

