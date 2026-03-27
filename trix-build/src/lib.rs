pub mod error;

use std::{
  fmt::Display,
  path::{Path, PathBuf},
};

use quote::format_ident;
use serde::Deserialize;
use syn::{Arm, ItemMacro, ItemMod, parse_quote};

use crate::error::{Error, Result};

/// Macros to generate Rust code containing tree-sitter grammars.
pub struct Macros {
  /// A macro invoked like:
  /// ```ignore
  /// grammars_mod!(<token trees>);
  /// ```
  /// Which expands to a module declaration where each grammar is a submodule:
  /// ```ignore
  /// <token trees> {
  ///   pub mod rust {
  ///     unsafe extern "C" { fn tree_sitter_rust() -> tree_sitter::Language; }
  ///     pub fn language() -> tree_sitter::Language { unsafe { tree_sitter_rust() } }
  ///   }
  ///
  ///   pub mod vine {
  ///     unsafe extern "C" { fn tree_sitter_vine() -> tree_sitter::Language; }
  ///     pub fn language() -> tree_sitter::Language { unsafe { tree_sitter_vine() } }
  ///   }
  ///
  ///   // ..
  /// }
  /// ```
  /// The input to the macro should describe how to declare the module, e.g.
  /// ```ignore
  /// grammars_mod!(pub mod grammars);
  /// ```
  pub grammars_mod: ItemMacro,

  /// A macro invoked like:
  /// ```ignore
  /// languages!($(#[$meta:meta])* $vis:vis enum $name:ident);
  /// ```
  /// Which expands to a type declaration where each grammar is a variant:
  /// ```ignore
  /// $(#[$meta])* $vis enum $name {
  ///   Rust,
  ///   Vine,
  ///   // ..
  /// }
  ///
  /// impl $name {
  ///   pub fn as_tree_sitter_language(self) -> tree_sitter::Language {
  ///     match self {
  ///       Language::Rust => tree_sitter_rust::language()
  ///       Language::Vine => tree_sitter_vine::language()
  ///     }
  ///   }
  /// }
  /// ```
  /// The input to the macro should describe how to declare the type, e.g.
  /// ```ignore
  /// languages!(#[derive(Clone, Copy, Debug)] pub mod Languages);
  /// ```
  pub languages: ItemMacro,

  /// A macro invoked like:
  /// ```ignore
  /// languages_decl!($(#[$meta:meta])* $vis:vis enum $name:ident);
  /// ```
  /// Which expands just to the type declaration portion of `Self::languages`.
  pub languages_decl: ItemMacro,

  /// A macro invoked like:
  /// ```ignore
  /// languages_impl!($name:ident);
  /// ```
  /// Which expands just to the implementation portion of `Self::languages`.
  pub languages_impl: ItemMacro,
}

impl Macros {
  /// Generates macros from paths to tree-sitter grammars. Each path must
  /// contain the outputs of `tree-sitter generate`. Each path may specify
  /// multiple grammars in its `tree-sitter.json`. If a `tree-sitter.json` is
  /// not found in a path, its ancestors are searched.
  pub fn from_grammar_paths(grammar_paths: &[PathBuf]) -> Result<Macros> {
    let mut mods: Vec<ItemMod> = Vec::new();
    let mut grammars = Vec::new();
    for grammar_path in grammar_paths {
      let tree_sitter_json = tree_sitter_json(grammar_path)?;

      let tree_sitter_config: TreeSitterConfig = serde_json::from_str(&tree_sitter_json)?;
      for grammar in tree_sitter_config.grammars {
        let Grammar { path, name, .. } = &grammar;
        let grammar_path = match path {
          Some(path) => grammar_path.join(path),
          None => grammar_path.clone(),
        };

        let mut build = cc::Build::new();
        let src_path = grammar_path.join("src");
        let parser_path = src_path.join("parser.c");
        let scanner_path = src_path.join("scanner.c");
        build
          .include(src_path)
          .opt_level(2) // To ignore FORTIFY_SOURCE warnings
          .flag("-Wno-unused-but-set-variable")
          .flag("-Wno-unused-label")
          .flag("-Wno-unused-parameter")
          .flag("-Wno-unused-value")
          .file(parser_path);
        if scanner_path.exists() {
          build.file(scanner_path);
        }
        build.compile(&format!("tree-sitter-{name}"));

        let mod_ident = format_ident!("{}", name);
        let fn_ident = format_ident!("tree_sitter_{}", name);
        mods.push(parse_quote! {
          #[allow(unused)]
          pub mod #mod_ident {
            unsafe extern "C" { fn #fn_ident() -> tree_sitter::Language; }

            pub fn language() -> tree_sitter::Language { unsafe { #fn_ident() } }
          }
        });
        grammars.push(grammar.clone());
      }
    }

    let variants = grammars
      .iter()
      .map(|g| format_ident!("{}", g.camelcase.as_ref().unwrap_or(&g.name).as_str()));
    let arms = grammars.iter().map(|g| -> Arm {
      let fn_ident = format_ident!("tree_sitter_{}", g.name);
      let variant = format_ident!("{}", g.camelcase.as_ref().unwrap_or(&g.name));
      parse_quote! {
        Language::#variant => {
            unsafe extern "C" { fn #fn_ident() -> tree_sitter::Language; }
            unsafe { #fn_ident() }
        }
      }
    });

    let grammars_mod = parse_quote! {
      #[allow(unused)]
      macro_rules! grammars_mod {
        ($($decl:tt)*) => {
          $($decl)* {
            #(#mods)*
          }
        }
      }
    };
    let languages_decl = parse_quote! {
      #[allow(unused)]
      macro_rules! languages_decl {
        ($(#[$meta:meta])* $vis:vis enum $name:ident) => {
          $(#[$meta])* $vis enum $name {
            #(#variants,)*
          }
        }
      }
    };

    let languages_impl = parse_quote! {
      #[allow(unused)]
      macro_rules! languages_impl {
        ($name:ident) => {
          #[allow(unused)]
          impl $name {
            pub fn as_tree_sitter_language(self) -> tree_sitter::Language {
              match self {
                #(#arms)*
              }
            }
          }
        };
      }
    };

    let languages = parse_quote! {
      #[allow(unused)]
      macro_rules! languages {
        ($(#[$meta:meta])* $vis:vis enum $name:ident) => {
          languages_decl!($(#[$meta])* $vis enum $name);
          languages_impl!($name);
        };
      }
    };

    Ok(Macros {
      grammars_mod,
      languages,
      languages_decl,
      languages_impl,
    })
  }
}

impl Display for Macros {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let Self {
      grammars_mod,
      languages,
      languages_decl,
      languages_impl,
    } = self;
    let file = parse_quote! {
      #grammars_mod
      #languages
      #languages_decl
      #languages_impl
    };
    let pretty = prettyplease::unparse(&file);
    write!(f, "{}", pretty)
  }
}

#[derive(Deserialize)]
struct TreeSitterConfig {
  grammars: Vec<Grammar>,
}

#[derive(Clone, Deserialize)]
struct Grammar {
  name: String,
  path: Option<PathBuf>,
  camelcase: Option<String>,
}

fn tree_sitter_json(mut grammar_path: &Path) -> Result<String> {
  let mut tree_sitter_json_path = grammar_path.join("tree-sitter.json");
  while !tree_sitter_json_path.exists() {
    grammar_path = grammar_path.parent().ok_or(Error::NoParent)?;
    tree_sitter_json_path = grammar_path.join("tree-sitter.json");
  }
  Ok(std::fs::read_to_string(tree_sitter_json_path)?)
}
