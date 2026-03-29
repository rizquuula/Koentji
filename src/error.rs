use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub message: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for AppError {
    fn from(message: String) -> Self {
        AppError { message }
    }
}

impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        AppError {
            message: message.to_string(),
        }
    }
}

#[cfg(feature = "ssr")]
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError {
            message: format!("Database error: {}", err),
        }
    }
}

impl From<leptos::prelude::ServerFnError> for AppError {
    fn from(err: leptos::prelude::ServerFnError) -> Self {
        AppError {
            message: err.to_string(),
        }
    }
}
