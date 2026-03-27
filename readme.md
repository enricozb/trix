<p align="center">
  <img src="https://github.com/enricozb/trix/raw/main/trix.webp" width="100" height="100">
</p>

# trix

A Rust crate and Nix flake to generate a Rust module with tree-sitter languages.

## Why?

1. Not all tree-sitter grammars are published as crates.
2. Not all tree-sitter grammars on crates.io are on the same tree-sitter
   version.
3. Depending on as many grammars as possible gets repetitive with data
   structures representing these languages (`pub enum Language { Rust, .. }`).

## Overview

This repo contains two components:
1. `trix-build`: a Rust crate with code generation functionality for multiple
   tree-sitter grammars at once.
2. a flake which executes `tree-sitter generate` on a list of grammars.

### Rust Crate (`trix-build` )

The `trix-build` crate exposes a `Macros` type with a `from_grammar_paths`
associated function, which takes in a list of paths to tree-sitter grammars,
and outputs Rust code (as strings) for two macros:
- `grammars_mod!($($vis:vis)? mod $name:ident)`: declares a module named `name`
  with a submodule for each grammar containing a function
  `language() -> tree_sitter::Language`.
- `languages!($($attrs)? $($vis:vis)? enum $name:ident)`: defines an enum `name`
  with a variant for each grammar. This enum also has a method
  `as_tree_sitter_language(self) -> tree_sitter::Language`.

These macros can be placed in a generated file (through `build.rs`) and be used
in a local library or binary. Thus, if you vendor your tree-sitter grammars,
`trix-build` can be used to generate a more ergonomic Rust binding relating all
grammars under a common module or type.

See [Usage - Rust](#usage---rust) section for details.

### Nix Flake

The Nix flake is useful for tracking tree-sitter grammars through flake inputs,
and then passing the outputs of `tree-sitter generate` to `trix-build` through
an environment variable. This makes it possible to avoid checking in tree-sitter
grammars and auxiliary files (`tree-sitter.json`, `scanner.c`, etc.) and pinning
them in the `flake.lock` file instead.

See [Usage - Nix](#usage---nix) section for details.

## Usage - Rust

For example, say you have a repository structure like the following:
```
.
├── build.rs
├── Cargo.lock
├── Cargo.toml
├── grammars/
│   ├── rust/
│   │   ├── grammar.js
│   │   ├── src/ (outputs of tree-sitter generate)
│   │   └── tree-sitter.json
│   └── vine/
│       ├── grammar.js
│       ├── src/ (outputs of tree-sitter generate)
│       └── tree-sitter.json
└── src/
    └── main.rs
```
Then, in `build.rs` you can generate a `grammars.rs` which contain macros to
generate a module and an enum representing the grammars in your source tree:
```rust
use std::path::PathBuf;

use trix_build::Macros;

fn main() {
  let grammar_paths = vec!["./grammars/rust", "./grammars/vine"];
  let macros = Macros::from_grammar_paths(&grammar_paths).unwrap();
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  std::fs::write(out_dir.join("grammars.rs"), macros.to_string()).unwrap();
}
```
Then, in `main.rs` (or `lib.rs` for libraries), after including the generated
`grammars.rs` file, there are two macros you can use to reference the grammars:
```rust
include!(concat!(env!("OUT_DIR"), "/", "grammars.rs"));

// A `grammars` module, containing the tree-sitter functions for each grammar:
//
// pub mod grammars {
//   pub mod rust {
//     unsafe extern "C" { fn tree_sitter_rust() -> tree_sitter::Language; }
//
//     pub fn language() -> tree_sitter::Language { unsafe { tree_sitter_rust() } }
//   }
//
//   pub mod vine {
//     unsafe extern "C" { fn tree_sitter_vine() -> tree_sitter::Language; }
//
//     pub fn language() -> tree_sitter::Language { unsafe { tree_sitter_vine() } }
//   }
// }
grammars_mod!(pub mod grammars);

// A `Languages` enum, with each grammar as a variant:
//
// pub enum Language {
//   Rust,
// }
//
// impl Language {
//   pub fn as_tree_sitter_language(self) -> tree_sitter::Language {
//     match self {
//       Language::Rust => tree_sitter_rust::language()
//       Language::Vine => tree_sitter_vine::language()
//     }
//   }
// }
languages!(
  #[derive(Clone, Copy, Debug)]
  pub enum Language
);
```

Note that, at Rust build-time, the outputs of `tree-sitter generate` must be
present in the directories passed to `Macros::from_grammar_paths`. It is up to
you how this is done, but `trix` also has a Nix flake which can facilitate this.

## Usage - Nix

To avoid checking in tree-sitter grammars at all, you can use `mkTrixConfig`
from trix's Nix flake, and pin tree-sitter grammars as inputs:
```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    trix.url = "github:enricozb/trix";
    tree-sitter-rust = {
      url = "github:tree-sitter/tree-sitter-rust";
      flake = false;
    };
    tree-sitter-vine = {
      url = "github:VineLang/vine";
      flake = false;
    };
  };

  outputs = { nixpkgs, trix, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          GRAMMARS = trix.mkTrixConfig.${system} {
            rust = tree-sitter-rust;
            vine = "${tree-sitter-vine}/lsp/tree-sitter-vine";
          };
        };
      }
    );
}
```
The `GRAMMARS` environment variable above is json string which can be
deserialized into a `TrixConfig`:
```rust
use trix_build::{Macros, TrixConfig};

fn main() {
  println!("cargo:rerun-if-env-changed=GRAMMARS");
  let grammars_json = std::env::var("GRAMMARS").unwrap();
  let config = TrixConfig::from_json(&grammars_json).unwrap();
  let macros = Macros::from_config(&config).unwrap();
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  std::fs::write(out_dir.join("grammars.rs"), macros.to_string()).unwrap();
}
```

A few notes about `mkTrixConfig`:
- if `tree-sitter.json` does not exist, we fake a minimal one with `name` and
  `metadata` fields.
- if `grammar.js` doesn't exist for a grammar, we assume `tree-sitter generate`
  has already been executed for it.
