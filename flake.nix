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
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      treefmt-nix,
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

        mkGrammarDrvs =
          grammarSrcs: builtins.concatStringsSep ":" (pkgs.lib.mapAttrsToList mkGrammarDrv grammarSrcs);
        mkGrammarDrv =
          name: src:
          pkgs.stdenv.mkDerivation {
            name = "tree-sitter-${name}";
            inherit src;
            nativeBuildInputs = [
              pkgs.jq
              pkgs.tree-sitter
              pkgs.nodejs_24
            ];
            configurePhase = ''
              echo 'skipping configure'
            '';
            buildPhase = ''
              for grammar_path in $(jq '.grammars[].path // "."' tree-sitter.json -r); do
                tree-sitter generate "$grammar_path/grammar.js"
              done
            '';
            installPhase = ''
              mkdir $out
              cp tree-sitter.json $out

              for grammar_path in $(jq '.grammars[].path // "."' tree-sitter.json -r); do
                echo checking $grammar_path

                ls "$grammar_path"
                mkdir -p "$out/$grammar_path"
                cp -r "$grammar_path/src" "$out/$grammar_path"
              done
            '';
          };
      in
      {
        inherit mkGrammarDrvs;

        devShells.default = craneLib.devShell {
          GRAMMARS = mkGrammarDrvs {
            rust = ./tests/grammar;
          };
        };

        formatter = treefmt.config.build.wrapper;
      }
    );
}
