use std::fs;
use std::path::{Component, Path, PathBuf};

use super::atomic_write;
use crate::error::AppError;
use crate::provider::Provider;

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

fn validate_provider_id(id: &str) -> Result<(), AppError> {
    if id.is_empty() || id.contains('/') || id.contains('\\') {
        return Err(AppError::InvalidProviderId(id.to_string()));
    }

    let mut components = Path::new(id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(AppError::InvalidProviderId(id.to_string())),
    }
}

fn provider_file_path_in(dir: &Path, id: &str) -> Result<PathBuf, AppError> {
    validate_provider_id(id)?;
    Ok(dir.join(format!("{}.json", id)))
}

fn write_provider_to_path(path: &Path, provider: &Provider) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(provider)?;
    atomic_write(path, json.as_bytes())
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
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!(
                        "Skipping unreadable provider file {}: {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            };
            let provider = match serde_json::from_str::<Provider>(&content) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Skipping malformed provider file {}: {}", path.display(), e);
                    continue;
                }
            };

            // Validate: file stem must match provider id
            let expected_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if provider.id != expected_stem {
                log::warn!(
                    "Skipping provider file {} — id mismatch: file stem '{}' != provider id '{}'",
                    path.display(),
                    expected_stem,
                    provider.id
                );
                continue;
            }

            // Validate: required fields must not be empty
            if provider.name.trim().is_empty()
                || provider.api_key.trim().is_empty()
                || provider.base_url.trim().is_empty()
            {
                log::warn!(
                    "Skipping provider file {} — empty required field(s)",
                    path.display()
                );
                continue;
            }

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
    let path = provider_file_path_in(dir, &provider.id)?;
    write_provider_to_path(&path, provider)
}

/// Save an existing provider to a specific directory.
pub fn save_existing_provider_to(dir: &Path, provider: &Provider) -> Result<(), AppError> {
    let path = provider_file_path_in(dir, &provider.id)?;
    if !path.exists() {
        return Err(AppError::NotFound(provider.id.clone()));
    }
    write_provider_to_path(&path, provider)
}

/// Save an existing provider to the iCloud directory.
pub fn save_existing_provider(provider: &Provider) -> Result<(), AppError> {
    let dir = get_icloud_providers_dir()?;
    save_existing_provider_to(&dir, provider)
}

/// Get a specific provider by ID.
pub fn get_provider(id: &str) -> Result<Provider, AppError> {
    let dir = get_icloud_providers_dir()?;
    get_provider_in(&dir, id)
}

/// Get a specific provider from a specific directory.
pub fn get_provider_in(dir: &Path, id: &str) -> Result<Provider, AppError> {
    let path = provider_file_path_in(dir, id)?;
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
    let path = provider_file_path_in(dir, id)?;
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
            cli_id: "claude".to_string(),
            name: name.to_string(),
            protocol_type: ProtocolType::Anthropic,
            api_key: "sk-ant-test".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            model_config: None,
            notes: None,
            test_model: None,
            upstream_model: None,
            upstream_model_map: None,
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
            assert!(
                !name.starts_with('.') || !name.ends_with(".tmp"),
                "Temp file should not remain: {}",
                name
            );
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

    #[test]
    fn test_rejects_unsafe_provider_ids() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        for id in ["../escape", "/tmp/escape", "nested/provider", r"..\\escape"] {
            let provider = make_test_provider(id, "Unsafe", 1710000000000);

            assert!(matches!(
                save_provider_to(dir, &provider),
                Err(AppError::InvalidProviderId(ref invalid)) if invalid == id
            ));
            assert!(matches!(
                get_provider_in(dir, id),
                Err(AppError::InvalidProviderId(ref invalid)) if invalid == id
            ));
            assert!(matches!(
                delete_provider_in(dir, id),
                Err(AppError::InvalidProviderId(ref invalid)) if invalid == id
            ));
        }
    }

    #[test]
    fn test_save_existing_provider_requires_existing_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let provider = make_test_provider("missing-update-id", "Missing", 1710000000000);

        let result = save_existing_provider_to(dir, &provider);
        assert!(matches!(result, Err(AppError::NotFound(ref id)) if id == "missing-update-id"));
    }

    #[test]
    fn test_list_skips_id_filename_mismatch() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Save a valid provider normally
        let good = make_test_provider("good-id", "Good Provider", 1710000000000);
        save_provider_to(dir, &good).unwrap();

        // Manually write a file where filename != internal id
        let bad_json = r#"{"id":"different-id","cli_id":"claude","name":"Bad","protocol_type":"anthropic","api_key":"sk-test","base_url":"https://api.example.com","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("wrong-name.json"), bad_json).unwrap();

        let providers = list_providers_in(dir).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "good-id");
    }

    #[test]
    fn test_list_skips_empty_required_fields() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // Provider with empty name
        let empty_name = r#"{"id":"empty-name","cli_id":"claude","name":"  ","protocol_type":"anthropic","api_key":"sk-test","base_url":"https://api.example.com","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("empty-name.json"), empty_name).unwrap();

        // Provider with empty api_key
        let empty_key = r#"{"id":"empty-key","cli_id":"claude","name":"Test","protocol_type":"anthropic","api_key":"","base_url":"https://api.example.com","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("empty-key.json"), empty_key).unwrap();

        // Provider with empty base_url
        let empty_url = r#"{"id":"empty-url","cli_id":"claude","name":"Test","protocol_type":"anthropic","api_key":"sk-test","base_url":"","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("empty-url.json"), empty_url).unwrap();

        // Provider with empty model is ALLOWED
        let empty_model = r#"{"id":"empty-model","cli_id":"claude","name":"Test","protocol_type":"anthropic","api_key":"sk-test","base_url":"https://api.example.com","model":"","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("empty-model.json"), empty_model).unwrap();

        // Valid provider
        let good = make_test_provider("valid", "Valid Provider", 1710000000000);
        save_provider_to(dir, &good).unwrap();

        let providers = list_providers_in(dir).unwrap();
        assert_eq!(providers.len(), 2);
        let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"empty-model"));
        assert!(ids.contains(&"valid"));
    }

    #[test]
    fn test_list_skips_malformed_json() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        fs::write(dir.join("broken.json"), "not valid json{{{").unwrap();

        let good = make_test_provider("good", "Good", 1710000000000);
        save_provider_to(dir, &good).unwrap();

        let providers = list_providers_in(dir).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "good");
    }

    #[test]
    fn test_list_keeps_non_empty_base_url_visible() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // base_url without http(s):// should still stay visible in read-only listings
        let bad_url = r#"{"id":"bad-url","cli_id":"claude","name":"Test","protocol_type":"anthropic","api_key":"sk-test","base_url":"not-a-url","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("bad-url.json"), bad_url).unwrap();

        // surrounding whitespace should not hide the provider either
        let spaced_url = r#"{"id":"spaced-url","cli_id":"claude","name":"Test","protocol_type":"anthropic","api_key":"sk-test","base_url":" https://api.example.com ","model":"test","created_at":1710000000000,"updated_at":1710000000000,"schema_version":1}"#;
        fs::write(dir.join("spaced-url.json"), spaced_url).unwrap();

        let good = make_test_provider("https-ok", "Good", 1710000000000);
        save_provider_to(dir, &good).unwrap();

        let providers = list_providers_in(dir).unwrap();
        assert_eq!(providers.len(), 3);
        let ids: Vec<&str> = providers.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"bad-url"));
        assert!(ids.contains(&"spaced-url"));
        assert!(ids.contains(&"https-ok"));
    }
}
