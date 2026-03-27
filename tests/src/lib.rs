//! A crate demonstrating an example use of trix, and also containing tests,
//! some of which are done rather hackily through doctests' `compile_fail`.

include!(concat!(env!("OUT_DIR"), "/", "grammars.rs"));

/// This should fail as `Languages` does not implement `Debug`.
/// ```compile_fail
/// languages!(pub enum Languages);
///
/// eprintln!("{:?}", Language::Rust);
/// ```
pub fn empty_attrs() {}

#[cfg(test)]
mod tests {
  #[test]
  fn module() {
    grammars_mod!(pub mod grammars);
    let rust = grammars::rust::language();
    assert!(format!("{rust:?}").starts_with("Language("));
  }

  #[test]
  fn vis() {
    languages!(enum Language);
    let x = Language::Rust;
    assert!(matches!(x, Language::Rust));
  }

  #[test]
  fn attrs() {
    languages!(#[derive(Clone, Copy, Debug)] pub enum Language);
    let x = Language::Rust;
    let y = x;
    assert_eq!(format!("{x:?} {y:?}"), "Rust Rust");
  }
}
