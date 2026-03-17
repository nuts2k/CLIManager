# Requirements: CLIManager

**Defined:** 2026-03-16
**Core Value:** 切换 Provider 时只做 surgical patch（精确修改凭据和模型字段），绝不重写配置文件的其他内容

## v2.5 Requirements — Claude 全局配置 Overlay

Requirements for milestone v2.5 Claude 全局配置 Overlay。

### Overlay 编辑与存储

- [x] **COVL-01**: 用户可以在 Settings → Advanced → Claude 小节编辑一段 JSON overlay（多行文本）
- [x] **COVL-02**: overlay 保存前必须通过 JSON 校验（root 必须为 object）；不合法时给出错误提示且拒绝保存
- [x] **COVL-03**: overlay 持久化文件优先写入 iCloud（可同步），iCloud 不可用时降级写入本地目录（功能可用但不同步）
- [x] **COVL-04**: 用户在 UI 中可以明确感知 overlay 当前存放位置（iCloud / 本地降级），并知道是否会跨设备同步

### Overlay 合并与保护字段

- [x] **COVL-05**: overlay 应用到 `~/.claude/settings.json` 时采用深度合并：object 递归合并、array 整体替换、scalar 覆盖
- [x] **COVL-06**: overlay 支持以 null 删除字段（例如 `{ "permissions": null }` 删除 settings.json 中对应 key）
- [x] **COVL-07**: overlay 不得覆盖 Provider/Proxy 管理的保护字段：`env.ANTHROPIC_AUTH_TOKEN`、`env.ANTHROPIC_BASE_URL`（这些字段最终值必须由 Provider/Proxy 写回并保持优先级）
- [x] **COVL-08**: overlay 中若包含保护字段，系统忽略这些字段且在 UI 侧提示“该字段由 Provider/Proxy 管理，不可覆盖”（提示形式可为说明文案或保存后 toast）

### 应用触发点（自动对齐）

- [ ] **COVL-09**: 用户保存 overlay 后立即 apply 到 `~/.claude/settings.json`（强一致）
- [ ] **COVL-10**: 应用启动时若 overlay 存在，后端执行一次 best-effort apply；失败不阻断启动但应记录日志/通知 UI
- [ ] **COVL-11**: iCloud 同步导致 overlay 文件变更时，文件 watcher 自动触发 apply（无需用户重启应用）

### 错误处理

- [ ] **COVL-12**: overlay 文件存在但 JSON 不合法时，apply 返回错误并在 UI 显示错误信息
- [x] **COVL-13**: `~/.claude/settings.json` 存在但不是合法 JSON 时，保持现有策略：拒绝写入并返回可见错误（不静默覆盖）

### 测试

- [ ] **COVL-14**: Rust 单元测试覆盖深度合并规则（递归合并/数组替换/标量覆盖/null 删除）
- [ ] **COVL-15**: Rust 测试覆盖保护字段永远优先（overlay 尝试覆盖 token/base_url 不得生效）
- [ ] **COVL-16**: 集成测试覆盖 ClaudeAdapter patch + overlay 注入（overlay 添加额外 env 字段不影响 surgical patch 行为）

## Future Requirements

None — v2.5 scope is focused.

## Out of Scope

| Feature | Reason |
|---------|--------|
| 通用 overlay 机制（适配 Codex 等所有 CLI） | v2.5 聚焦 Claude Code，避免过早抽象 |
| UI 表单化配置（非 JSON 编辑） | v2.5 先做 JSON overlay，后续再考虑表单化 |
| 复杂规则引擎（JSONPath/白名单路径等） | v2.5 仅做 JSON 片段深度合并 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| COVL-01 | Phase 24 | Complete |
| COVL-02 | Phase 24 | Complete |
| COVL-03 | Phase 24 | Complete |
| COVL-04 | Phase 24 | Complete |
| COVL-05 | Phase 24 | Complete |
| COVL-06 | Phase 24 | Complete |
| COVL-07 | Phase 24 | Complete |
| COVL-08 | Phase 24 | Complete |
| COVL-09 | Phase 24 | Pending |
| COVL-10 | Phase 24 | Pending |
| COVL-11 | Phase 24 | Pending |
| COVL-12 | Phase 24 | Pending |
| COVL-13 | Phase 24 | Complete |
| COVL-14 | Phase 25 | Pending |
| COVL-15 | Phase 25 | Pending |
| COVL-16 | Phase 25 | Pending |

**Coverage:**
- v2.5 requirements: 16 total
- Mapped to phases: 16
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-16*
*Last updated: 2026-03-16 after roadmap revision (Phase 24-25)*
