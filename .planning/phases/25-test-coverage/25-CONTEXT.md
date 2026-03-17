# Phase 25: 「测试覆盖」 - Context

**Gathered:** 2026-03-17
**Status:** Ready for planning

<domain>
## Phase Boundary

关键 overlay 注入行为具备可重复验证的自动化测试，防止深度合并/保护字段优先级/ClaudeAdapter surgical patch 回归。覆盖 COVL-14、COVL-15、COVL-16 三项需求。

</domain>

<decisions>
## Implementation Decisions

### 测试策略：审计补缺
- Phase 24 实现过程中已内嵌约 25 个 overlay 相关测试（json_merge.rs ~17 个 + claude.rs ~8 个 overlay 集成测试）
- Phase 25 不从零重写，而是跑一遍 cargo test、逐条审计 COVL-14/15/16 的覆盖情况，仅补充缺失的边界用例
- 若审计确认无缺口，直接写 VERIFICATION.md 记录审计结果并标记完成，不强行凑测试

### 集成测试粒度：模块内集成
- 在 claude.rs mod tests 内补充端到端场景（已有 overlay_path_override 注入模式）
- 验证 patch() 方法通过文件系统读写的完整链路
- 不创建独立 tests/ 集成测试文件

### 边界用例侧重：核心回归防护
- 只补与三项需求直接相关的缺失场景，例如：
  - overlay 为空对象 `{}` 时 merge 无副作用
  - 嵌套路径的 null 删除（如 `{"a": {"b": null}}`）
  - overlay + clear() 交互（clear 后保护字段移除，overlay 字段是否保留）
  - 多次序贯 patch（先 patch with overlay，再换 provider patch）保护字段始终正确
- 不追求极端场景（超大 JSON、数百层嵌套、Unicode key 等）

### Claude's Discretion
- 审计过程中发现的具体缺口数量和补充测试的具体用例设计
- 测试命名和组织方式（沿用现有 test_ 前缀命名惯例）

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `json_merge.rs` mod tests: 7 个 strip_protected_fields 测试 + 10 个 merge_with_null_delete 测试（含 test_merge_combined_rules 综合测试）
- `claude.rs` mod tests: test_provider() helper + write_overlay() helper + overlay_path_override 注入模式
- tempfile crate 用于文件系统隔离测试

### Established Patterns
- 模块内 `#[cfg(test)] mod tests` 组织方式，所有 Rust 模块统一使用
- `ClaudeAdapter::new_with_paths_and_overlay()` 构造函数支持测试注入
- `serde_json::json!` 宏构造测试数据
- `TempDir` 隔离每个测试的文件系统副作用

### Integration Points
- `patch_claude_json()` 是核心 surgical patch 函数（私有，仅通过 ClaudeAdapter::patch() 间接测试）
- `strip_protected_fields()` 和 `merge_with_null_delete()` 是公开函数，可直接单元测试
- COVL-14 对应 `merge_with_null_delete()` 测试
- COVL-15 对应 `strip_protected_fields()` 测试 + `patch_claude_json()` 端到端保护字段验证
- COVL-16 对应 `ClaudeAdapter::patch()` overlay 集成测试

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 25-test-coverage*
*Context gathered: 2026-03-17*
