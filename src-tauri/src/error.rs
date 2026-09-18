//! IPC error type (SPEC §8.3). Serializes as `{ "code": string, "message": string }`.
// WP0 stub: remove this allow once used (WP1).
#![allow(dead_code)]

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
