# Phase 13: 端到端验证 - Research

**Researched:** 2026-03-14
**Domain:** Tauri 发版流程端到端验证（release script → CI → updater → install → relaunch）
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- 用 `/ship patch` 真实发版，验证完整工作流（不手动 tag）
- 首次验证版本号：v0.2.1（/ship patch 从当前 0.2.0 bump）
- GitHub Secrets 已配置（TAURI_SIGNING_PRIVATE_KEY）
- Release Draft 改为自动 Publish（需修改 release.yml 的 releaseDraft 配置）
- 两次发版测试：先发 v0.2.1 并安装作为基准，再发 v0.2.2 测试更新流
- 基准版本通过 CI 产出的 DMG 安装到 /Applications（完整模拟用户体验）
- 快速验证：弹窗出现 → 点击更新 → 下载完成 → 安装成功即通过，不做详细截图记录
- relaunch() 如果报 os error 1（Issue #2273），降级为手动重启也算通过
- 本机架构：aarch64 (Apple Silicon)，完整端到端测试
- x86_64 架构：仅检查 CI 构建成功 + Release 中有对应产物，不实际安装
- latest.json：检查包含两个架构的下载 URL 和签名字段完整性
- CI DMG 打包随机失败（Bug #13804）：重跑 CI，不立即排查
- app 检测不到更新：排查 latest.json URL、endpoints 配置、网络连接，修复后发新版重测
- 最多重试 2 次（v0.2.2 和 v0.2.3），再不行记录问题待解决
- 整体通过标准（实用标准）：aarch64 完整走通 + x86_64 CI 成功，允许 relaunch 降级为手动重启

### Claude's Discretion

- 验证步骤的具体执行顺序
- 排查问题时的调试手段
- release.yml 修改的具体实现方式（releaseDraft: false 或其他方案）

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CICD-01 | 三段式 v*.*.* tag 推送触发 GitHub Actions 构建 | /ship patch 产生 v0.2.1 tag → 触发 CI |
| CICD-02 | macOS 双架构构建（aarch64 + x86_64），生成 DMG | release.yml matrix 已配置；验证两个 job 均 green |
| CICD-03 | 构建产物（DMG + .app.tar.gz + .sig + latest.json）自动上传到 GitHub Release | releaseDraft: false 后 Release 即时可见；检查产物完整性 |
| SIGN-01 | CI 构建时 macOS ad-hoc 代码签名（APPLE_SIGNING_IDENTITY="-"） | Bug #13804 随机失败已知；重跑 CI 是标准处理方式 |
| SIGN-02 | 生成 updater Ed25519 签名密钥对并安全备份 | 已完成（Phase 12-01）；验证 latest.json 中 .sig 存在即可 |
| SIGN-03 | 私钥存储到 GitHub Secrets，公钥写入 tauri.conf.json | 已完成；CI 能产出 .sig 文件即证明密钥配置正确 |
| UPD-01 | 集成 tauri-plugin-updater + tauri-plugin-process | 已完成；端到端测试验证集成有效 |
| UPD-02 | App 启动时自动检查 GitHub Releases 的 latest.json | 安装 v0.2.1 → 启动 → 等待弹窗 → 弹窗出现即通过 |
| UPD-03 | 自定义 React 更新 UI（进度条 + 稍后提醒） | 点击"立即更新"后观察进度条显示 |
| UPD-04 | 签名验证通过后下载安装并重启 app | 下载完成 + 安装成功 = 通过；relaunch 可降级手动 |
| REL-01 | Cargo.toml 作为唯一版本来源，tauri.conf.json 省略 version 字段 | /ship patch 仅修改 Cargo.toml；CI 正确读取版本 |
| REL-02 | 项目专用发版技能（非全局 /release），bump Cargo.toml → CHANGELOG → commit → tag → push | 运行 /ship patch 观察输出 |
| REL-03 | GitHub Release Notes 包含 Gatekeeper 安装指引 | 检查 Release 页面 releaseBody 是否渲染正确 |
</phase_requirements>

---

## Summary

Phase 13 是验证阶段，不写新功能代码。唯一的代码变更是将 `release.yml` 中 `releaseDraft: true` 改为 `releaseDraft: false`，使 CI 完成后 Release 自动发布（而非 Draft），这样 `latest.json` 才会被 updater 的 endpoint 识别（GitHub Releases 的 `releases/latest/download/` 路径只对非 Draft Release 有效）。

