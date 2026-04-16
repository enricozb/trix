use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
  #[error("no parent")]
  NoParent,
  #[error("failed to read: {0:?}")]
  Read(#[from] std::io::Error),
  #[error("failed to deserialize tree-sitter.json: {0:?}")]
  Deserialize(#[from] serde_json::Error),
  #[error("failed to read env var: {0}")]
  Var(#[from] std::env::VarError)
}

pub type Result<T> = std::result::Result<T, Error>;
