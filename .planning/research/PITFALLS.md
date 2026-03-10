# Domain Pitfalls

**Domain:** AI CLI Configuration Manager (Desktop App with iCloud Sync)
**Project:** CLIManager
**Researched:** 2026-03-10

---

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or fundamental architecture changes. Each of these was either directly observed in cc-switch or is a highly probable risk given the project's design constraints.

---

### Pitfall 1: Full-File Rewrite Destroys User's Other Settings

**What goes wrong:** When switching providers, the app serializes its internal model of the config and writes the entire file. Any fields the app does not track (user-added settings, comments, custom formatting) are silently destroyed.

**Why it happens:** It is the path of least resistance to serialize a struct to JSON and write it. The developer thinks "I'm writing the correct config" but forgets the file has more content than the app knows about.

**Consequences:**
- Users lose `~/.claude/settings.json` customizations (permissions, tool blocks, behavioral settings, custom model configs) every time they switch providers.
- Users stop trusting the tool and revert to manual editing.
- This was cc-switch's **number one reported bug**. The `write_json_file(&path, &settings)` call in `write_live_snapshot` (live.rs:646) replaces the entire `settings.json` with only the provider fields.

**Prevention:**
1. **Read-Modify-Write (surgical patch):** Read the current file, parse it, modify only the target fields (API key, model, base URL), write back the complete document.
2. **Field allowlist:** Explicitly enumerate which fields the app is authorized to touch. Never write fields outside this list.
3. **Snapshot diffing in tests:** Before/after test that creates a settings.json with extra fields, performs a switch, and asserts extra fields survive.
4. **Never serialize your internal model directly to the config file.** The internal Provider struct and the CLI config file are different schemas.

**Detection (warning signs):**
- Any code path where you serialize a data structure and write it as the entire file contents.
- `serde_json::to_string_pretty(provider)` flowing into a file write without a merge step.
- Tests that only check "did the target fields change" without checking "did other fields survive."

**Phase:** Must be solved in the **core architecture phase** (Phase 1). This is the project's raison d'etre. Get this wrong and nothing else matters.

---

### Pitfall 2: iCloud Sync of SQLite or Device-Local State

**What goes wrong:** Putting SQLite databases, device-specific settings, or "current active provider" state into an iCloud-synced directory causes conflicts, corruption, and "state bounce" across devices.

**Why it happens:** Developers want "everything synced" and place the entire app data directory under iCloud Drive. iCloud is eventually consistent with no cross-device file locking, no multi-file transaction guarantees, and no ordering guarantees for renames.

**Consequences (all observed in cc-switch):**
- **SQLite corruption:** iCloud syncs a half-written WAL or journal file. The other device opens a corrupted database. SQLite's `PRAGMA incremental_vacuum` and startup cleanup writes cause write contention even when the user does nothing.
- **State bounce:** Device A sets "current provider = X", syncs. Device B sets "current provider = Y", syncs. Both keep flipping back and forth as iCloud propagates conflicting writes.
- **Half-file propagation:** `settings.json` written via `truncate + write` (not atomic rename) means iCloud may sync the truncated-but-not-yet-written state -- an empty file -- to the other device.
- **Conflicted copies:** iCloud creates `settings (conflicted copy).json` which the app ignores, silently losing data.

