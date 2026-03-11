# Phase 2: Surgical Patch Engine - Research

**Researched:** 2026-03-11
**Domain:** Rust config file read-modify-write with format preservation (JSON + TOML)
**Confidence:** HIGH

## Summary

Phase 2 implements surgical patching of CLI config files when switching providers. Two CLI tools are targeted: Claude Code (`~/.claude/settings.json`) and Codex CLI (`~/.codex/auth.json` + `~/.codex/config.toml`). The core challenge is modifying only credential/model fields while preserving all other content, including JSON key ordering and TOML comments.

For JSON files, `serde_json` with the `preserve_order` feature provides key-order-preserving read-modify-write via `serde_json::Value` manipulation. For TOML, `toml_edit` is the standard Rust crate for format-preserving edits (comments, whitespace, ordering all survive). The existing codebase already has `serde_json`, `atomic_write`, and the `_in/_to` test isolation pattern established in Phase 1.

**Primary recommendation:** Use `serde_json` with `preserve_order` for JSON surgical patching (Value-level merge), `toml_edit` for TOML surgical patching (DocumentMut), and a Rust trait `CliAdapter` to unify the adapter interface across Claude and Codex CLIs.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- Claude Code CLI (`~/.claude/settings.json`): 写入 `api_key` + `base_url`，暂不写 `model`
- Codex CLI (`~/.codex/auth.json`): 写入 `api_key`
- Codex CLI (`~/.codex/config.toml`): 写入 `base_url`，暂不写 `model`
- `model_config`（haiku/sonnet/opus/reasoning_effort）v1 暂不处理
- 架构上预留扩展能力，未来可灵活添加更多字段映射
- 使用 Rust trait 抽象统一适配器接口（read_config / patch / backup / validate）
- ClaudeAdapter 和 CodexAdapter 各自实现 trait
- 备份文件统一存放在 `~/.cli-manager/backups/` 下，按 CLI 分子目录（claude/、codex/）
- 每次 patch 前都备份，使用时间戳后缀命名（如 `settings.json.2026-03-11T10-30-00.bak`）
- 最多保留 5 份备份，超过时自动删除最旧的
- 配置文件不存在时跳过备份（没有原文件可备份）
- Codex 顺序写入：先 auth.json，再 config.toml；config.toml 写入失败则从备份恢复 auth.json
- TOML 注释和原始格式必须保留（使用 TOML 感知的库解析修改）
- patch 前验证：原文件格式合法性（JSON/TOML 可解析）
- patch 后验证：结果文件格式合法性
- 只检查格式合法性，不检查字段语义正确性
- 目标 CLI 配置文件不存在时自动创建新文件，只包含需要 patch 的字段

### Claude's Discretion
- 具体的 Rust TOML 库选择（需支持保留注释和格式）
- trait 的具体方法签名和错误类型设计
- 时间戳格式的精确规范
- 备份清理的触发时机（patch 前清理还是 patch 后清理）

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PTCH-01 | Switching Provider only modifies credential and model fields in CLI config files | `serde_json::Value` merge for JSON; `toml_edit::DocumentMut` for TOML -- both support targeted field updates without touching other keys |
| PTCH-02 | All other content preserved intact after switching (including TOML comments) | `toml_edit` preserves comments/whitespace/ordering; `serde_json` with `preserve_order` preserves key ordering |
| PTCH-03 | Config files validated before and after patching; if validation fails, write aborted | JSON: `serde_json::from_str::<Value>` for parse validation; TOML: `DocumentMut::parse` for validation; both produce clear errors |
| PTCH-04 | Original config backed up before first write to each CLI config file | `fs::copy` to `~/.cli-manager/backups/{cli}/` with timestamp suffix; glob + sort for rotation |
| ADPT-01 | Claude Code adapter reads and patches `~/.claude/settings.json` | JSON Value-level merge of `env.ANTHROPIC_AUTH_TOKEN` and `env.ANTHROPIC_BASE_URL` fields |
| ADPT-02 | Codex adapter reads and patches `~/.codex/auth.json` + `config.toml` with two-phase write and rollback | Sequential write with rollback from backup; cc-switch reference code validates this pattern works |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_json | 1.x (with `preserve_order` feature) | JSON read-modify-write with key order preservation | Already in deps; `preserve_order` feature adds IndexMap backing for key ordering |
| toml_edit | 0.25.x | Format-preserving TOML editing (comments, whitespace, ordering) | De-facto standard for TOML editing in Rust (354M+ downloads); used by Cargo itself |
| chrono | 0.4.x | Timestamp generation for backup filenames | Already in deps |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3.x | Test isolation with temp directories | Already in dev-deps; used in Phase 1 tests |
| dirs | 5.x | Home directory resolution | Already in deps |
| thiserror | 2.x | Error type derivation | Already in deps; extend AppError for patch errors |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| toml_edit | toml (0.8) | `toml` crate does NOT preserve comments or formatting -- unsuitable for PTCH-02 |
| serde_json Value merge | serde_json struct deserialization | Struct deserialization would discard unknown fields; Value-level merge preserves everything |

