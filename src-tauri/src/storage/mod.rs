pub mod icloud;
pub mod local;

use crate::error::AppError;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Atomically write data to a file by writing to a unique temp file first, then renaming.
///
/// 使用 tempfile 生成唯一临时文件名，避免并发写入时共享同一 inode
/// 导致的数据损坏和 rename 失败。
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| AppError::Io {
        path: path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"),
    })?;
    fs::create_dir_all(parent).map_err(|e| AppError::Io {
        path: parent.display().to_string(),
        source: e,
    })?;

    // 使用 tempfile 在同一目录创建唯一临时文件（同一文件系统，保证 rename 原子性）
    let mut tmp_file = tempfile::NamedTempFile::new_in(parent).map_err(|e| AppError::Io {
        path: parent.display().to_string(),
        source: e,
    })?;

    tmp_file.write_all(data).map_err(|e| AppError::Io {
        path: tmp_file.path().display().to_string(),
        source: e,
    })?;
    tmp_file.flush().map_err(|e| AppError::Io {
        path: tmp_file.path().display().to_string(),
        source: e,
    })?;

    // persist = rename tmp → target，失败时自动清理临时文件
    tmp_file.persist(path).map_err(|e| AppError::Io {
        path: path.display().to_string(),
        source: e.error,
    })?;

    Ok(())
}