**Prevention:**
1. **Strict data layer separation** (already in CLIManager's design -- validate it stays enforced):
   - `~/Library/Mobile Documents/...CLIManager/` (iCloud): Only per-provider JSON files. One file per provider. No SQLite. No device state.
   - `~/.cli-manager/` (local): Device-specific state (current active provider, device path overrides, cached data).
2. **Never put SQLite in a cloud-synced directory.** This is a well-documented anti-pattern. SQLite's own documentation warns against it.
3. **Never put "current active provider" in the sync layer.** Each device needs its own active selection.
4. **Use atomic rename (tmp + rename) for iCloud files**, not truncate + write. Even though iCloud doesn't guarantee cross-device atomicity, it significantly reduces the window for syncing a partial file.

**Detection (warning signs):**
- Any `.db` file path resolving into `~/Library/Mobile Documents/`.
- "Current provider" or "device ID" fields in a synced file.
- User reports of settings "jumping back" to a previous state.

**Phase:** Must be enforced in **architecture/data layer phase** (Phase 1). The sync vs. local boundary is a foundational decision.

---

### Pitfall 3: Multi-File Config Writes Without Transactional Semantics

**What goes wrong:** A single provider switch writes multiple files (e.g., Codex requires both `auth.json` AND `config.toml`). If the second write fails or iCloud syncs only one file to another device, the result is a "half-configured" state.

**Why it happens:** Some CLIs split their config across multiple files (Codex: auth.json + config.toml; Gemini: .env + settings.json). There is no filesystem-level transaction that can atomically update two files.

**Consequences:**
- Codex sees valid auth credentials but wrong model/config, or vice versa.
- cc-switch implemented a two-phase write with rollback for Codex (`write_codex_live_atomic` in codex_config.rs:62-109): write auth.json first, write config.toml second, rollback auth.json if second fails. But this is best-effort -- a crash between steps 1 and 2 leaves an inconsistent state.
- iCloud magnifies this: it may sync `auth.json` immediately but delay `config.toml` by seconds or minutes.

**Prevention:**
1. **For local writes (no iCloud involved):** Use the two-phase approach with rollback. It is imperfect but the best available option. Accept the tiny crash-window risk.
2. **Minimize multi-file writes:** If the CLI adapter for a tool requires writing N files, treat this as a single logical operation and document the ordering + rollback strategy.
3. **For iCloud-synced Provider files:** CLIManager's design of one-file-per-provider in iCloud avoids this for the sync layer. But the *live config write* (which patches the CLI's actual config files) still faces this problem for multi-file CLIs like Codex.
4. **Validation on read:** When reading a multi-file config, validate that all files are consistent. If they are not, warn the user rather than silently using partial state.

**Detection (warning signs):**
- Any CLI adapter that writes to more than one file path.
- Error handling that catches failures on the second write but does not rollback the first.
- Tests that only verify "did both files get written" but not "what happens if the second write fails."

**Phase:** Must be handled per-CLI-adapter in **adapter implementation phase** (Phase 2). Each adapter must document its file write strategy.

---

### Pitfall 4: Read-Modify-Write Race Condition with CLI

**What goes wrong:** CLIManager reads `settings.json`, modifies provider fields, and writes it back. Between the read and write, the CLI (or the user) also modifies `settings.json`. CLIManager's write overwrites the CLI's change.

**Why it happens:** Read-Modify-Write without file locking has an inherent TOCTOU (time-of-check/time-of-use) race window. The project explicitly accepts this risk ("CLI and CLIManager simultaneously writing the same file has very low probability"), which is a reasonable tradeoff -- but only if the window is minimized.

**Consequences:**
- User changes a setting in Claude Code CLI, then immediately switches provider in CLIManager. The CLI change is lost.
- This is much less severe than the full-file-rewrite pitfall (it only loses changes made during the tiny race window, not all non-tracked fields). But it can still frustrate users.

**Prevention:**
1. **Minimize the read-write gap:** Read the file, modify in memory, and write back as a single tight sequence. Do not hold the parsed content in memory for extended periods before writing.
2. **Do not cache the file content across operations.** Re-read the file fresh for every write operation.
3. **Surgical patch scope:** Only modify the exact fields being changed. The smaller the diff, the lower the probability of conflicting with a concurrent change to a *different* field.
4. **Consider file modification time check:** After reading, check `mtime` before writing. If the file changed between read and write, re-read and re-apply the patch. This is not bulletproof but shrinks the race window to near zero.
5. **Do NOT add full file locking.** The project decision to skip file locks is correct. File locks between a Tauri app and various CLI tools (which do not respect advisory locks) would add complexity with no benefit.