**Installation (additions to Cargo.toml):**
```toml
[dependencies]
toml_edit = "0.25"

[dependencies.serde_json]
version = "1"
features = ["preserve_order"]
```

## Architecture Patterns

### Recommended Project Structure
```
src-tauri/src/
├── adapter/
│   ├── mod.rs           # CliAdapter trait + shared utilities (backup, validate)
│   ├── claude.rs        # ClaudeAdapter impl
│   └── codex.rs         # CodexAdapter impl
├── error.rs             # Extended with patch-specific variants
├── provider.rs          # (existing) Provider struct
├── storage/             # (existing) iCloud + local storage
└── commands/
    └── provider.rs      # (existing) Tauri commands -- will call adapters in Phase 3
```

### Pattern 1: CliAdapter Trait
**What:** A trait that unifies the read-config/patch/backup/validate interface
**When to use:** Every CLI adapter implements this trait

```rust
use std::path::Path;
use crate::error::AppError;
use crate::provider::Provider;

/// Result of a patch operation
pub struct PatchResult {
    pub files_written: Vec<String>,
    pub backups_created: Vec<String>,
}

pub trait CliAdapter {
    /// Human-readable name for error messages
    fn cli_name(&self) -> &str;

    /// Patch CLI config files with the given provider's credentials
    fn patch(&self, provider: &Provider) -> Result<PatchResult, AppError>;
}
```

Note: backup, validate, and rollback are internal implementation details of each adapter's `patch` method, not separate trait methods. This keeps the trait simple and allows adapters to have different transactional semantics (Claude = single file, Codex = two-phase with rollback).

### Pattern 2: JSON Surgical Patch (Claude Code)
**What:** Read JSON as `serde_json::Value`, merge only target fields, write back
**When to use:** `settings.json` patching

```rust
use serde_json::Value;

/// Merge provider fields into existing settings JSON.
/// Only touches env.ANTHROPIC_AUTH_TOKEN and env.ANTHROPIC_BASE_URL.
fn patch_claude_json(existing: &str, api_key: &str, base_url: &str) -> Result<String, AppError> {
    let mut root: Value = serde_json::from_str(existing)?;

    // Ensure env object exists
    let env = root
        .as_object_mut()
        .ok_or_else(|| /* error */)?
        .entry("env")
        .or_insert_with(|| Value::Object(Default::default()));

    let env_obj = env.as_object_mut().ok_or_else(|| /* error */)?;
    env_obj.insert("ANTHROPIC_AUTH_TOKEN".into(), Value::String(api_key.into()));
    env_obj.insert("ANTHROPIC_BASE_URL".into(), Value::String(base_url.into()));

    serde_json::to_string_pretty(&root).map_err(Into::into)
}
```

### Pattern 3: TOML Surgical Patch (Codex config.toml)
**What:** Parse TOML as `DocumentMut`, set only target keys, write back preserving comments
**When to use:** `config.toml` patching

```rust
use toml_edit::DocumentMut;

fn patch_codex_toml(existing: &str, base_url: &str) -> Result<String, AppError> {
    let mut doc = existing.parse::<DocumentMut>()
        .map_err(|e| /* toml parse error */)?;

    doc["base_url"] = toml_edit::value(base_url);

    Ok(doc.to_string())
}
```

### Pattern 4: _in/_to Test Isolation (from Phase 1)
**What:** Internal functions accept explicit paths; public functions use default paths
**When to use:** All adapter code for testability

