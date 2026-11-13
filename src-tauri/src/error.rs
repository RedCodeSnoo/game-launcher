//! Erreur renvoyée à l'interface : un code stable (traduit côté interface) et un détail.
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: String,
    pub detail: String,
}

impl AppError {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        AppError { code: code.to_string(), detail: detail.into() }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        match e.downcast::<AppError>() {
            Ok(app) => app,
            Err(e) => AppError::new("INTERNAL", format!("{e:#}")),
        }
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::new("DATABASE", e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::new("IO", e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
