pub mod error;

use std::{
  fmt::Display,
  path::{Path, PathBuf},
};

use indoc::formatdoc;
use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Deserialize)]
pub struct TreeSitterConfig {
  grammars: Vec<Grammar>,
}

#[derive(Clone, Deserialize)]
pub struct Grammar {
  name: String,
  path: Option<PathBuf>,
  camelcase: Option<String>,
}

pub fn tree_sitter_json(mut grammar_path: &Path) -> Result<String> {
  let mut tree_sitter_json_path = grammar_path.join("tree-sitter.json");
  while !tree_sitter_json_path.exists() {
    grammar_path = grammar_path.parent().ok_or(Error::NoParent)?;
    tree_sitter_json_path = grammar_path.join("tree-sitter.json");
  }
  Ok(std::fs::read_to_string(tree_sitter_json_path)?)
}

pub struct Macros {
  pub grammars_mod: String,
  pub languages: String,
}

impl Macros {
  pub fn from_grammar_paths(grammar_paths: &[PathBuf]) -> Result<Macros> {
    let mut mods = Vec::new();

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

        mods.push(formatdoc!(
          r#"
            pub mod {name} {{
              unsafe extern "C" {{ fn tree_sitter_{name}() -> tree_sitter::Language; }}

              pub fn language() -> tree_sitter::Language {{ unsafe {{ tree_sitter_{name}() }} }}
            }}
          "#,
        ));
        grammars.push(grammar.clone());
      }
    }

    let mods = mods.join("\n");
    let languages = grammars
      .iter()
      .map(|g| g.camelcase.as_ref().unwrap_or(&g.name).as_str())
      .collect::<Vec<_>>()
      .join(",\n");
    let matches = grammars
      .iter()
      .map(|g| {
        format!(
          r#"Language::{camelcase} => {{
            unsafe extern "C" {{ fn tree_sitter_{name}() -> tree_sitter::Language; }}
            unsafe {{ tree_sitter_{name}() }}
          }}"#,
          camelcase = g.camelcase.as_ref().unwrap_or(&g.name),
          name = g.name,
        )
      })
      .collect::<Vec<_>>()
      .join(",\n");

    let grammars_mod = formatdoc!(
      "
      #[allow(unused)]
      macro_rules! grammars_mod {{
        ($($decl:tt)*) => {{
          $($decl)* {{
            {mods}
          }}
        }}
      }}
    "
    );
    let languages = formatdoc!(
      "
      #[allow(unused)]
      macro_rules! languages {{
        ($($decl:tt)*) => {{
          $($decl)* {{
            {languages}
          }}

          impl Language {{
            pub fn as_tree_sitter_language(self) -> tree_sitter::Language {{
              match self {{
                {matches}
              }}
            }}
          }}
        }};
      }}
    "
    );

    Ok(Macros {
      grammars_mod,
      languages,
    })
  }
}

impl Display for Macros {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}\n\n{}", self.grammars_mod, self.languages)
  }
}
