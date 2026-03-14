# Project Research Summary

**Project:** CLIManager v2.1 Release Engineering
**Domain:** CI/CD、macOS 代码签名、Tauri 2 自动更新
**Researched:** 2026-03-14
**Confidence:** HIGH

## Executive Summary

CLIManager v2.1 的 Release Engineering 是一项典型的 Tauri 2 macOS 发版工程任务，核心目标是建立"git tag push → CI 自动构建 → GitHub Release 发布 → 用户自动更新"的完整闭环。专家方案极为精简：官方 `tauri-apps/tauri-action@v1` 承担绝大部分重工——自动编译双架构 DMG、生成 updater 所需的 `latest.json`、上传所有 artifacts 到 GitHub Release；在此之上只需添加 2 个 Rust crate 和 2 个 npm 包即可完成自动更新集成，整体新增代码量极低。

推荐方案是以 git tag 作为版本号的唯一来源，通过本地 `scripts/release.sh` 脚本同步三文件版本号并 push tag 触发 CI。签名策略采用 ad-hoc 模式（`APPLE_SIGNING_IDENTITY=-`），无需 Apple Developer 账号即可满足 Apple Silicon 的运行要求；Tauri updater 独立使用 Ed25519 密钥对校验更新包完整性，两套签名机制完全解耦。更新端点直接使用 GitHub Releases 的静态 `latest.json`，零服务器运维成本。

最关键的风险有两类：一是已知 Bug 风险——ad-hoc 签名在 macOS CI 中可能随机失败（Issue #13804），`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 从环境变量读取也存在已知 Bug（Issue #13485）；二是配置陷阱风险——`latest.json` 格式任意字段错误会导致更新检查静默失败，updater 私钥丢失将使已安装用户永久无法自动更新。规避策略是：生成密钥时不设置密码、立即备份私钥、优先让 `tauri-action@v1` 自动生成 `latest.json` 而不手写、发版脚本中加版本号格式校验。

---

## Key Findings

### Recommended Stack

v2.1 只在现有成熟技术栈（Tauri 2.10、React 19、Vite 7、axum 0.8 等）上增量添加 Release Engineering 能力，新增依赖极少。版本号严格对齐是关键约束：`tauri-plugin-updater@2.10.0`（Rust）与 `@tauri-apps/plugin-updater@2.10.0`（npm）必须精确同版本，且 2.10.0 是 `tauri-action@v1` 生成的新格式 `latest.json` 所要求的最低版本，低版本 updater 无法解析新格式。

**核心新增技术：**
- `tauri-apps/tauri-action@v1`：GitHub Actions 构建 + 打包 + 发布一体化 — 官方维护，自动生成 `latest.json`，替代至少 50 行手写 YAML
- `tauri-plugin-updater@2.10.0`（Rust）+ `@tauri-apps/plugin-updater@2.10.0`（npm）：应用内自动更新 — 官方插件，与 Tauri 2.10 对应版本
- `tauri-plugin-process@2.3.1`（Rust）+ `@tauri-apps/plugin-process@2.3.1`（npm）：更新安装后重启应用 — updater 的标准配套
- `APPLE_SIGNING_IDENTITY=-`：ad-hoc 代码签名 — 无证书、无额外工具、单个环境变量搞定 Apple Silicon 运行要求
- `TAURI_SIGNING_PRIVATE_KEY`：updater Ed25519 签名私钥 — 存入 GitHub Secret，与 macOS 代码签名完全独立
- `dtolnay/rust-toolchain@stable`：CI 安装 Rust toolchain — 轻量、维护活跃

### Expected Features

研究明确划分了 v2.1 必做、可延后和明确不做三个层次。

**必须（Table Stakes）：**
- GitHub Actions tag 触发构建（aarch64 + x86_64 双架构 matrix）— 持续分发的基础流水线
- Ad-hoc 代码签名 — Apple Silicon 可运行的最低要求
- Updater 签名密钥对生成与配置 — `tauri-plugin-updater` 强制要求，无法跳过
- `tauri-plugin-updater` 基础集成（启动时检查 + 下载 + 安装 + 重启）— v2.1 核心功能
- 本地发版脚本（bump 版本 + tag + push）— 消除手动同步三文件的操作误差
- Gatekeeper 用户引导文档（Release Notes 中说明 `xattr` 步骤）— 防止用户因"已损坏"提示放弃

