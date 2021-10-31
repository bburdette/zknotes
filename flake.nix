{
  description = "zknotes, a web based zettelkasten";

  inputs.nixpkgs.url = github:NixOS/nixpkgs/21.05;

  outputs = { self, nixpkgs }: {

    packages.x86_64-linux.hello = nixpkgs.legacyPackages.x86_64-linux.hello;

    defaultPackage.x86_64-linux = self.packages.x86_64-linux.hello;

  };
}
