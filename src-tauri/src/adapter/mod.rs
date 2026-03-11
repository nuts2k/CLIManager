pub mod claude;

use crate::error::AppError;
use crate::provider::Provider;
use std::fs;
use std::path::Path;

/// Result of a patch operation.
#[derive(Debug)]
pub struct PatchResult {
    pub files_written: Vec<String>,
    pub backups_created: Vec<String>,
}

/// Unified interface for CLI config patching.
pub trait CliAdapter {
    /// Human-readable CLI name for error messages.
    fn cli_name(&self) -> &str;

    /// Patch CLI config files with the given provider's credentials.
    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError>;
}

/// Create a timestamped backup of `source` in `backup_dir`.
/// Returns the backup file path as a String.
pub fn create_backup(source: &Path, backup_dir: &Path) -> Result<String, AppError> {
    fs::create_dir_all(backup_dir).map_err(|e| AppError::Io {
        path: backup_dir.display().to_string(),
        source: e,
    })?;

    let filename = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H-%M-%S%.3f");
    let backup_name = format!("{}.{}.bak", filename, timestamp);
    let backup_path = backup_dir.join(&backup_name);

    fs::copy(source, &backup_path).map_err(|e| AppError::Io {
        path: backup_path.display().to_string(),
        source: e,
    })?;

    Ok(backup_path.display().to_string())
}

/// Rotate backups in `backup_dir`, keeping at most `max_count` .bak files.
/// Deletes oldest first (sorted by filename, which includes timestamp).
pub fn rotate_backups(backup_dir: &Path, max_count: usize) -> Result<(), AppError> {
    let entries = fs::read_dir(backup_dir).map_err(|e| AppError::Io {
        path: backup_dir.display().to_string(),
        source: e,
    })?;

    let mut backups: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "bak")
        })
        .collect();

    backups.sort_by_key(|e| e.file_name());

    while backups.len() > max_count {
        let oldest = backups.remove(0);
        let _ = fs::remove_file(oldest.path()); // best-effort
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_backup_copies_file_with_timestamp_suffix() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("settings.json");
        fs::write(&source, r#"{"key":"value"}"#).unwrap();

        let backup_dir = tmp.path().join("backups");
        let result = create_backup(&source, &backup_dir).unwrap();

        assert!(result.contains("settings.json."));
        assert!(result.ends_with(".bak"));

        // Backup content matches source
        let backup_content = fs::read_to_string(&result).unwrap();
        assert_eq!(backup_content, r#"{"key":"value"}"#);
    }

    #[test]
    fn test_create_backup_creates_backup_dir_if_missing() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("settings.json");
        fs::write(&source, "{}").unwrap();

        let backup_dir = tmp.path().join("deep").join("nested").join("backups");
        assert!(!backup_dir.exists());

        let result = create_backup(&source, &backup_dir);
        assert!(result.is_ok());
        assert!(backup_dir.exists());
    }

    #[test]
    fn test_rotate_backups_keeps_at_most_n_backups() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        // Create 7 backup files with ordered timestamps
        for i in 0..7 {
            let name = format!("settings.json.2026-03-11T10-00-0{}.000.bak", i);
            fs::write(backup_dir.join(&name), "content").unwrap();
        }

        rotate_backups(&backup_dir, 5).unwrap();

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
            .collect();
        assert_eq!(remaining.len(), 5);

        // The oldest two (00, 01) should have been deleted
        assert!(!backup_dir
            .join("settings.json.2026-03-11T10-00-00.000.bak")
            .exists());
        assert!(!backup_dir
            .join("settings.json.2026-03-11T10-00-01.000.bak")
            .exists());
    }

    #[test]
    fn test_rotate_backups_noop_when_under_limit() {
        let tmp = TempDir::new().unwrap();
        let backup_dir = tmp.path().join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        for i in 0..3 {
            let name = format!("settings.json.2026-03-11T10-00-0{}.000.bak", i);
            fs::write(backup_dir.join(&name), "content").unwrap();
        }

        rotate_backups(&backup_dir, 5).unwrap();

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
            .collect();
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn test_apperror_toml_variant_displays_message() {
        let err = AppError::Toml("unexpected token at line 3".to_string());
        assert_eq!(err.to_string(), "TOML error: unexpected token at line 3");
    }

    #[test]
    fn test_apperror_validation_variant_displays_message() {
        let err = AppError::Validation("invalid JSON in settings.json".to_string());
        assert_eq!(
            err.to_string(),
            "Validation failed: invalid JSON in settings.json"
        );
    }
}