**应该有（Differentiators，v2.1.x）：**
- CHANGELOG 自动生成（git-cliff）— 前提是团队已遵守 Conventional Commits
- 自定义更新 UI（进度条 + 稍后提醒）— 优先验证基础更新流程后再打磨 UX
- Release Draft 审核流程 — `releaseDraft: true` 已内建支持，无需额外开发

**明确不做（Anti-Features）：**
- Apple Developer 证书签名 + Notarization — v3.x 再考虑，当前无 Apple 账号
- Windows / Linux 构建 — 项目依赖 iCloud Drive，macOS 优先
- macOS App Store 分发 — 沙箱限制与本地 HTTP 代理不兼容
- 自建更新服务器 / CDN — GitHub Releases 零成本且 tauri-action 原生支持

### Architecture Approach

整体架构是"本地脚本 → git tag → GitHub Actions → GitHub Releases → Tauri updater"的线性闭环，各环节职责清晰。`scripts/release.sh` 负责本地版本同步和 tag push；`release.yml` 负责 CI 构建（matrix 双架构）和 Release Draft 创建；`tauri-plugin-updater` 在运行时轮询 `latest.json` 完成自动更新。

**主要组件：**
1. `scripts/release.sh`（新增）— 本地发版：同步三文件版本号 + `Cargo.lock` + git tag push
2. `.github/workflows/release.yml`（新增）— CI：tag 触发 → 双架构 matrix 构建 → tauri-action 发布 Release Draft
3. `src-tauri/tauri.conf.json`（修改）— 声明 updater 端点、pubkey、`createUpdaterArtifacts: true`、`signingIdentity: "-"`
4. `src/lib/updater.ts`（新增）— 前端：`check()` → `downloadAndInstall()` → `relaunch()` 独立于业务组件
5. `src-tauri/src/lib.rs`（修改）— 注册 `tauri_plugin_updater::Builder::new().build()`，用 `#[cfg(desktop)]` 限定
6. `src-tauri/capabilities/default.json`（修改）— 新增 `updater:default` 权限
7. GitHub Releases `latest.json` — updater 静态端点，由 tauri-action 自动生成并上传

### Critical Pitfalls

研究识别出 8 个已知陷阱，以下是最高优先级的 5 个：