```rust
// Public: uses default paths
pub fn patch_claude(provider: &Provider) -> Result<PatchResult, AppError> {
    let config_dir = get_claude_config_dir();
    let backup_dir = get_backup_dir("claude");
    patch_claude_in(&config_dir, &backup_dir, provider)
}

// Internal: explicit paths for testing
fn patch_claude_in(
    config_dir: &Path,
    backup_dir: &Path,
    provider: &Provider,
) -> Result<PatchResult, AppError> {
    // all logic here, fully testable with tempdir
}
```

### Anti-Patterns to Avoid
- **Whole-file rewrite via struct serialization:** Never deserialize settings.json into a typed struct and serialize back -- this destroys unknown fields. Always use `Value`-level merge.
- **Using `toml` crate for editing:** The `toml` crate (as opposed to `toml_edit`) strips comments and reformats -- this violates PTCH-02.
- **Shared rollback logic across adapters:** Claude is single-file, Codex is two-file. Don't force a generic transaction abstraction -- let each adapter handle its own rollback semantics.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| TOML format-preserving edit | Custom regex/string replacement on TOML | `toml_edit::DocumentMut` | TOML syntax is complex (multiline strings, inline tables, dotted keys); regex will break |
| JSON key order preservation | Manual string manipulation | `serde_json` with `preserve_order` feature | One feature flag gives you IndexMap-backed maps |
| Atomic file write | Custom temp-file + rename | Existing `atomic_write` from `storage/mod.rs` | Already tested and working in Phase 1 |
| Timestamp formatting | Manual string formatting | `chrono::Local::now().format(...)` | Already in deps, handles edge cases |

**Key insight:** The two critical "don't hand-roll" items are TOML comment preservation and JSON key ordering. Both have mature, battle-tested solutions in the Rust ecosystem. Hand-rolling either would be a source of subtle bugs.

## Common Pitfalls

### Pitfall 1: serde_json Default Map Loses Key Order
**What goes wrong:** Without `preserve_order`, serde_json uses BTreeMap which alphabetizes keys. User's `settings.json` comes back with keys reordered.
**Why it happens:** `preserve_order` is an opt-in feature, not the default.
**How to avoid:** Enable `preserve_order` feature in Cargo.toml: `serde_json = { version = "1", features = ["preserve_order"] }`
**Warning signs:** Keys in output JSON appear in alphabetical order instead of original order.

### Pitfall 2: toml_edit Index Operator Creates Implicit Tables
**What goes wrong:** Accessing `doc["nonexistent"]["key"]` with toml_edit doesn't fail -- it creates implicit tables, potentially polluting the document.
**Why it happens:** toml_edit's Index impl returns `Item::None` for missing keys, and assignment through None creates the path.
**How to avoid:** For reads, use `.get()` which returns `Option`. For writes, only assign to keys you intend to create.
**Warning signs:** Extra empty tables appearing in the TOML output.

### Pitfall 3: Codex auth.json Has Complex Structure
**What goes wrong:** Assuming auth.json is a simple `{"api_key": "..."}`. Real Codex auth.json has `auth_mode`, `tokens` (with OAuth fields), `OPENAI_API_KEY`, etc.
**Why it happens:** Codex supports multiple auth modes (chatgpt OAuth, API key).
**How to avoid:** Only patch `OPENAI_API_KEY` field at the top level. Use Value-level merge to leave all other fields (`auth_mode`, `tokens`, etc.) untouched. When creating a new auth.json, include only the `OPENAI_API_KEY` field.
**Warning signs:** OAuth tokens getting overwritten, `auth_mode` changing unexpectedly.

### Pitfall 4: Backup Directory Race Condition
**What goes wrong:** Two rapid switches could both try to create backups simultaneously, potentially exceeding the 5-backup limit or causing naming collisions.
**Why it happens:** Timestamp granularity (seconds) may not distinguish rapid operations.
**How to avoid:** Use millisecond or nanosecond precision in timestamp. The 5-backup cleanup is best-effort (if it fails, just log and continue).
**Warning signs:** More than 5 backup files accumulating.

### Pitfall 5: Claude settings.json Field Naming
**What goes wrong:** Using wrong env var names. Claude Code uses `ANTHROPIC_AUTH_TOKEN` (not `ANTHROPIC_API_KEY`) and `ANTHROPIC_BASE_URL`.
**Why it happens:** Multiple naming conventions exist across Anthropic's tooling.
**How to avoid:** Verified from actual `~/.claude/settings.json` on the developer's machine: the field names are `env.ANTHROPIC_AUTH_TOKEN` and `env.ANTHROPIC_BASE_URL`. Also confirmed by cc-switch reference code.
**Warning signs:** Claude Code not picking up the switched provider credentials.

