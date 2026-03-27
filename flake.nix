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
    tree-sitter-rust = {
      url = "github:tree-sitter/tree-sitter-rust";
      flake = false;
    };
    vine.url = "github:VineLang/vine";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      treefmt-nix,

      tree-sitter-fish,
      tree-sitter-rust,
      vine,
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

            # - if `tree-sitter.json` does not exist, we fake a minimal one with
            #   `name` and `metadata` fields.
            # - if `grammar.js` doesn't exist for a grammar, we assume
            #   `tree-sitter generate` has already been executed for it.
            buildPhase = ''
              if ! [ -f tree-sitter.json ]; then
                echo '{ "name": "${name}", "metadata": { "version": "0.0.0" } }' > tree-sitter.json
              fi

              local grammar_paths=$(jq '.grammars? // [{path:"."}] | .[] | .path // "."' tree-sitter.json -r)
              for grammar_path in $grammar_paths; do
                if [ -f "$grammar_path/grammar.js" ]; then
                  tree-sitter generate "$grammar_path/grammar.js"
                fi
              done
            '';

            installPhase = ''
              mkdir $out
              cp tree-sitter.json $out

              local grammar_paths=$(jq '.grammars? // [{path:"."}] | .[] | .path // "."' tree-sitter.json -r)
              for grammar_path in $grammar_paths; do
                mkdir -p "$out/$grammar_path"
                cp -r "$grammar_path/src" "$out/$grammar_path"
              done
            '';
          };
      in
      {
        inherit mkTrixConfig;

        devShells.default = craneLib.devShell {
          GRAMMARS = mkTrixConfig {
            fish = tree-sitter-fish;
            rust = tree-sitter-rust;
            vine = vine.packages.${system}.tree-sitter-vine;
          };
        };

        formatter = treefmt.config.build.wrapper;
      }
    );
}