**Detection (warning signs):**
- File content stored in app state and reused across multiple write operations.
- Long-running async operations between file read and file write.
- Write operations that do not re-read the file first.

**Phase:** Core to the **surgical patch implementation** (Phase 1). The Read-Modify-Write loop is the most important code path in the project.

---

### Pitfall 5: CLI Config Format Changes Break the App Silently

**What goes wrong:** A CLI tool updates its config format (new fields, renamed fields, structural changes), and CLIManager either crashes, corrupts the config, or silently fails to switch providers.

**Why it happens:** CLIManager must parse config files it does not own. These files are defined by external projects (Claude Code, Codex) that can change their schema at any time without notice.

**Consequences:**
- Claude Code adds a new required field. CLIManager does not know about it. After a switch, Claude Code fails to start or behaves unexpectedly.
- Claude Code renames `settings.json` to something else (it already has a legacy `claude.json` path). CLIManager writes to the old path; Claude Code ignores it.
- Codex changes its `config.toml` structure. CLIManager's TOML merge logic produces invalid config.

**Prevention:**
1. **Defensive parsing:** Never assume a fixed schema. Parse as `serde_json::Value` (dynamic), not as a rigid struct. This is what the surgical patch approach naturally does -- it treats the file as an opaque document and only touches known fields.
2. **Preserve unknown fields:** The Read-Modify-Write approach inherently preserves fields CLIManager does not understand, which is exactly right.
3. **Version detection:** If the CLI has a version indicator in its config or can be queried (`claude --version`), use it to select the correct adapter behavior.
4. **Graceful degradation:** If a required target field path does not exist in the config, warn the user rather than crashing or creating it blindly.
5. **Integration tests against real CLI configs:** Maintain a set of sample config files from different CLI versions. Run tests against them to detect breakage early.
6. **Monitor CLI changelogs:** Claude Code and Codex are actively developed. Subscribe to their release notes.

**Detection (warning signs):**
- Hard-coded field paths without fallback logic.
- Strict struct deserialization (`serde(deny_unknown_fields)`).
- No tests using real-world config samples.
- User reports of "switching worked last week but broke after CLI update."

**Phase:** Ongoing concern, but the **adapter abstraction layer** (Phase 2) must be designed to accommodate this. The adapter interface should have a version/compatibility concept.

---

## Moderate Pitfalls

---

### Pitfall 6: FSEvents Debouncing and Infinite Loops

**What goes wrong:** CLIManager watches the iCloud sync directory via FSEvents. When a synced Provider file changes, it re-reads and re-patches the CLI config. But CLIManager's own write also triggers FSEvents, causing a re-patch loop.

**Prevention:**
1. **Self-write suppression:** Maintain a "recently written by me" set of file paths with timestamps. When an FSEvents notification arrives, check if the file was written by CLIManager within the last N milliseconds. If so, ignore the event.
2. **Debounce:** Coalesce rapid-fire FSEvents into a single handling event (e.g., 500ms debounce window).
3. **Content comparison:** Before re-patching, compare the new provider data with what was last applied. If identical, skip the write.
4. **Avoid watching the live config directory.** Only watch the iCloud Provider files directory. Never watch `~/.claude/` or `~/.codex/` -- that way lies infinite loops (write config -> detect change -> re-read -> re-write -> ...).

**Detection (warning signs):**
- CPU spikes after a provider switch.
- Log entries showing repeated read-write cycles.
- Config file `mtime` updating continuously.

**Phase:** **File watching implementation** (Phase 2-3). Must be designed into the watcher from the start.

---

### Pitfall 7: iCloud Drive Availability and Evicted Files

**What goes wrong:** iCloud Drive can "evict" files (replace them with a stub/placeholder) to save local disk space. Reading an evicted file returns an error or empty content instead of the expected provider data.

