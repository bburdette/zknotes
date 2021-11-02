{
  description = "zknotes, a web based zettelkasten";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
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
              elmPackages.elm-live
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
      in
        rec {
          inherit pname;
          # `nix build`
          packages.${pname} = pkgs.stdenv.mkDerivation {
            name = pname;
            src = ./.;
            # root = ./.;
            # buildInputs = with pkgs; ;
            installPhase = ''
              mkdir $out
              cp -r ${elm-stuff} $out
              cp -r ${rust-stuff} $out
              cp -r $src/server/static $out
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
              rustc
              sqlite
              pkgconfig
              openssl.dev 
              elmPackages.elm
              elmPackages.elm-live
              elmPackages.elm-optimize-level-2
            ];
          };
        }
    );
}

