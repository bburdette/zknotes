let 
  elm = import elm/shell.nix;
  rust = import server/shell.nix;
in
  with elm;
  with rust;
  stdenv.mkDerivation {
    name = "dev-env";
    buildInputs = elm.buildInputs ++ rust.buildInputs;
  }
