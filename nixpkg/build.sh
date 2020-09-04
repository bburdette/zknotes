nix-build -E --show-trace 'with import <nixpkgs> { }; callPackage ./default.nix {}'
