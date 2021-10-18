let 
  nixpkgs = import <nixpkgs> {};
in
  with nixpkgs;
  stdenv.mkDerivation {
    name = "elm-env";
    buildInputs = [ 
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
  }