验证流程采用"两次发版"模式：先用 `/ship patch` 发 v0.2.1 建立基准版本（安装到 /Applications），再发 v0.2.2 触发真实更新流。整个链路为：`/ship patch` → git tag v0.2.x → GitHub Actions CI → 双架构 DMG + latest.json → GitHub Release（自动发布）→ 已安装 app 启动时检测到更新 → UpdateDialog 弹出 → 用户确认 → 下载 + 签名验证 + 安装 + 重启。

已知风险两个：Bug #13804（ad-hoc 签名后 DMG 打包随机失败，重跑 CI 是唯一已知处理方式）和 Issue #2273（relaunch 报 os error 1，这是 macOS 对未公证 app 的权限限制，手动重启可接受）。

**Primary recommendation:** 先改 releaseDraft 为 false，然后按顺序执行两次发版验证，遭遇 CI 随机失败直接重跑，遭遇 relaunch 失败降级手动重启。

---

## Standard Stack

### 当前已配置的技术栈（无需变更）

| 组件 | 版本/路径 | 用途 |
|------|-----------|------|
| tauri-apps/tauri-action@v1 | v1 | CI 构建 + Release 上传 |
| tauri-plugin-updater | 2.x | updater 检查、下载、安装 |
| tauri-plugin-process | 2.x | relaunch() |
| .github/workflows/release.yml | 当前文件 | CI 流水线，唯一需修改文件 |
| .claude/commands/ship.md | 当前文件 | /ship 发版技能 |
| src/components/updater/useUpdater.ts | 当前文件 | 更新检测逻辑 |
| src/components/updater/UpdateDialog.tsx | 当前文件 | 更新 UI |
| tauri.conf.json plugins.updater.endpoints | 已配置 | 指向 GitHub Releases latest.json |

### 唯一需修改项

| 文件 | 当前值 | 目标值 | 原因 |
|------|--------|--------|------|
| `.github/workflows/release.yml` | `releaseDraft: true` | `releaseDraft: false` | Draft Release 不会成为 `releases/latest`，updater endpoint 无法找到 latest.json |

---

## Architecture Patterns

### 发版链路全景

```
本地开发机
├── /ship patch
│   ├── 读取 Cargo.toml version (0.2.0)
│   ├── bump → 0.2.1
│   ├── 更新 CHANGELOG.md
│   ├── git commit "chore(release): v0.2.1"
│   ├── git tag v0.2.1
│   └── git push && git push --tags
│
GitHub Actions (触发：tag v[0-9]*.[0-9]*.[0-9]*)
├── Job: macos-latest (aarch64)
│   ├── pnpm install
│   ├── tauri-action@v1 --target aarch64-apple-darwin
│   │   ├── TAURI_SIGNING_PRIVATE_KEY → .app.tar.gz.sig
│   │   ├── APPLE_SIGNING_IDENTITY="-" → ad-hoc codesign
│   │   └── 上传：CLIManager_0.2.1_aarch64.dmg + .app.tar.gz + .sig
│   └── 创建/更新 GitHub Release（releaseDraft: false → 自动发布）
│
└── Job: macos-latest (x86_64) [并行]
    ├── pnpm install
    ├── tauri-action@v1 --target x86_64-apple-darwin
    │   └── 上传：CLIManager_0.2.1_x86_64.dmg + .app.tar.gz + .sig
    └── 更新 latest.json（合并两架构信息）
        └── latest.json 位于 releases/latest/download/latest.json

已安装 app (v0.2.1 → 启动后检测 v0.2.2)
├── AppShell mount → useUpdater.checkForUpdate()
│   └── check() → https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json
│       ├── latest.json version > 当前版本 → update.available
│       └── UpdateDialog 弹出
├── 用户点击"立即更新"
│   ├── downloadAndInstall() → 下载 .app.tar.gz
│   │   ├── 验证 Ed25519 签名
│   │   ├── 解压到临时目录
│   │   └── 替换 /Applications/CLIManager.app
│   └── relaunch() [可能失败 os error 1 → 手动重启]
```

### releaseDraft: false 工作原理

tauri-action@v1 的 `releaseDraft: false`（默认值）表示：
- CI 完成后 Release 立即发布（非 Draft 状态）
- GitHub 的 `releases/latest` 指向最新的非 Draft、非 Prerelease 的 Release
- `releases/latest/download/latest.json` 自动解析到最新 Release 的该文件
- updater endpoint `https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json` 才能正常工作

如果保持 `releaseDraft: true`，Draft Release 不会成为 `latest`，updater 的 endpoint 返回 404 或旧版本，app 永远检测不到更新。

### latest.json 字段结构（tauri-action@v1 生成）

