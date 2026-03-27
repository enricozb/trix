//! A crate demonstrating an example use of trix, and some tests

include!(concat!(env!("OUT_DIR"), "/", "grammars.rs"));

#[cfg(test)]
#[allow(unused)]
mod tests {
  #[test]
  fn module() {
    grammars_mod!(pub mod grammars);
    let typescript = grammars::typescript::language();
    assert!(format!("{typescript:?}").starts_with("Language("));
  }

  #[test]
  fn languages_vis() {
    languages!(enum Language);
    let x = Language::TypeScript;
    assert!(matches!(x, Language::TypeScript));
  }

  #[test]
  fn languages_attrs() {
    languages!(#[derive(Clone, Copy, Debug)] pub enum Language);
    let x = Language::TypeScript;
    let y = x;
    assert_eq!(format!("{x:?} {y:?}"), "TypeScript TypeScript");
  }

  #[test]
  fn languages_decl() {
    languages_decl!(enum Language);

    // If this compiles, then `as_tree_sitter_language` is not being generated,
    // which is the desired behavior when using `languages_decl!`.
    impl Language {
      fn as_tree_sitter_language() {}
    }

    let x = Language::TypeScript;
    assert!(matches!(x, Language::TypeScript));
  }

  #[test]
  fn languages_impl() {
    enum Language {
      Fish,
      TypeScript,
    }
    languages_impl!(Language);
    let typescript = Language::TypeScript.as_tree_sitter_language();
    assert!(format!("{typescript:?}").starts_with("Language("));
  }
}
