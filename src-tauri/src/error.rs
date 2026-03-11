#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid provider ID: {0}")]
    InvalidProviderId(String),
    #[error("Provider not found: {0}")]
    NotFound(String),
    #[error("iCloud directory not available")]
    ICloudUnavailable,
    #[error("TOML error: {0}")]
    Toml(String),
    #[error("Validation failed: {0}")]
    Validation(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
