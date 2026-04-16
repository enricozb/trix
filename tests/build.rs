use std::path::PathBuf;

use trix_build::{Macros, TrixConfig};

fn main() {
  println!("cargo:rerun-if-env-changed=TRIX_CONFIG_JSON");
  let config = TrixConfig::from_env("TRIX_CONFIG_JSON").unwrap();
  let macros = Macros::from_config(&config).unwrap();
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  std::fs::write(out_dir.join("grammars.rs"), macros.to_string()).unwrap();
}
