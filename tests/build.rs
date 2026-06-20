use std::path::PathBuf;

use trix_build::{Macros, TrixConfig};

fn main() {
  println!("cargo:rerun-if-env-changed=VENDOR_DIR");
  println!("cargo:rerun-if-env-changed=TRIX_CONFIG_JSON");

  // Prefer a vendor directory (e.g. produced by `trix vendor`) if one is set.
  // This allows building without access to the original (e.g. nix-store)
  // grammar sources referenced by `TRIX_CONFIG_JSON`.
  let config = match std::env::var("VENDOR_DIR") {
    Ok(vendor_dir) => TrixConfig::from_vendor_dir(vendor_dir).unwrap(),
    Err(_) => TrixConfig::from_env("TRIX_CONFIG_JSON").unwrap(),
  };

  let macros = Macros::from_config(&config).unwrap();
  let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  std::fs::write(out_dir.join("grammars.rs"), macros.to_string()).unwrap();
}
