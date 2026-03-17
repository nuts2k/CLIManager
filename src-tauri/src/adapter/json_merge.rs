use serde_json::Value;

use crate::error::AppError;

// ============================================================
// 保护字段常量
// ============================================================

/// overlay 中不允许被用户覆盖的 env 字段（由 Provider/Proxy patch 决定最终值）
pub const PROTECTED_ENV_KEYS: [&str; 2] = ["ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_BASE_URL"];

// ============================================================
// 保护字段剥离
// ============================================================

/// 从 overlay 中移除保护字段，返回被移除路径列表与净化后的 overlay
#[derive(Debug)]
pub struct StripResult {
    /// 被移除的字段路径列表（形如 "env.ANTHROPIC_AUTH_TOKEN"）
    pub stripped_paths: Vec<String>,
    /// 移除保护字段后的 overlay（结构不变，仅删去保护字段）
    pub overlay: Value,
}

/// 从 overlay 中剥离保护字段。
///
/// - overlay 的 root 必须是 object，否则返回 Validation 错误。
/// - 如果 overlay.env 是 object，则移除其中所有 PROTECTED_ENV_KEYS 中的字段。
/// - stripped_paths 形如 "env.ANTHROPIC_AUTH_TOKEN"。
pub fn strip_protected_fields(overlay: &Value) -> Result<StripResult, AppError> {
    let root_obj = overlay.as_object().ok_or_else(|| {
        AppError::Validation("overlay root must be a JSON object".to_string())
    })?;

    let mut stripped_paths = Vec::new();
    let mut new_root = root_obj.clone();

    if let Some(env_val) = new_root.get_mut("env") {
        if let Some(env_obj) = env_val.as_object_mut() {
            for key in PROTECTED_ENV_KEYS {
                if env_obj.remove(key).is_some() {
                    stripped_paths.push(format!("env.{}", key));
                }
            }
        }
        // 如果 env 不是 object（例如是 string/array），保持原样，不做深挖
    }

    Ok(StripResult {
        stripped_paths,
        overlay: Value::Object(new_root),
    })
}

// ============================================================
// 深度合并 + null 删除
// ============================================================

