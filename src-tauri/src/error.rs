use serde::Serialize;
use uuid::Uuid;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub recovery: String,
    pub correlation_id: String,
    #[serde(skip_serializing)]
    technical_detail: String,
}

impl AppError {
    pub fn from_technical(
        code: &'static str,
        message: impl Into<String>,
        recovery: impl Into<String>,
        technical_detail: impl Into<String>,
    ) -> Self {
        let error = Self {
            code,
            message: message.into(),
            recovery: recovery.into(),
            correlation_id: Uuid::new_v4().to_string(),
            technical_detail: technical_detail.into(),
        };
        eprintln!(
            "{} {}: {}",
            error.correlation_id, error.code, error.technical_detail
        );
        error
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::from_technical(
            "INTERNAL_ERROR",
            message,
            "Try again. If the problem continues, restart the app.",
            "Unexpected application state.",
        )
    }

    pub fn technical_detail(&self) -> &str {
        &self.technical_detail
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}
