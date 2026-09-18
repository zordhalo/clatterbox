//! IPC error type (SPEC §8.3). Serializes as `{ "code": string, "message": string }`.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    Io,
    PackInvalid,
    Exists,
    Platform,
}

#[derive(Clone, Debug, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct CmdError {
    pub code: ErrorCode,
    pub message: String,
}

impl CmdError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<clatterbox_core::PackError> for CmdError {
    fn from(err: clatterbox_core::PackError) -> Self {
        Self::new(ErrorCode::PackInvalid, err.to_string())
    }
}

impl From<std::io::Error> for CmdError {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, err.to_string())
    }
}

impl From<tauri::Error> for CmdError {
    fn from(err: tauri::Error) -> Self {
        Self::new(ErrorCode::Platform, err.to_string())
    }
}