### Pitfall 6: Codex config.toml Field Naming
**What goes wrong:** Using wrong key name in config.toml.
**Why it happens:** Codex config structure is not heavily documented.
**How to avoid:** Verified from actual `~/.codex/config.toml`: top-level keys include `model`, `service_tier`, `[projects.*]` tables, `[notice.*]` tables. For base_url, the field to patch needs verification -- it may not be a standard top-level key. The CONTEXT.md says to write `base_url`, so use that key name.
**Warning signs:** Codex not connecting to the expected endpoint.

## Code Examples

### Complete Claude Adapter Patch Flow
```rust
// Source: Verified from existing codebase patterns + serde_json docs

fn patch_claude_in(
    config_dir: &Path,
    backup_dir: &Path,
    provider: &Provider,
) -> Result<PatchResult, AppError> {
    let settings_path = config_dir.join("settings.json");
    let mut backups_created = Vec::new();

    // Step 1: Read existing or create empty
    let existing = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        // Pre-patch validation
        serde_json::from_str::<Value>(&content)?;
        content
    } else {
        "{}".to_string()
    };

    // Step 2: Backup (only if file exists)
    if settings_path.exists() {
        let backup_path = create_backup(&settings_path, backup_dir)?;
        backups_created.push(backup_path);
        rotate_backups(backup_dir, 5)?;
    }

    // Step 3: Surgical patch
    let patched = patch_claude_json(&existing, &provider.api_key, &provider.base_url)?;

    // Step 4: Post-patch validation
    serde_json::from_str::<Value>(&patched)?;

    // Step 5: Atomic write
    atomic_write(&settings_path, patched.as_bytes())?;

    Ok(PatchResult {
        files_written: vec![settings_path.display().to_string()],
        backups_created,
    })
}
```

### Backup with Timestamp and Rotation
```rust
use chrono::Local;

fn create_backup(source: &Path, backup_dir: &Path) -> Result<String, AppError> {
    fs::create_dir_all(backup_dir)?;
    let filename = source.file_name().unwrap().to_string_lossy();
    let timestamp = Local::now().format("%Y-%m-%dT%H-%M-%S%.3f");
    let backup_name = format!("{}.{}.bak", filename, timestamp);
    let backup_path = backup_dir.join(&backup_name);
    fs::copy(source, &backup_path)?;
    Ok(backup_path.display().to_string())
}

fn rotate_backups(backup_dir: &Path, max_count: usize) -> Result<(), AppError> {
    let mut backups: Vec<_> = fs::read_dir(backup_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "bak"))
        .collect();
    backups.sort_by_key(|e| e.file_name());
    while backups.len() > max_count {
        let oldest = backups.remove(0);
        let _ = fs::remove_file(oldest.path()); // best-effort cleanup
    }
    Ok(())
}
```