**Prevention:**
1. **Check file materialization status** before reading. On macOS, use `NSURL` resource values (`NSURLUbiquitousItemIsDownloadedKey`, `NSURLUbiquitousItemDownloadingStatusKey`) or check for the `.icloud` placeholder prefix in filenames.
2. **Trigger download** if the file is evicted: `NSFileManager.startDownloadingUbiquitousItem(at:)`.
3. **Graceful handling:** If a provider file is evicted and cannot be downloaded (offline), show the provider as "syncing" in the UI rather than crashing or showing corrupt data.
4. **Small file sizes help:** Per-provider JSON files are tiny (< 1KB). iCloud is unlikely to evict very small files, but the code should not assume this.

**Detection (warning signs):**
- Files in the iCloud container starting with `.` and ending with `.icloud` (placeholder format).
- Read errors only on machines with low disk space or "Optimize Mac Storage" enabled.

**Phase:** **iCloud sync layer** (Phase 2). Must be handled when implementing the sync directory reader.

---

### Pitfall 8: First-Launch Import Corrupts Existing Config

**What goes wrong:** On first launch, CLIManager scans existing CLI configs to create initial Provider entries. If this import logic is too aggressive or makes assumptions about the config structure, it can misparse fields or, worse, trigger a write-back that corrupts the original config.

**Prevention:**
1. **Import is read-only.** The first-launch import should ONLY read CLI configs and create Provider records in CLIManager's data store. It must NEVER write back to the CLI config files during import.
2. **Partial import is OK.** If a field cannot be parsed, skip it and warn the user. Do not fail the entire import.
3. **Preview before applying.** Show the user what was detected and let them confirm before creating providers.
4. **Backup original.** Before any operation that writes to a CLI config file, create a timestamped backup copy (e.g., `settings.json.bak.1710072000`).

**Detection (warning signs):**
- Import code paths that call any `write_*` function.
- Users reporting that their first launch "messed up" their CLI config.

**Phase:** **First-launch experience** (Phase 2-3). The import flow must be carefully isolated from the write flow.

---

### Pitfall 9: Provider Model Coupling to CLI Instead of Protocol

**What goes wrong:** The Provider data model is designed around specific CLI tools (e.g., "this is a Claude Code provider", "this is a Codex provider") instead of around API protocols (Anthropic API, OpenAI-compatible API). Adding a new CLI that uses the same API protocol requires duplicating provider data.

**Why it happens:** It feels natural to think "I'm configuring Claude Code" rather than "I'm configuring an Anthropic API endpoint." cc-switch fell into this pattern with per-app provider tables.

**Prevention:**
1. **Protocol-first modeling** (already planned): A Provider stores protocol-level data (API key, base URL, model, protocol type). CLI adapters translate this into CLI-specific config format.
2. **Adapter pattern:** Each CLI has an adapter that knows how to read/write its config format. The adapter takes a protocol-level Provider and produces CLI-specific config patches.
3. **Do not store CLI-specific config blobs in the Provider model.** cc-switch's `settings_config: Record<string, any>` stored the entire CLI config blob in the provider, making providers non-portable across CLIs.

**Detection (warning signs):**
- Provider struct containing fields like `claude_specific_setting` or `codex_auth_format`.
- Provider CRUD requiring an `app_type` parameter.
- Unable to share a single provider across multiple CLIs without duplication.

**Phase:** **Data model design** (Phase 1). This is a foundational modeling decision.

---

### Pitfall 10: Write Surface Amplification (One Switch = Many Writes)

