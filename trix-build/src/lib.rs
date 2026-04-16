pub mod error;

use std::{
  borrow::Cow, collections::{HashMap, HashSet}, ffi::OsStr, fmt::Display, path::{Path, PathBuf}
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
  /// Which expands just to the type declaration portion of [`Self::languages`].
  pub languages_decl: ItemMacro,

  /// A macro invoked like:
  /// ```ignore
  /// languages_impl!($name:ident);
  /// ```
  /// Which expands just to the implementation portion of [`Self::languages`].
  pub languages_impl: ItemMacro,
}

impl Macros {
  /// Generates macros from paths to tree-sitter grammars. These paths should
  /// contain a `tree-sitter.json`, but if they don't, the parts of its contents
  /// which are relevant to `trix` are inferred from the name of the grammar
  /// alone. See [`TreeSitterConfig::from_name`].
  pub fn from_config(trix_config: &TrixConfig) -> Result<Macros> {
    let mut mods: Vec<ItemMod> = Vec::new();
    let mut grammars = Vec::new();
    for (name, source) in &trix_config.sources {
      let tree_sitter_config =
        TreeSitterConfig::from_source(source).unwrap_or_else(|_| TreeSitterConfig::from_name(name.clone()));
      for grammar in tree_sitter_config.grammars {
        let Grammar { path, name, .. } = &grammar;
        let grammar_path = match path {
          Some(path) => source.src.join(path),
          None => source.src.clone(),
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

    let variants = grammars.iter().map(|g| format_ident!("{}", g.camelcase()));
    let arms = grammars.iter().map(|g| -> Arm {
      let fn_ident = format_ident!("tree_sitter_{}", g.name);
      let variant = format_ident!("{}", g.camelcase());
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

#[derive(Default, Deserialize)]
pub struct TrixConfig {
  #[serde(flatten)]
  pub sources: HashMap<String, Source>,
}

impl TrixConfig {
  /// Parses environment variable `var` as a JSON trix config.
  pub fn from_env<S: AsRef<OsStr>>(var: S) -> Result<Self> {
    let config_json = std::env::var(var)?;
    Ok(serde_json::from_str(&config_json)?)
  }

  pub fn from_json<S: AsRef<str>>(s: S) -> Result<Self> {
    Ok(serde_json::from_str(s.as_ref())?)
  }
}

#[derive(Deserialize)]
pub struct Source {
  pub src: PathBuf,
  pub filter: Option<HashSet<String>>,
}

impl Source {
  pub fn new<P: AsRef<Path>>(src: P, filter: Option<impl IntoIterator<Item = impl Display>>) -> Self {
    Self {
      src: src.as_ref().to_owned(),
      filter: filter.map(|filter| filter.into_iter().map(|s| format!("{s}")).collect()),
    }
  }
}

#[derive(Deserialize)]
struct TreeSitterConfig {
  grammars: Vec<Grammar>,
}

impl TreeSitterConfig {
  /// Recursively searches ancestors of `dir` for a `tree-sitter.json`,
  /// and uses it to deserialize into `Self`.
  fn from_source(source: &Source) -> Result<Self> {
    let mut dir = source.src.as_path();
    let mut tree_sitter_json_path = dir.join("tree-sitter.json");
    while !tree_sitter_json_path.exists() {
      dir = dir.parent().ok_or(Error::NoParent)?;
      tree_sitter_json_path = dir.join("tree-sitter.json");
    }
    let json = std::fs::read_to_string(tree_sitter_json_path)?;
    let mut config: Self = serde_json::from_str(&json)?;
    if let Some(filter) = &source.filter {
      config.grammars.retain(|g| filter.contains(&g.name));
    }
    Ok(config)
  }

  /// Generates an inferred version of a `tree-sitter.json` from the `name` of
  /// a grammar. Specifically, a single grammar is inferred with:
  /// - a `name` of `name`
  /// - a `camelcase` of `name` with the first letter capitalized
  /// - a `path` of `.`
  fn from_name(name: String) -> Self {
    Self {
      grammars: vec![Grammar::from_name(name)],
    }
  }
}

#[derive(Clone, Deserialize)]
struct Grammar {
  name: String,
  path: Option<PathBuf>,
  camelcase: Option<String>,
}

impl Grammar {
  fn from_name(name: String) -> Self {
    Self {
      camelcase: Some(capitalize(&name).into_owned()),
      name,
      path: None,
    }
  }

  fn camelcase(&self) -> String {
    self
      .camelcase
      .clone()
      .unwrap_or_else(|| capitalize(&self.name).into_owned())
  }
}

fn capitalize<'a>(s: &'a str) -> Cow<'a, str> {
  let mut i = s.chars();
  let Some(c) = i.next() else { return Cow::Borrowed("") };
  if c.is_uppercase() {
    Cow::Borrowed(s)
  } else {
    Cow::Owned(format!("{}{}", c.to_uppercase(), i.collect::<String>()))
  }
}