```json
{
  "version": "0.2.2",
  "notes": "...",
  "pub_date": "2026-03-14T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<base64 Ed25519 sig>",
      "url": "https://github.com/nuts2k/CLIManager/releases/download/v0.2.2/CLIManager_0.2.2_aarch64.app.tar.gz"
    },
    "darwin-x86_64": {
      "signature": "<base64 Ed25519 sig>",
      "url": "https://github.com/nuts2k/CLIManager/releases/download/v0.2.2/CLIManager_0.2.2_x86_64.app.tar.gz"
    }
  }
}
```

注意：tauri-action@v1 新版本的 latest.json 支持 `{os}-{arch}-{installer}` 格式（需要 tauri-plugin-updater 2.10.0+），但经典 `darwin-aarch64` 格式仍兼容。

---

## Don't Hand-Roll

| 问题 | 不要自建 | 使用已有方案 |
|------|---------|------------|
| 签名验证 | 不要手动校验 .sig | tauri-plugin-updater 内置 Ed25519 验证 |
| 版本比较 | 不要手写 semver 比较 | check() 返回的 update 对象已做版本比较 |
| 下载进度 | 不要用 fetch + ReadableStream | downloadAndInstall(callback) 已提供 Started/Progress/Finished 事件 |
| DMG 打包 | 不要脚本化 DMG 创建 | tauri-action 完整处理 |
| Release 创建 | 不要用 gh cli 手动创建 | tauri-action 自动管理 Release |

---

## Common Pitfalls

### Pitfall 1：Draft Release 导致 updater 永远检测不到更新
**What goes wrong：** `releaseDraft: true` 下 Release 是 Draft，`releases/latest` 不指向它，latest.json 访问返回旧版本或 404，app 的 `check()` 返回 null。
**Root cause：** GitHub 的 `releases/latest` API 只返回最新的已发布（非 Draft）Release。
**Prevention：** 修改 release.yml 将 `releaseDraft: true` 改为 `releaseDraft: false`，这是验证前必做的唯一代码变更。
**验证方式：** curl `https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json` 应返回 200 且包含正确版本。

### Pitfall 2：Bug #13804 — ad-hoc 签名后 DMG 打包随机失败
**What goes wrong：** CI 的 ad-hoc 签名（`APPLE_SIGNING_IDENTITY="-"`）成功，但后续 `bundle_dmg.sh` 失败，整个 job 报错。
**Root cause：** macOS runner 环境下 DMG 创建工具（hdiutil/bundle_dmg.sh）与 ad-hoc 签名的交互存在竞态条件，未修复。
**Prevention：** 无法完全规避，这是已知未修复的 Tauri bug（#13804）。
**处理方式：** 直接重跑失败的 CI job，不需要修改代码，通常第二次能通过。
**Warning signs：** CI 报 "failed to bundle project: error running bundle_dmg.sh"，而不是编译错误。

### Pitfall 3：relaunch() 报 "Operation not permitted (os error 1)"
**What goes wrong：** `downloadAndInstall()` 完成后，`relaunch()` 抛出错误，app 不会自动重启。
**Root cause：** macOS 对未公证（未经 Apple Notarization）的 app 在替换自身后重新执行有权限限制（Issue #2273，2025年仍未修复）。
**Prevention：** 无法规避（需要 Apple Developer 签名 + Notarization，已明确超出 v2.1 范围）。
**已有对策：** useUpdater.ts 中 relaunch() 失败已被 try/catch 捕获，不影响安装成功状态。
**验收标准：** 安装完成后手动重启 app，打开后显示新版本号即通过。

### Pitfall 4：updater 在 v0.2.1 安装后检测到自身版本
**What goes wrong：** 从 CI 下载的 DMG 安装 v0.2.1，app 启动后立即去 check latest.json，如果此时 latest Release 仍是 v0.2.1，check() 返回 null（版本相同不触发更新）。
**Prevention：** 两次发版设计的目的就是解决这个问题——必须等 v0.2.2 发布后，运行 v0.2.1 的 app 才能检测到更新。

### Pitfall 5：/ship patch 检查到 v2.0/v2.1 等 GSD tag
**What goes wrong：** `/ship` 脚本用 `git describe --tags --abbrev=0 HEAD` 获取上一个 tag，可能拿到 v2.0 或 v2.1（GSD 里程碑 tag），导致 CHANGELOG 范围错误。
**Root cause：** 仓库中存在 v1.0、v1.1、v2.0 这些 GSD 里程碑 tag，`git describe` 不区分。
**Prevention：** 验证 /ship 执行结果时注意 CHANGELOG 条目范围是否合理；如果条目有误，手动编辑 CHANGELOG.md 即可，不影响版本发布流程。