**What goes wrong:** A single "switch provider" action triggers writes to multiple systems: live config files, device settings, UI state, and (in cc-switch's case) MCP configs, skills, and database. Each additional write increases the probability of failure, partial state, and iCloud conflicts.

**Why it happens:** Feature accumulation. As the app grows, more systems need to react to a provider switch. Without discipline, the switch code path becomes a cascade of side effects.

**Consequences (observed in cc-switch):**
- `sync_current_to_live` (live.rs:830-864) writes providers for ALL app types, then syncs ALL MCP configs, then syncs ALL skills. A single switch operation touches potentially dozens of files.
- Each file write is an iCloud conflict opportunity.
- If any write fails mid-cascade, the system is in a partially-switched state.

**Prevention:**
1. **v1: A switch only writes to the specific CLI's config files.** Nothing else. No MCP, no skills, no cascading writes.
2. **Explicit write budget:** Document "a switch operation writes to exactly N files" for each CLI adapter. If adding a feature increases N, that is a design review trigger.
3. **Lazy propagation:** If future features (MCP, skills) need to react to a switch, make them react lazily (on next access) rather than eagerly (write everything now).
4. **Separate "sync all" from "switch one":** cc-switch conflated these operations. CLIManager should have a distinct "re-sync everything" command (for recovery) separate from the normal switch path.

**Detection (warning signs):**
- Switch handler calling more than the target CLI's adapter.
- Write count per switch growing over time.
- Functions named `sync_all_*` in the switch code path.

**Phase:** **Architecture discipline** throughout all phases. Must be actively guarded against as features are added.

---

## Minor Pitfalls

---

### Pitfall 11: JSON Formatting and Comment Preservation

**What goes wrong:** Some users hand-edit their CLI config files and add formatting preferences (indentation, key ordering) or use JSON5/JSONC formats with comments. Standard JSON serialization destroys comments and may reorder keys.

**Prevention:**
1. **For JSON files (Claude Code):** Use a format-preserving JSON parser if possible, or accept that standard `serde_json` will normalize formatting. Document this behavior.
2. **For TOML files (Codex):** Use `toml_edit` (not `toml`) which preserves formatting, comments, and ordering. cc-switch already does this for TOML -- carry this forward.
3. **For JSON5 files:** If supporting CLIs that use JSON5 (like OpenClaw), use a JSON5 parser. Standard JSON parsers will reject JSON5 syntax.
4. **Minimize rewrite scope:** The surgical patch approach helps here -- if you only modify 2 fields, the rest of the file (including formatting around untouched fields) is preserved by the parse-modify-serialize cycle at the field level.

**Phase:** **Adapter implementation** (Phase 2). Each adapter must choose the right parser.

---

### Pitfall 12: API Key Exposure in Logs or Error Messages

**What goes wrong:** Debug logging, error messages, or crash reports include raw API keys from provider configs.

**Prevention:**
1. **Never log provider config values.** Log provider IDs and names, never API keys or tokens.
2. **Redact in error messages:** If an error includes config content, mask API keys (show first 4 + last 4 characters).
3. **No API keys in Tauri IPC error strings.** The `.map_err(|e| e.to_string())` pattern used in Tauri commands can leak internal details to the frontend console.

**Phase:** **All phases.** Establish a logging/error convention in Phase 1 and enforce it.

---

### Pitfall 13: Blocking the Main Thread with File I/O

**What goes wrong:** Tauri's command handlers run on the main thread by default. File I/O (especially reading configs, scanning directories for first-launch import, or waiting for iCloud file downloads) blocks the UI.

**Prevention:**
1. **Use `async` Tauri commands** for any operation involving file I/O.
2. **Offload heavy operations** (directory scanning, multi-file writes) to a background thread via `tokio::spawn_blocking` or Tauri's async command infrastructure.
3. **The switch operation should feel instant.** For a single JSON patch, the I/O is sub-millisecond. But if the switch cascades into multiple writes or triggers re-reads, it can stall.

**Phase:** **Implementation** (Phase 1-2). Choose async-by-default for Tauri commands from the start.

---

### Pitfall 14: Hardcoded Home Directory Assumptions

**What goes wrong:** Assuming `~` always resolves correctly, or that CLI config directories are always at their default locations. Some users customize config paths via environment variables or symlinks.

**Prevention:**
1. **Support directory overrides** (already planned in CLIManager). Let users specify custom paths for each CLI's config directory.
2. **Use `dirs::home_dir()` not `$HOME`** -- cc-switch learned this the hard way on Windows where `$HOME` may be injected by Git/MSYS and differ from the actual user directory.
3. **Resolve symlinks carefully.** If `~/.claude` is a symlink, ensure the app follows it correctly and does not write to the symlink file itself.

**Phase:** **Path resolution utilities** (Phase 1). Build a robust path module early.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Surgical patch implementation | Full-file rewrite sneaking in via "convenience" code path | Code review rule: any `write_json_file(path, data)` where `data` is not the result of a read-modify cycle is a bug. |
| Data model design | Coupling Provider to CLI instead of protocol | Enforce protocol-first modeling from day one. Provider has no `app_type` field. |
| CLI adapter: Claude Code | `settings.json` format changes in new Claude Code versions | Parse as `Value`, not as typed struct. Test against real config samples. |
| CLI adapter: Codex | Two-file write (auth.json + config.toml) partial failure | Implement two-phase write with rollback. Accept tiny crash-window risk. |
| iCloud sync layer | Evicted files, placeholder detection | Use NSFileManager APIs to check download status before reads. |
| iCloud sync layer | FSEvents infinite loop | Self-write suppression + debounce + content comparison. |
| File watching | Watching live config dirs causes feedback loops | Only watch iCloud Provider directory. Never watch CLI config directories. |
| First-launch import | Import accidentally writing back to CLI configs | Import function signature should not accept write handles. Make it structurally impossible to write during import. |
| Provider switch | Write surface growing as features accumulate | Explicit write budget per switch. Phase 1 switch = exactly 1-2 file writes per CLI. |
| i18n | Retrofitting i18n after shipping | Start with i18n framework in Phase 1. Much cheaper than adding later. |

---

## Lessons from cc-switch (Direct Observations)

These are not hypothetical -- they are bugs and design problems directly observed in the cc-switch codebase:

1. **`atomic_write` gives false confidence.** cc-switch implemented tmp-file + rename (config.rs:183-238) and called it "atomic write." This is locally atomic but does NOT help with: (a) iCloud sync consistency, (b) cross-file transaction consistency, or (c) the full-file-rewrite-destroying-other-fields problem. The name "atomic" made developers feel safe when they were not.

2. **`settings.json` written via truncate, not atomic rename.** The device settings file (settings.rs:405-439) used `OpenOptions(truncate(true))` instead of the tmp+rename pattern. iCloud could sync the truncated (empty) file to another device. Inconsistency in write strategies within the same codebase.

3. **SQLite startup writes caused phantom conflicts.** Even with no user action, opening cc-switch ran `cleanup_old_stream_check_logs`, `rollup_and_prune`, and `PRAGMA incremental_vacuum` (database/mod.rs:138-151). On two devices with iCloud-synced db, this created write contention at every launch.

4. **`sync_current_to_live` was a blast radius amplifier.** One switch triggered writes for ALL app types + ALL MCP configs + ALL skills (live.rs:830-864). What should have been a 1-file operation became a 10+ file operation.

5. **Provider data was a CLI-specific config blob.** `settings_config: Record<string, any>` stored the entire CLI config structure per provider (types.ts:14-15), making providers non-portable and the data model fragile.

---

## Sources

- cc-switch source code analysis: `cc-switch/src-tauri/src/config.rs`, `codex_config.rs`, `services/provider/live.rs`
- iCloud root cause analysis: `icloud-sync-root-cause-zh.md` (project document)
- cc-switch reference notes: `cc-switch-ref-notes-zh.md` (project document)
- CLIManager project spec: `.planning/PROJECT.md`
- Apple documentation on iCloud Drive file coordination (training data, MEDIUM confidence)
- SQLite documentation on network filesystems (training data, HIGH confidence -- well-documented anti-pattern)
