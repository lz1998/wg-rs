use std::io;

use thiserror::Error;

pub type WgResult<T> = Result<T, WgError>;

#[derive(Error, Debug)]
pub enum WgError {
    #[error("invalid packet")]
    InvalidPacket,
    #[error("io error, {0}")]
    IO(#[from] io::Error),
}
