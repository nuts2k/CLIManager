# Phase 13: 端到端验证 - Context

**Gathered:** 2026-03-14
**Status:** Ready for planning

<domain>
## Phase Boundary

完整发版流程端到端验证：release script → CI build → updater check → download → install → relaunch。不写新功能代码，验证 Phase 12 所有产出在真实环境中联通。唯一允许的代码变更：修复验证过程中发现的 bug 和将 release.yml 改为自动 Publish。

</domain>

<decisions>
## Implementation Decisions

### 发版触发方式
- 用 `/ship patch` 真实发版，验证完整工作流（不手动 tag）
- 首次验证版本号：v0.2.1（/ship patch 从当前 0.2.0 bump）
- GitHub Secrets 已配置（TAURI_SIGNING_PRIVATE_KEY）
- Release Draft 改为自动 Publish（需修改 release.yml 的 releaseDraft 配置）

### 更新流测试方案
- 两次发版测试：先发 v0.2.1 并安装作为基准，再发 v0.2.2 测试更新流
- 基准版本通过 CI 产出的 DMG 安装到 /Applications（完整模拟用户体验）
- 快速验证：弹窗出现 → 点击更新 → 下载完成 → 安装成功即通过，不做详细截图记录
- relaunch() 如果报 os error 1（Issue #2273），降级为手动重启也算通过

### 双架构验证策略
- 本机架构：aarch64 (Apple Silicon)，完整端到端测试
- x86_64 架构：仅检查 CI 构建成功 + Release 中有对应产物，不实际安装
- latest.json：检查包含两个架构的下载 URL 和签名字段完整性
- x86_64 CI job 成功即视为该架构通过

### 失败回退处理
- CI DMG 打包随机失败（Bug #13804）：重跑 CI，不立即排查
- app 检测不到更新：排查 latest.json URL、endpoints 配置、网络连接，修复后发新版重测
- 最多重试 2 次（v0.2.2 和 v0.2.3），再不行记录问题待解决
- 整体通过标准（实用标准）：aarch64 完整走通 + x86_64 CI 成功，允许 relaunch 降级为手动重启

### Claude's Discretion
- 验证步骤的具体执行顺序
- 排查问题时的调试手段
- release.yml 修改的具体实现方式（releaseDraft: false 或其他方案）

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.github/workflows/release.yml`：Phase 12 创建的 CI 流水线，需改 releaseDraft 配置
- `.claude/commands/ship.md`：Phase 12 创建的发版技能，用于触发验证
- `src/components/updater/useUpdater.ts`：更新检测 hook
- `src/components/updater/UpdateDialog.tsx`：更新弹窗组件

### Established Patterns
- `/ship patch|minor|major`：bump Cargo.toml → CHANGELOG → commit → tag → push
- CI tag 触发：`v[0-9]*.[0-9]*.[0-9]*` 格式
- updater endpoints：指向 GitHub Releases latest.json

### Integration Points
- Cargo.toml version → /ship bump → git tag → CI trigger → Release → latest.json → updater check
- 完整链路依赖所有环节正确接线

</code_context>

<specifics>
## Specific Ideas

- 验证流程是"两次发版"模式：v0.2.1 建立基准 → v0.2.2 触发更新
- 对 relaunch 失败宽容：ad-hoc 签名下这是已知限制，手动重启可接受

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 13-e2e-verification*
*Context gathered: 2026-03-14*
