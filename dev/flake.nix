{
  description = "trix-dev";
  inputs = {
    trix.url = "../.";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.follows = "trix/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "trix/nixpkgs";
    };
    crane.url = "github:ipetkov/crane";

    tree-sitter-fish = {
      url = "github:ram02z/tree-sitter-fish";
      flake = false;
    };
    tree-sitter-typescript = {
      url = "github:tree-sitter/tree-sitter-typescript";
      flake = false;
    };
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
      trix,
      tree-sitter-fish,
      tree-sitter-typescript,
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

        treefmt = treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "readme.md";
          programs.nixfmt.enable = true;
          programs.rustfmt.enable = true;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustToolchain);
        grammars = {
          fish.src = tree-sitter-fish;
          typescript = {
            src = tree-sitter-typescript;
            filter = [ "typescript" ];
          };
        };
        trixLib = trix.mkLib pkgs grammars;
      in
      {
        devShells.default = craneLib.devShell {
          TRIX_CONFIG_JSON = builtins.toJSON trixLib.config;
        };

        formatter = treefmt.config.build.wrapper;
      }
    );
}
