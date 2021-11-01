{
  description = "zknotes, a web based zettelkasten";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        naersk-lib = naersk.lib."${system}";
      in
        rec {
          pname = "zknotes";

          # `nix build`
          packages.${pname} = naersk-lib.buildPackage {
            pname = pname;
            # src = ./.;
            root = ./.;
            buildInputs = with pkgs; [
              cargo
              rustc
              sqlite
              pkgconfig
              openssl.dev 
              nix
              ];
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
              nix
              ];
          };
        }
    );
}

