{
  description = "spread";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # included as it does not include a tree-sitter.json
    tree-sitter-fish = {
      url = "github:ram02z/tree-sitter-fish";
      flake = false;
    };
    tree-sitter-typescript = {
      url = "github:tree-sitter/tree-sitter-typescript";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      treefmt-nix,

      tree-sitter-fish,
      tree-sitter-typescript,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain);
        treefmt = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs.nixfmt.enable = true;
          programs.rustfmt.enable = true;
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

        devShells.default = craneLib.devShell {
          GRAMMARS = mkTrixConfig {
            fish.src = tree-sitter-fish;
            typescript = {
              src = tree-sitter-typescript;
              filter = [ "typescript" ];
            };
          };
        };

        formatter = treefmt.config.build.wrapper;
      }
    );
}