1. **Ad-hoc 签名 CI 随机失败（已知 Bug #13804）** — 双重保险：在 `tauri.conf.json` 的 `bundle.macOS.signingIdentity` 设 `"-"` 并同时设置 `APPLE_SIGNING_IDENTITY="-"` 环境变量；预留 CI 重跑预算；在 `macos-13` 上失败率更低
2. **`TAURI_SIGNING_PRIVATE_KEY` 私钥丢失**（恢复成本 HIGH）— 生成时立即双备份（GitHub Secret + 密码管理器）；公钥 commit 到仓库；一旦丢失已安装用户永久无法自动更新
3. **`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 从 ENV 读取失败（已知 Bug #13485）** — 生成密钥时不设置密码（直接回车），用 GitHub Secret 访问控制替代密码保护
4. **`latest.json` 格式任意字段错误导致更新静默失败** — 始终让 `tauri-action@v1` 自动生成 `latest.json`，不手写；`pubkey` 必须是文件内容而非路径；发布后用 `curl -L <url> | jq .` 验证结构
5. **GitHub Token 缺少 `contents: write` 权限** — 在每个 job 级别显式声明 `permissions: contents: write`，不依赖全局仓库设置

---

## Implications for Roadmap

基于研究中识别的依赖约束和风险集中点，建议将 v2.1 Release Engineering 分为 3 个阶段：

### Phase 1: CI/CD 基础与签名配置

**Rationale:** 所有后续工作的基础。密钥对必须最先生成（公钥写入 `tauri.conf.json` 后才能构建），GitHub Actions 工作流必须建立后其他一切才能被 CI 验证。8 个 Pitfall 中有 5 个集中在此阶段，必须逐一处理。

**Delivers:** 可工作的 CI 流水线——push tag → 双架构 DMG 构建 → GitHub Release Draft 创建；updater 签名密钥对已备份

**Addresses:**
- 生成并备份 updater 签名密钥对（Ed25519）
- `tauri.conf.json` 新增 `createUpdaterArtifacts: true`、`signingIdentity: "-"`、`pubkey`、`endpoints`
- `.github/workflows/release.yml` 编写（matrix、`contents: write` 权限、`tauri-action@v1`、`APPLE_SIGNING_IDENTITY=-`）
- 版本号三文件同步策略确认（以 `Cargo.toml` 为单一来源，CI 注入）

**Avoids:** Pitfall #1（ad-hoc 随机失败）、Pitfall #3（密钥密码 ENV Bug）、Pitfall #5（版本不同步）、Pitfall #8（Token 权限）

### Phase 2: Tauri updater 集成与端到端验证

**Rationale:** 依赖 Phase 1 建立的 CI 流水线（需要已发布的 Release 来测试 updater 端点）。updater 集成本身代码量少，但端到端验证（安装旧版 → 触发更新 → 安装成功 → 新版可打开）必须完整测试，Pitfall #6（`latest.json` 格式错误）和 Pitfall #7（更新包被 Gatekeeper 二次拦截）都在此阶段才能发现。

**Delivers:** 完整的自动更新闭环——app 启动时检查更新、下载、签名验证、安装、重启；两个架构的 DMG 都能更新

**Uses:** `tauri-plugin-updater@2.10.0`、`tauri-plugin-process@2.3.1`（Rust + npm 双端）

**Implements:**
- `src-tauri/Cargo.toml` 新增 updater + process 依赖（desktop-only target）
- `src-tauri/src/lib.rs` 注册 updater 插件（`#[cfg(desktop)]`）
- `src-tauri/capabilities/default.json` 新增 `updater:default` 权限
- `src/lib/updater.ts` 前端更新检查逻辑
- 端到端测试：安装 v2.0.x → 发布 v2.1.0 → 确认 aarch64 和 x86_64 都能收到并完成更新

**Avoids:** Pitfall #6（`latest.json` 格式错误）、Pitfall #7（Gatekeeper 二次拦截）

### Phase 3: 发版脚本与用户引导文档

**Rationale:** 依赖 Phase 1（需要知道三文件版本格式约定）和 Phase 2（需要知道完整更新流程才能写引导文档）。发版脚本让整个流程变成单命令触发，用户引导文档是 Pitfall #2（Gatekeeper"已损坏"警告）的主要缓解措施。

**Delivers:** 可重复执行的发版 SOP；用户能按文档指引完成首次安装和更新

**Addresses:**
- `scripts/release.sh` 本地发版脚本（版本号 semver 校验 + 三文件更新 + `Cargo.lock` 更新 + tag push）
- Release Notes 模板（第一行说明 Gatekeeper 处理步骤、`xattr` 命令）
- README 更新（安装说明、更新说明、架构选择指引）
- 验证 checklist（两个架构测试、end-to-end 更新流程、私钥备份确认等 8 项）

**Avoids:** Pitfall #2（Gatekeeper"已损坏"警告导致用户放弃）、Pitfall #5（版本三文件手动不同步）

### Phase Ordering Rationale

- **顺序约束明确：** Phase 1 的密钥对公钥必须写入 `tauri.conf.json` 才能开始构建；Phase 2 需要 Phase 1 的 CI 流水线产出真实 Release 来测试 updater 端点；Phase 3 的发版脚本文档化的是 Phase 1+2 建立的流程。
- **风险前置：** 8 个关键 Pitfall 中 5 个集中在 Phase 1，让最危险的配置在第一阶段就处理完，后续阶段风险大幅降低。
- **可并行优化：** Phase 1 中 updater 插件的 Rust 代码修改（`lib.rs`、`Cargo.toml`）可与 CI 工作流编写并行；Phase 2 和 Phase 3 的发版脚本核心逻辑也可提前编写，等 CI 验证完成后补充完善。

### Research Flags

