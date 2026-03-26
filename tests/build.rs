use std::path::PathBuf;

use trix_build::Macros;

fn main() {
  println!("cargo:rerun-if-env-changed=GRAMMARS");
  let grammar_paths = std::env::var("GRAMMARS").unwrap();
  let grammar_paths: Vec<_> = grammar_paths.split(":").map(PathBuf::from).collect();
  let macros = Macros::from_grammar_paths(&grammar_paths).unwrap();
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  std::fs::write(out_dir.join("grammars.rs"), macros.to_string()).unwrap();
}