### Codex Two-Phase Write with Rollback
```rust
fn patch_codex_in(
    config_dir: &Path,
    backup_dir: &Path,
    provider: &Provider,
) -> Result<PatchResult, AppError> {
    let auth_path = config_dir.join("auth.json");
    let config_path = config_dir.join("config.toml");

    // Backup both files first (if they exist)
    // ... (same pattern as Claude)

    // Phase 1: Patch and write auth.json
    let auth_patched = patch_codex_auth_json(&auth_path, &provider.api_key)?;
    atomic_write(&auth_path, auth_patched.as_bytes())?;

    // Phase 2: Patch and write config.toml (rollback auth.json on failure)
    let toml_result = patch_and_write_codex_toml(&config_path, &provider.base_url);
    if let Err(e) = toml_result {
        // Rollback: restore auth.json from backup
        restore_from_backup(&auth_path, backup_dir)?;
        return Err(e);
    }

    Ok(PatchResult { /* ... */ })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `toml` crate for editing | `toml_edit` for format-preserving edits | toml_edit stable since 2022+ | Comments and formatting survive edits |
| serde_json BTreeMap default | serde_json `preserve_order` feature | Available since serde_json 1.0 | Key insertion order preserved |
| cc-switch whole-file rewrite | CLIManager surgical Value-level merge | This project (2026) | Only target fields change, everything else preserved |

**Deprecated/outdated:**
- `toml` crate for editing use cases: Use `toml_edit` instead when format preservation matters
- cc-switch `settings_config` pattern (stores entire config blob per provider): CLIManager deliberately avoids this

## Open Questions

1. **Codex config.toml `base_url` field**
   - What we know: CONTEXT.md says to write `base_url` to config.toml
   - What's unclear: The actual `~/.codex/config.toml` on this machine has `model` and `service_tier` at top level, plus `[projects]` and `[notice]` tables, but no `base_url` field visible. Codex may use an env var or different key name.
   - Recommendation: Proceed with `base_url` as specified in CONTEXT.md. If Codex doesn't read it, this is a config-schema issue to fix in a future iteration, not a Phase 2 architectural problem.

2. **Codex auth.json `OPENAI_API_KEY` field**
   - What we know: The actual auth.json has `auth_mode: "chatgpt"`, `OPENAI_API_KEY: null`, and OAuth `tokens` block
   - What's unclear: When using API key auth (not ChatGPT OAuth), what's the expected auth.json structure?
   - Recommendation: Patch only `OPENAI_API_KEY` at the Value level, leaving all other fields intact. For new files, create `{"OPENAI_API_KEY": "<key>"}`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in Rust test framework) |
| Config file | `src-tauri/Cargo.toml` (test deps: `tempfile = "3"`) |
| Quick run command | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full suite command | `cargo test --manifest-path src-tauri/Cargo.toml` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PTCH-01 | Only credential fields modified | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter -- --nocapture` | No -- Wave 0 |
| PTCH-02 | Other content preserved (TOML comments, JSON keys) | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter -- --nocapture` | No -- Wave 0 |
| PTCH-03 | Pre/post validation; invalid state aborts | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter -- --nocapture` | No -- Wave 0 |
| PTCH-04 | Backup before first write, rotation | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter -- --nocapture` | No -- Wave 0 |
| ADPT-01 | Claude adapter patches settings.json | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter::claude -- --nocapture` | No -- Wave 0 |
| ADPT-02 | Codex adapter two-phase write + rollback | unit | `cargo test --manifest-path src-tauri/Cargo.toml adapter::codex -- --nocapture` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Per wave merge:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `src-tauri/src/adapter/mod.rs` -- trait definition + backup/rotate utilities + tests
- [ ] `src-tauri/src/adapter/claude.rs` -- Claude adapter + tests
- [ ] `src-tauri/src/adapter/codex.rs` -- Codex adapter + tests
- [ ] Add `toml_edit = "0.25"` to Cargo.toml dependencies
- [ ] Enable `preserve_order` feature on `serde_json` in Cargo.toml

## Sources

### Primary (HIGH confidence)
- Actual `~/.claude/settings.json` on developer machine -- verified field names `env.ANTHROPIC_AUTH_TOKEN`, `env.ANTHROPIC_BASE_URL`
- Actual `~/.codex/config.toml` and `~/.codex/auth.json` on developer machine -- verified file structures
- [toml_edit crates.io](https://crates.io/crates/toml_edit) -- version 0.25.4, format-preserving TOML editing
- [toml_edit docs.rs](https://docs.rs/toml_edit) -- DocumentMut API, comment preservation behavior
- [serde_json preserve_order docs](https://docs.rs/serde_json/latest/serde_json/struct.Map.html) -- IndexMap backing for key order
- cc-switch reference code (`cc-switch/src-tauri/src/codex_config.rs`) -- two-phase write with rollback pattern

### Secondary (MEDIUM confidence)
- [serde_json preserve_order GitHub issue](https://github.com/serde-rs/json/issues/54) -- feature design and behavior details
- [toml_edit tutorial](https://generalistprogrammer.com/tutorials/toml_edit-rust-crate-guide) -- usage patterns

### Tertiary (LOW confidence)
- Codex `base_url` field name in config.toml -- not verified in actual Codex documentation, proceeding with CONTEXT.md specification

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- `serde_json` already in use, `toml_edit` is the de-facto standard (354M+ downloads, used by Cargo)
- Architecture: HIGH -- trait-based adapter pattern well-established; `_in/_to` pattern proven in Phase 1
- Pitfalls: HIGH -- verified against actual config files on developer machine and cc-switch reference code

**Research date:** 2026-03-11
**Valid until:** 2026-04-11 (stable libraries, unlikely to change)
