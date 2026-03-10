use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::provider::Provider;
use super::atomic_write;

/// Resolve the iCloud providers directory.
/// Falls back to ~/.cli-manager/providers/ if iCloud Drive is unavailable.
pub fn get_icloud_providers_dir() -> Result<PathBuf, AppError> {
    get_providers_dir_impl(None)
}

fn get_providers_dir_impl(override_dir: Option<&Path>) -> Result<PathBuf, AppError> {
    if let Some(dir) = override_dir {
        fs::create_dir_all(dir).map_err(|e| AppError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
        return Ok(dir.to_path_buf());
    }

    let home = dirs::home_dir().ok_or(AppError::ICloudUnavailable)?;
    let mobile_docs = home.join("Library/Mobile Documents");

    let providers_dir = if mobile_docs.exists() {
        home.join("Library/Mobile Documents/com~apple~CloudDocs/CLIManager/providers")
    } else {
        log::warn!("iCloud Drive not available, falling back to ~/.cli-manager/providers/");
        home.join(".cli-manager/providers")
    };

    if !providers_dir.exists() {
        fs::create_dir_all(&providers_dir).map_err(|e| AppError::Io {
            path: providers_dir.display().to_string(),
            source: e,
        })?;
    }

    Ok(providers_dir)
}

fn provider_file_path_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{}.json", id))
}

/// List all providers, sorted by created_at.
pub fn list_providers() -> Result<Vec<Provider>, AppError> {
    let dir = get_icloud_providers_dir()?;
    list_providers_in(&dir)
}

