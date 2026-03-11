# Requirements: CLIManager

**Defined:** 2026-03-10
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Provider Management

- [x] **PROV-01**: User can create a new Provider with name, API key, base URL, and model
- [x] **PROV-02**: User can view all Providers in a list with clear display
- [x] **PROV-03**: User can edit an existing Provider's settings
- [x] **PROV-04**: User can delete a Provider
- [x] **PROV-05**: User can see which Provider is currently active for each CLI at a glance
- [x] **PROV-06**: User can switch active Provider with one click (< 1 second)

### Surgical Patch

- [x] **PTCH-01**: Switching Provider only modifies credential and model fields in CLI config files
- [x] **PTCH-02**: All other content in CLI config files is preserved intact after switching (including TOML comments)
- [x] **PTCH-03**: Config files are validated before and after patching; if validation fails, write is aborted
- [x] **PTCH-04**: Original config is backed up before first write to each CLI config file

### CLI Adapters

- [x] **ADPT-01**: Claude Code adapter reads and patches `~/.claude/settings.json` (credential + model fields only)
- [x] **ADPT-02**: Codex adapter reads and patches `~/.codex/auth.json` + `config.toml` with two-phase write and rollback
- [x] **ADPT-03**: Provider data model uses protocol type (Anthropic, OpenAI-compatible, etc.) for future CLI reuse

### iCloud Sync

- [x] **SYNC-01**: Provider data stored as individual JSON files in iCloud Drive directory
- [x] **SYNC-02**: Device-local settings (active provider, path overrides) stored in `~/.cli-manager/local.json`, never synced
- [x] **SYNC-03**: File watcher (FSEvents) monitors iCloud sync directory for Provider file changes
- [ ] **SYNC-04**: UI automatically refreshes when Provider files are added, modified, or deleted via sync
- [x] **SYNC-05**: When active Provider is modified by sync, CLI configs are automatically re-patched with updated values

### Onboarding

- [ ] **ONBD-01**: First launch scans existing `~/.claude/` and `~/.codex/` configs and creates initial Providers
- [ ] **ONBD-02**: User can also manually create Providers from scratch at any time

### i18n

- [x] **I18N-01**: UI supports Chinese and English with all text externalized
- [x] **I18N-02**: Default language is Chinese
- [x] **I18N-03**: User can switch language in settings

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Tray

- **TRAY-01**: System tray menu shows providers grouped by CLI
- **TRAY-02**: User can switch provider directly from tray menu

### MCP Management

- **MCP-01**: User can manage MCP server configurations
- **MCP-02**: MCP configs can be toggled per CLI

### Additional CLI Support

- **CLI-01**: Gemini CLI adapter
- **CLI-02**: OpenCode adapter (multi-protocol)
- **CLI-03**: OpenClaw adapter

### Local Proxy

- **PRXY-01**: Local proxy server for request forwarding
- **PRXY-02**: Active provider parameter hot-reload via file watcher

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Proxy / Failover / Usage tracking | cc-switch 中最臃肿的模块（50+ schema fields），与配置切换核心价值无关 |
| Skills management | 编辑器/IDE 关注点，非 Provider 配置管理 |
| Prompts management | 同上，与切换 Provider 主线弱耦合 |
| Session manager | 与 Provider 切换正交的独立功能 |
| Deep link import | 非 MVP 必需，手动创建 + 自动导入已覆盖 onboarding |
| WebDAV / custom sync | iCloud Drive 足够，架构天然支持未来扩展 |
| Provider categories / icons / partner badges | 商业平台复杂度，用户只有 3-10 个 Provider |
| Usage query scripts | 用户可在 Provider 的 web dashboard 查看 |
| Endpoint speed testing | 用户从 CLI 行为即可感知 |
| Universal Provider abstraction | 协议类型建模已实现复用，无需额外抽象层 |
| Whole-file atomic write | 这正是 cc-switch 的问题根源，CLIManager 用 surgical patch 替代 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| PROV-01 | Phase 3 | Complete |
| PROV-02 | Phase 3 | Complete |
| PROV-03 | Phase 3 | Complete |
| PROV-04 | Phase 3 | Complete |
| PROV-05 | Phase 3 | Complete |
| PROV-06 | Phase 3 | Complete |
| PTCH-01 | Phase 2 | Complete |
| PTCH-02 | Phase 2 | Complete |
| PTCH-03 | Phase 2 | Complete |
| PTCH-04 | Phase 2 | Complete |
| ADPT-01 | Phase 2 | Complete |
| ADPT-02 | Phase 2 | Complete |
| ADPT-03 | Phase 1 | Complete |
| SYNC-01 | Phase 1 | Complete |
| SYNC-02 | Phase 1 | Complete |
| SYNC-03 | Phase 4 | Complete |
| SYNC-04 | Phase 4 | Pending |
| SYNC-05 | Phase 4 | Complete |
| ONBD-01 | Phase 5 | Pending |
| ONBD-02 | Phase 5 | Pending |
| I18N-01 | Phase 3 | Complete |
| I18N-02 | Phase 3 | Complete |
| I18N-03 | Phase 3 | Complete |

**Coverage:**
- v1 requirements: 23 total
- Mapped to phases: 23
- Unmapped: 0

---
*Requirements defined: 2026-03-10*
*Last updated: 2026-03-10 after roadmap creation*