/// 将 overlay 深度合并进 base（in-place），支持 null 删除语义。
///
/// 合并规则：
/// - base/object + overlay/object => 递归合并每个 key
/// - base/any + overlay/array    => overlay 数组整体替换
/// - base/any + overlay/scalar   => overlay 覆盖
/// - overlay 的某个 key 为 null  => 从 base object 删除该 key（noop 若不存在）
///
/// base 必须为 object，否则返回 Validation 错误。
pub fn merge_with_null_delete(
    base: &mut Value,
    overlay: &Value,
) -> Result<(), AppError> {
    let base_obj = base.as_object_mut().ok_or_else(|| {
        AppError::Validation("merge base must be a JSON object".to_string())
    })?;

    let overlay_obj = match overlay.as_object() {
        Some(obj) => obj,
        None => {
            // overlay 不是 object 时，整体替换（按 scalar/array 规则）
            *base = overlay.clone();
            return Ok(());
        }
    };

    for (key, overlay_val) in overlay_obj {
        if overlay_val.is_null() {
            // null 语义：删除 base 中对应 key
            base_obj.remove(key);
        } else if overlay_val.is_object() {
            // 递归合并
            let base_val = base_obj
                .entry(key.clone())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if base_val.is_object() {
                merge_with_null_delete(base_val, overlay_val)?;
            } else {
                // base 对应 key 不是 object，直接替换
                *base_val = overlay_val.clone();
            }
        } else {
            // array 或 scalar：直接覆盖/替换
            base_obj.insert(key.clone(), overlay_val.clone());
        }
    }

    Ok(())
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- strip_protected_fields 测试 ---

    #[test]
    fn test_strip_removes_auth_token() {
        let overlay = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "sk-ant-secret",
                "OTHER_KEY": "keep-me"
            }
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert_eq!(result.stripped_paths, vec!["env.ANTHROPIC_AUTH_TOKEN"]);
        assert!(result.overlay["env"]["ANTHROPIC_AUTH_TOKEN"].is_null());
        assert_eq!(result.overlay["env"]["OTHER_KEY"], "keep-me");
    }

    #[test]
    fn test_strip_removes_base_url() {
        let overlay = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://bad.example.com",
                "CUSTOM": "hello"
            }
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert_eq!(result.stripped_paths, vec!["env.ANTHROPIC_BASE_URL"]);
        assert!(result.overlay["env"]["ANTHROPIC_BASE_URL"].is_null());
        assert_eq!(result.overlay["env"]["CUSTOM"], "hello");
    }

    #[test]
    fn test_strip_removes_both_protected_keys() {
        let overlay = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "secret",
                "ANTHROPIC_BASE_URL": "https://override.example.com",
                "KEEP": "yes"
            }
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert_eq!(result.stripped_paths.len(), 2);
        assert!(result.stripped_paths.contains(&"env.ANTHROPIC_AUTH_TOKEN".to_string()));
        assert!(result.stripped_paths.contains(&"env.ANTHROPIC_BASE_URL".to_string()));
        assert_eq!(result.overlay["env"]["KEEP"], "yes");
    }

    #[test]
    fn test_strip_noop_when_no_protected_keys() {
        let overlay = json!({
            "env": {
                "MY_VAR": "value"
            },
            "other": 42
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert!(result.stripped_paths.is_empty());
        assert_eq!(result.overlay, overlay);
    }

    #[test]
    fn test_strip_noop_when_no_env_key() {
        let overlay = json!({
            "permissions": {"allow": ["Bash"]}
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert!(result.stripped_paths.is_empty());
        assert_eq!(result.overlay, overlay);
    }

    #[test]
    fn test_strip_fails_when_root_not_object() {
        let overlay = json!([1, 2, 3]);
        let result = strip_protected_fields(&overlay);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_strip_ignores_non_object_env() {
        // env 是 string 时：不做深挖，保持原样
        let overlay = json!({
            "env": "not-an-object"
        });
        let result = strip_protected_fields(&overlay).unwrap();

        assert!(result.stripped_paths.is_empty());
        assert_eq!(result.overlay["env"], "not-an-object");
    }

    // --- merge_with_null_delete 测试 ---

    #[test]
    fn test_merge_scalar_overwrites() {
        let mut base = json!({"key": "old-value", "other": 1});
        let overlay = json!({"key": "new-value"});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base["key"], "new-value");
        assert_eq!(base["other"], 1);
    }

    #[test]
    fn test_merge_adds_new_key() {
        let mut base = json!({"existing": true});
        let overlay = json!({"new_key": "added"});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base["existing"], true);
        assert_eq!(base["new_key"], "added");
    }

    #[test]
    fn test_merge_null_deletes_key() {
        let mut base = json!({"to_delete": "value", "keep": "yes"});
        let overlay = json!({"to_delete": null});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert!(base.get("to_delete").is_none() || base["to_delete"].is_null());
        // serde_json Map::remove 实际删除 key
        assert!(base.as_object().unwrap().get("to_delete").is_none());
        assert_eq!(base["keep"], "yes");
    }

    #[test]
    fn test_merge_null_noop_when_key_not_in_base() {
        let mut base = json!({"keep": "yes"});
        let overlay = json!({"nonexistent": null});

        let result = merge_with_null_delete(&mut base, &overlay);
        assert!(result.is_ok());
        assert_eq!(base["keep"], "yes");
    }

    #[test]
    fn test_merge_array_replaces_whole() {
        let mut base = json!({"list": [1, 2, 3]});
        let overlay = json!({"list": [4, 5]});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base["list"], json!([4, 5]));
    }

    #[test]
    fn test_merge_object_recursive() {
        let mut base = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "old-key",
                "KEEP": "keep-me"
            }
        });
        let overlay = json!({
            "env": {
                "NEW_VAR": "new-value"
            }
        });

        merge_with_null_delete(&mut base, &overlay).unwrap();

        // 旧字段保留
        assert_eq!(base["env"]["ANTHROPIC_AUTH_TOKEN"], "old-key");
        assert_eq!(base["env"]["KEEP"], "keep-me");
        // 新字段加入
        assert_eq!(base["env"]["NEW_VAR"], "new-value");
    }

    #[test]
    fn test_merge_deep_nested_recursion() {
        let mut base = json!({
            "a": {
                "b": {
                    "c": "original",
                    "d": "keep"
                }
            }
        });
        let overlay = json!({
            "a": {
                "b": {
                    "c": "updated"
                }
            }
        });

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base["a"]["b"]["c"], "updated");
        assert_eq!(base["a"]["b"]["d"], "keep");
    }

    #[test]
    fn test_merge_fails_when_base_not_object() {
        let mut base = json!([1, 2, 3]);
        let overlay = json!({"key": "val"});

        let result = merge_with_null_delete(&mut base, &overlay);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_merge_base_non_object_key_replaced_by_overlay_object() {
        // base 中某个 key 是 string，overlay 该 key 是 object => 直接替换
        let mut base = json!({"key": "string-val"});
        let overlay = json!({"key": {"nested": true}});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base["key"], json!({"nested": true}));
    }

    #[test]
    fn test_merge_empty_overlay_no_side_effects() {
        let mut base = json!({
            "permissions": {"allow": ["Bash"]},
            "env": {"TOKEN": "val"},
            "scalar": 42
        });
        let original = base.clone();
        let overlay = json!({});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert_eq!(base, original);
    }

    #[test]
    fn test_merge_nested_null_deletes_deep_key() {
        let mut base = json!({"a": {"b": "val", "c": "keep"}, "top": true});
        let overlay = json!({"a": {"b": null}});

        merge_with_null_delete(&mut base, &overlay).unwrap();

        assert!(base["a"].as_object().unwrap().get("b").is_none());
        assert_eq!(base["a"]["c"], "keep");
        assert_eq!(base["top"], true);
    }

    #[test]
    fn test_merge_combined_rules() {
        // 综合测试：多种规则同时生效
        let mut base = json!({
            "permissions": {"allow": ["Bash"], "deny": ["Write"]},
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "old-key",
                "CUSTOM": "keep"
            },
            "delete_me": "gone",
            "scalar": "old"
        });

        let overlay = json!({
            "permissions": {"allow": ["Bash", "Read"]},  // array 整体替换
            "env": {"NEW_VAR": "added"},                  // 递归合并，不动其他 key
            "delete_me": null,                             // null 删除
            "scalar": "new"                               // scalar 覆盖
        });

        merge_with_null_delete(&mut base, &overlay).unwrap();

        // array 整体替换
        assert_eq!(base["permissions"]["allow"], json!(["Bash", "Read"]));
        // deny 保留（overlay 没提到）
        assert_eq!(base["permissions"]["deny"], json!(["Write"]));
        // env 递归合并
        assert_eq!(base["env"]["ANTHROPIC_AUTH_TOKEN"], "old-key");
        assert_eq!(base["env"]["CUSTOM"], "keep");
        assert_eq!(base["env"]["NEW_VAR"], "added");
        // null 删除
        assert!(base.as_object().unwrap().get("delete_me").is_none());
        // scalar 覆盖
        assert_eq!(base["scalar"], "new");
    }
}