/// List all providers in a specific directory, sorted by created_at.
pub fn list_providers_in(dir: &Path) -> Result<Vec<Provider>, AppError> {
    let mut providers = Vec::new();

    if !dir.exists() {
        return Ok(providers);
    }

    let entries = fs::read_dir(dir).map_err(|e| AppError::Io {
        path: dir.display().to_string(),
        source: e,
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let content = fs::read_to_string(&path).map_err(|e| AppError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            let provider: Provider = serde_json::from_str(&content)?;
            providers.push(provider);
        }
    }

    providers.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    Ok(providers)
}

/// Save a provider to the iCloud directory.
pub fn save_provider(provider: &Provider) -> Result<(), AppError> {
    let dir = get_icloud_providers_dir()?;
    save_provider_to(&dir, provider)
}

/// Save a provider to a specific directory.
pub fn save_provider_to(dir: &Path, provider: &Provider) -> Result<(), AppError> {
    let path = provider_file_path_in(dir, &provider.id);
    let json = serde_json::to_string_pretty(provider)?;
    atomic_write(&path, json.as_bytes())
}

/// Get a specific provider by ID.
pub fn get_provider(id: &str) -> Result<Provider, AppError> {
    let dir = get_icloud_providers_dir()?;
    get_provider_in(&dir, id)
}

/// Get a specific provider from a specific directory.
pub fn get_provider_in(dir: &Path, id: &str) -> Result<Provider, AppError> {
    let path = provider_file_path_in(dir, id);
    if !path.exists() {
        return Err(AppError::NotFound(id.to_string()));
    }
    let content = fs::read_to_string(&path).map_err(|e| AppError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    let provider: Provider = serde_json::from_str(&content)?;
    Ok(provider)
}

/// Delete a provider by ID.
pub fn delete_provider(id: &str) -> Result<(), AppError> {
    let dir = get_icloud_providers_dir()?;
    delete_provider_in(&dir, id)
}

/// Delete a provider from a specific directory.
pub fn delete_provider_in(dir: &Path, id: &str) -> Result<(), AppError> {
    let path = provider_file_path_in(dir, id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| AppError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProtocolType;
    use tempfile::TempDir;

    fn make_test_provider(id: &str, name: &str, created_at: i64) -> Provider {
        Provider {
            id: id.to_string(),
            name: name.to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-ant-test".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            created_at,
            updated_at: created_at,
            schema_version: 1,
        }
    }

    #[test]
    fn test_save_creates_json_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let provider = make_test_provider("test-uuid-1", "Test Provider", 1710000000000);

        save_provider_to(dir, &provider).unwrap();

        let file_path = dir.join("test-uuid-1.json");
        assert!(file_path.exists(), "Provider JSON file should exist");

        let content = fs::read_to_string(&file_path).unwrap();
        let loaded: Provider = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.id, "test-uuid-1");
        assert_eq!(loaded.name, "Test Provider");
    }

    #[test]
    fn test_list_providers_returns_sorted() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let p1 = make_test_provider("id-1", "Provider A", 1710000002000);
        let p2 = make_test_provider("id-2", "Provider B", 1710000001000);
        let p3 = make_test_provider("id-3", "Provider C", 1710000003000);

        save_provider_to(dir, &p1).unwrap();
        save_provider_to(dir, &p2).unwrap();
        save_provider_to(dir, &p3).unwrap();

        let providers = list_providers_in(dir).unwrap();
        assert_eq!(providers.len(), 3);
        assert_eq!(providers[0].id, "id-2"); // earliest created_at
        assert_eq!(providers[1].id, "id-1");
        assert_eq!(providers[2].id, "id-3"); // latest created_at
    }

    #[test]
    fn test_get_provider_returns_specific() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let provider = make_test_provider("get-test-id", "Get Test", 1710000000000);

        save_provider_to(dir, &provider).unwrap();

        let loaded = get_provider_in(dir, "get-test-id").unwrap();
        assert_eq!(loaded.name, "Get Test");
    }

    #[test]
    fn test_get_provider_not_found() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let result = get_provider_in(dir, "nonexistent-id");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(id) => assert_eq!(id, "nonexistent-id"),
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_delete_provider_removes_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let provider = make_test_provider("del-test-id", "Delete Test", 1710000000000);

        save_provider_to(dir, &provider).unwrap();
        assert!(dir.join("del-test-id.json").exists());

        delete_provider_in(dir, "del-test-id").unwrap();
        assert!(!dir.join("del-test-id.json").exists());

        // Subsequent get should return NotFound
        let result = get_provider_in(dir, "del-test-id");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn test_save_provider_overwrites_existing() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let mut provider = make_test_provider("update-id", "Original Name", 1710000000000);
        save_provider_to(dir, &provider).unwrap();

        provider.name = "Updated Name".to_string();
        provider.updated_at = 1710000001000;
        save_provider_to(dir, &provider).unwrap();

        let loaded = get_provider_in(dir, "update-id").unwrap();
        assert_eq!(loaded.name, "Updated Name");
        assert_eq!(loaded.updated_at, 1710000001000);
    }

    #[test]
    fn test_atomic_write_uses_rename() {
        // Verify atomic write by checking no .tmp files remain after write
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let provider = make_test_provider("atomic-test", "Atomic Test", 1710000000000);

        save_provider_to(dir, &provider).unwrap();

        // No .tmp files should remain
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            assert!(!name.starts_with('.') || !name.ends_with(".tmp"),
                "Temp file should not remain: {}", name);
        }
    }

    #[test]
    fn test_dir_created_automatically() {
        let tmp = TempDir::new().unwrap();
        let nested_dir = tmp.path().join("nested/providers");
        assert!(!nested_dir.exists());

        let provider = make_test_provider("nested-test", "Nested Test", 1710000000000);
        // Use get_providers_dir_impl with override to test dir creation
        let resolved = get_providers_dir_impl(Some(&nested_dir)).unwrap();
        assert!(resolved.exists());

        save_provider_to(&resolved, &provider).unwrap();
        assert!(resolved.join("nested-test.json").exists());
    }

    #[test]
    fn test_list_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let providers = list_providers_in(tmp.path()).unwrap();
        assert!(providers.is_empty());
    }
}
