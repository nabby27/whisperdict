use serde::Serialize;
use thiserror::Error;

pub const FREE_LIMIT_REACHED_CODE: &str = "FREE_LIMIT_REACHED";
pub const LICENSE_INVALID_CODE: &str = "LICENSE_INVALID";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error, Clone)]
#[error("{code}: {message}")]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

impl CommandError {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    pub const fn free_limit_reached() -> Self {
        Self::new(FREE_LIMIT_REACHED_CODE, "Free plan limit reached")
    }

    pub const fn license_invalid() -> Self {
        Self::new(LICENSE_INVALID_CODE, "License file is invalid")
    }

    pub fn payload(&self) -> CommandErrorPayload {
        CommandErrorPayload {
            code: self.code.to_string(),
            message: self.message.to_string(),
        }
    }
}

pub fn map_error(error: anyhow::Error) -> String {
    if let Some(command_error) = error.downcast_ref::<CommandError>() {
        return match serde_json::to_string(&command_error.payload()) {
            Ok(payload) => payload,
            Err(_) => command_error.to_string(),
        };
    }

    error.to_string()
}