---

## Code Examples

### 修改 release.yml（唯一代码变更）

```yaml
# .github/workflows/release.yml
# 修改前：
          releaseDraft: true

# 修改后：
          releaseDraft: false
```

单行修改，位于 `uses: tauri-apps/tauri-action@v1` 的 `with:` 块内。

### 验证 latest.json 可达性（排查命令）

```bash
# 验证 endpoint 可达，确认 Release 已发布
curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json" | python3 -m json.tool

# 期望输出：包含 version 字段和 platforms.darwin-aarch64
```

### 验证 Release 产物完整性

```bash
# 列出 Release 资产（需 gh CLI）
gh release view v0.2.1 --json assets --jq '.assets[].name'

# 期望包含：
# CLIManager_0.2.1_aarch64.dmg
# CLIManager_0.2.1_x86_64.dmg
# CLIManager_0.2.1_aarch64.app.tar.gz
# CLIManager_0.2.1_aarch64.app.tar.gz.sig
# CLIManager_0.2.1_x86_64.app.tar.gz
# CLIManager_0.2.1_x86_64.app.tar.gz.sig
# latest.json
```

### 安装 DMG 到 /Applications

```bash
# 挂载 DMG
hdiutil attach ~/Downloads/CLIManager_0.2.1_aarch64.dmg

# 复制到 Applications
cp -R /Volumes/CLIManager/CLIManager.app /Applications/

# 卸载 DMG
hdiutil detach /Volumes/CLIManager

# 解除 Gatekeeper 限制（ad-hoc 签名必须执行）
xattr -cr /Applications/CLIManager.app
```

### 检查 app 运行版本（调试用）

```bash
# 从 app bundle 读取版本
/usr/libexec/PlistBuddy -c "Print CFBundleShortVersionString" \
  /Applications/CLIManager.app/Contents/Info.plist
```

---

## 验证执行顺序（推荐）

### Wave 1：准备工作（改 releaseDraft）

1. 修改 `.github/workflows/release.yml`：`releaseDraft: true` → `releaseDraft: false`
2. commit + push（不带 tag，不触发 CI）：`git commit -m "ci: auto-publish release (releaseDraft false)"`

### Wave 2：发布基准版本 v0.2.1

