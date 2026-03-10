pub mod icloud;

use crate::error::AppError;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Atomically write data to a file by writing to a temp file first, then renaming.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"),
    })?;
    fs::create_dir_all(parent).map_err(|e| AppError::Io {
        path: parent.display().to_string(),
        source: e,
    })?;

    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));

    let mut file = fs::File::create(&tmp_path).map_err(|e| AppError::Io {
        path: tmp_path.display().to_string(),
        source: e,
    })?;
    file.write_all(data).map_err(|e| AppError::Io {
        path: tmp_path.display().to_string(),
        source: e,
    })?;
    file.flush().map_err(|e| AppError::Io {
        path: tmp_path.display().to_string(),
        source: e,
    })?;

    fs::rename(&tmp_path, path).map_err(|e| AppError::Io {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}
