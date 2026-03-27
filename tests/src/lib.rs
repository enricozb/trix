//! A crate demonstrating an example use of trix, and some tests

include!(concat!(env!("OUT_DIR"), "/", "grammars.rs"));

#[cfg(test)]
mod tests {
  #[test]
  fn module() {
    grammars_mod!(pub mod grammars);
    let rust = grammars::rust::language();
    assert!(format!("{rust:?}").starts_with("Language("));
  }

  #[test]
  fn languages_vis() {
    languages!(enum Language);
    let x = Language::Rust;
    assert!(matches!(x, Language::Rust));
  }

  #[test]
  fn languages_attrs() {
    languages!(#[derive(Clone, Copy, Debug)] pub enum Language);
    let x = Language::Rust;
    let y = x;
    assert_eq!(format!("{x:?} {y:?}"), "Rust Rust");
  }

  #[test]
  fn languages_decl() {
    languages_decl!(enum Language);

    // If this compiles, then `as_tree_sitter_language` is not being generated,
    // which is the desired behavior when using `languages_decl!`.
    impl Language {
      #[allow(unused)]
      fn as_tree_sitter_language() {}
    }

    let x = Language::Rust;
    assert!(matches!(x, Language::Rust));
  }

  #[test]
  fn languages_impl() {
    enum Language {
      Rust,
    }
    languages_impl!(Language);
    let rust = Language::Rust.as_tree_sitter_language();
    assert!(format!("{rust:?}").starts_with("Language("));
  }
}