1. 运行 `/ship patch`
2. 监控 GitHub Actions：[https://github.com/nuts2k/CLIManager/actions](https://github.com/nuts2k/CLIManager/actions)
3. 等待两个 job（aarch64 + x86_64）均为绿色（约 10-15 分钟）
4. 如有 CI 失败（DMG 打包报错）：重跑 job，不修改代码
5. Release 发布后验证：
   - 访问 [https://github.com/nuts2k/CLIManager/releases](https://github.com/nuts2k/CLIManager/releases)，v0.2.1 应显示为 Release（非 Draft）
   - `curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json"` 应返回 version: "0.2.1"
   - 检查 Release 页面 Gatekeeper 指引（REL-03）
6. 下载 `CLIManager_0.2.1_aarch64.dmg` 并安装到 `/Applications`（含 xattr -cr）

### Wave 3：发布更新版本 v0.2.2

1. 运行 `/ship patch`（从 0.2.1 bump 到 0.2.2）
2. 等待 CI 完成（可重跑）
3. 确认 `releases/latest/download/latest.json` version 变为 "0.2.2"

### Wave 4：验证更新流

1. 启动 `/Applications/CLIManager.app`
2. 等待 UpdateDialog 弹出（app 启动时自动检查，~5 秒内）
3. 点击"立即更新"，观察进度条
4. 下载完成 → 安装成功 → relaunch（或手动重启）
5. 确认新版本正常运行（Settings 页面版本号显示 0.2.2）

---

## Validation Architecture

### 验证矩阵（替代自动化测试）

本阶段为人工端到端验证，无自动化测试可覆盖。验证标准如下：

| 需求 ID | 验证行为 | 验证方式 | 通过标准 |
|---------|---------|---------|---------|
| CICD-01 | tag 触发 CI | GitHub Actions 页面 | workflow 出现且状态 ≠ skipped |
| CICD-02 | 双架构构建 | Actions matrix jobs | aarch64 + x86_64 两个 job 均绿色 |
| CICD-03 | 产物完整 | `gh release view v0.2.1` | DMG + .app.tar.gz + .sig + latest.json 均存在 |
| SIGN-01 | ad-hoc 签名 | CI 日志 + DMG 可安装 | 无签名错误，DMG 安装到 /Applications 成功 |
| SIGN-02/03 | Ed25519 签名 | latest.json 中 .sig 字段 | signature 字段非空 |
| UPD-01 | 插件集成 | app 启动不报错 | 无 plugin not found 错误 |
| UPD-02 | 自动检查 | 启动 v0.2.1 后等待 | UpdateDialog 在 ~5 秒内弹出 |
| UPD-03 | 更新 UI | 点击"立即更新" | 进度条显示，稍后提醒按钮存在 |
| UPD-04 | 下载安装 | 确认更新 | 安装完成，新版本运行 |
| REL-01 | 版本来源 | CI 读取正确版本 | Release 标题含正确 semver |
| REL-02 | /ship 技能 | 运行 /ship patch | 5 步输出均为 ✅ |
| REL-03 | Gatekeeper 指引 | Release 页面 | 包含 xattr 命令块 |

### 测试框架
本阶段无自动化单元/集成测试，验证方式为人工操作 + CLI 命令观察。
- 快速命令：`curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json" | python3 -m json.tool`
- 完整验证：按上方 Wave 1-4 执行顺序完整走一遍

### Wave 0 Gaps

None — 本阶段为验证阶段，不需要新增测试基础设施。

---

## State of the Art

| 旧方法 | 当前方法 | 变更原因 |
|--------|---------|---------|
| 手动发版（手写 tag + gh release create） | `/ship patch` 一键脚本 | Phase 12-04 完成 |
| 手动上传 DMG | tauri-action@v1 自动上传 | Phase 12-02 完成 |
| 无自动更新 | tauri-plugin-updater 集成 | Phase 12-03 完成 |
| Draft Release（手动点发布） | releaseDraft: false（Phase 13 唯一代码变更） | updater endpoint 需要非 Draft Release |

---

## Open Questions

1. **latest.json 双 job 并发写入冲突**
   - What we know：两个 matrix job（aarch64 + x86_64）都会调用 tauri-action 写 latest.json，存在竞态
   - What's unclear：tauri-action v1 如何处理并发写入（是合并两架构还是最后写入覆盖）
   - Recommendation：验证后检查 latest.json 是否同时包含 darwin-aarch64 和 darwin-x86_64 两个 platform 条目；tauri-action 官方示例采用相同 matrix 模式，通常能正确合并

2. **/ship 的 git describe 拾取 GSD tag**
   - What we know：仓库中有 v1.0、v1.1、v2.0 等 GSD 里程碑 tag（两段式）
   - What's unclear：`git describe --tags --abbrev=0 HEAD` 会拿到哪个 tag 作为 CHANGELOG 起点
   - Recommendation：执行 /ship 时观察 CHANGELOG.md 的条目范围，如不正确可手动编辑；不影响发版主流程

---

## Sources

### Primary (HIGH confidence)
- 代码库文件直接读取（release.yml、useUpdater.ts、UpdateDialog.tsx、tauri.conf.json、Cargo.toml、ship.md）— 当前实际实现状态
- [tauri-apps/tauri-action README](https://github.com/tauri-apps/tauri-action) — releaseDraft 参数说明（默认 false）
- [tauri-apps/tauri-action 官方示例](https://github.com/tauri-apps/tauri-action/blob/dev/examples/publish-to-auto-release.yml) — auto-publish 配置

### Secondary (MEDIUM confidence)
- [Bug #13804 — ad-hoc 签名 DMG 随机失败](https://github.com/tauri-apps/tauri/issues/13804) — CI 随机失败的根因确认
- [Issue #2273 — relaunch os error 1](https://github.com/tauri-apps/plugins-workspace/issues/2273) — relaunch 失败的已知问题确认
- [Tauri v2 updater 博客](https://ratulmaharaj.com/posts/tauri-automatic-updates/) — latest.json 格式与 endpoint 配置验证

### Tertiary (LOW confidence)
- 搜索结果中关于 tauri-action@v1 的新格式（`{os}-{arch}-{installer}` 键名）需要 tauri-plugin-updater 2.10.0+，当前项目使用版本待确认

---

## Metadata

**Confidence breakdown:**
- 代码变更（releaseDraft: false）: HIGH — 直接从官方文档确认
- CI 流水线执行：HIGH — release.yml 已实现，逻辑清晰
- updater 端到端：HIGH — 组件均已就绪，链路清晰
- 已知 Bug 处理：HIGH — 官方 issue 确认，处理策略明确
- latest.json 并发合并：MEDIUM — 官方示例采用相同模式，但未找到明确合并机制文档

**Research date:** 2026-03-14
**Valid until:** 2026-04-14（tauri-action 变更频繁，建议 30 天内执行验证）
