use thiserror::Error;

#[derive(Error, Debug)]
pub enum IpsError {
    #[error("Invalid Patch: `{0}")]
    InvalidPatch(String),

    #[error("Bad IO")]
    Io(#[from] std::io::Error),

    #[error("Bad path")]
    InvalidPath(),
}
