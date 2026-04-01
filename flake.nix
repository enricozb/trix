{
  description = "spread";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        mkTrixConfig = grammarSrcs: builtins.toJSON (builtins.mapAttrs mkGrammarDrv grammarSrcs);
        mkGrammarDrv =
          name:
          {
            src,
            filter ? null,
          }:
          let
            drv = pkgs.stdenv.mkDerivation {
              name = "tree-sitter-${name}";
              inherit src;

              nativeBuildInputs = [
                pkgs.nushell
                pkgs.tree-sitter
                pkgs.nodejs_24
              ];

              configurePhase = ''echo "skipping configure"'';
              buildPhase = ''nu ${./build.nu} --build "${name}"'';
              installPhase = "nu ${./build.nu} --install";
            };
          in
          {
            src = drv;
            inherit filter;
          };
      in
      {
        inherit mkTrixConfig;
      }
    );
}