**无需额外 research-phase 的阶段：**
- **Phase 1:** 官方文档完整，`tauri-action@v1` README 详尽，已知 Bug 已在研究中记录具体规避方法
- **Phase 2:** `tauri-plugin-updater` 官方文档覆盖完整，配置选项清晰；端到端测试是执行问题而非研究问题
- **Phase 3:** 发版脚本是简单 shell 脚本，文档是写作任务，无需 research

**所有阶段均适合直接进入实现，无需 `/gsd:research-phase`。**

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | 所有版本号均经 npm / docs.rs / 官方文档多源交叉验证；`tauri-action@v1` vs `@v0` 的格式差异有官方 README 确认 |
| Features | HIGH | 官方文档 + tauri-action 仓库 + 多篇社区实战文章一致；Table Stakes / Anti-Features 划分清晰 |
| Architecture | HIGH | 组件职责和数据流直接来自官方文档和 tauri-action README；Build Order 的依赖约束有官方文档支撑 |
| Pitfalls | HIGH | 8 个 Pitfall 均有 GitHub Issue 编号或官方文档条目支撑，非推断性风险 |

**Overall confidence:** HIGH

### Gaps to Address

- **macOS runner 版本选择（`macos-latest` vs `macos-13`）：** ad-hoc 签名随机失败在不同 runner 版本上的失败率尚无定量数据，需在 Phase 1 CI 运行中观测记录，失败时降级到 `macos-13`。
- **Gatekeeper 更新后二次拦截实际行为（Pitfall #7）：** `tar` 解压后 quarantine xattr 继承行为在不同 macOS 版本上不确定，需在 Phase 2 端到端测试中实际验证，以决定是否在 updater 完成后加 `xattr` 清除步骤。
- **`tauri-action` 版本一致性（STACK.md vs ARCHITECTURE.md 轻微差异）：** STACK.md 推荐 `@v1`，ARCHITECTURE.md 部分代码示例写的是 `@v0`。结论是用 `@v1`（STACK.md 判断更新且有官方 README 支撑），实现时需统一。

---

## Sources

### Primary（HIGH confidence）
- [Tauri v2 GitHub Actions 官方文档](https://v2.tauri.app/distribute/pipelines/github/) — workflow 结构、runner 配置、secrets 用法
- [tauri-apps/tauri-action GitHub README](https://github.com/tauri-apps/tauri-action) — v0 vs v1 差异、`__VERSION__` 占位符、`tauriScript`、`includeUpdaterJson`
- [tauri-plugin-updater 官方文档](https://v2.tauri.app/plugin/updater/) — `latest.json` 格式、endpoints、pubkey、权限配置
- [macOS Code Signing 官方文档](https://v2.tauri.app/distribute/sign/macos/) — ad-hoc signing `signingIdentity: "-"` 配置和限制
- [Tauri Configuration Files 官方文档](https://v2.tauri.app/develop/configuration-files/) — `version` 字段语义、`--config` JSON override
- [Tauri Issue #13804](https://github.com/tauri-apps/tauri/issues/13804) — ad-hoc 签名 CI 随机失败已知 Bug
- [Tauri Issue #13485](https://github.com/tauri-apps/plugins-workspace/issues/13485) — `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` ENV 读取已知 Bug
- [Tauri plugins-workspace Issue #2610](https://github.com/tauri-apps/plugins-workspace/issues/2610) — `latest.json` 获取失败静默行为
- [GitHub Actions runner pricing docs](https://docs.github.com/en/billing/reference/actions-runner-pricing) — macOS runner 10x 计费倍率

### Secondary（MEDIUM confidence）
- [Dev.to: Ship Tauri v2 App Like a Pro (Part 1 & 2)](https://dev.to/tomtomdu73/) — 实战签名配置和 release 自动化
- [thatgurjot.com: Tauri Auto-updater with GitHub](https://thatgurjot.com/til/tauri-auto-updater/) — `latest.json` 端点配置实例
- [yuexun.me: Native macOS Updates in Tauri](https://yuexun.me/native-macos-updates-in-tauri/) — 内置 dialog 局限性分析
- [macOS Gatekeeper bypass via tar extraction (Unit 42)](https://unit42.paloaltonetworks.com/gatekeeper-bypass-macos/) — tar 解压不传播 quarantine xattr 的技术背景

---
*Research completed: 2026-03-14*
*Ready for roadmap: yes*
