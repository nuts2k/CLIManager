# Architecture Research: Release Engineering Integration

**Domain:** CI/CD + 代码签名 + 自动更新集成（Tauri 2 桌面应用发布工程）
**Researched:** 2026-03-14
**Confidence:** HIGH（官方文档 + tauri-action README 直接验证）

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────┐
│                     本地开发工作流                             │
├──────────────────────────────────────────────────────────────┤
│  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐    │
│  │ scripts/   │  │ CHANGELOG  │  │  git tag + push      │    │
│  │ release.sh │→ │ 更新       │→ │  v2.1.0              │    │
│  └────────────┘  └────────────┘  └──────────┬───────────┘    │
└─────────────────────────────────────────────┼────────────────┘
                                              │ tag push 触发
┌─────────────────────────────────────────────▼────────────────┐
│                    GitHub Actions (.github/workflows/)        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │ release.yml (tag: v*)                               │     │
│  │                                                     │     │
│  │  ┌──────────────┐    ┌─────────────────────────┐   │     │
│  │  │ matrix:      │    │ steps:                  │   │     │
│  │  │ macos-latest │    │ 1. checkout             │   │     │
│  │  │ (arm64)      │    │ 2. extract version      │   │     │
│  │  │ macos-latest │    │ 3. setup pnpm + Node    │   │     │
│  │  │ (x86_64)     │    │ 4. install Rust          │   │     │
│  │  └──────────────┘    │ 5. rust-cache            │   │     │
│  │                      │ 6. pnpm install          │   │     │
│  │                      │ 7. tauri-action@v0       │   │     │
│  │                      │    (--config version     │   │     │
│  │                      │     inject + ad-hoc sign)│   │     │
│  │                      └─────────────────────────┘   │     │
│  └─────────────────────────────────────────────────────┘     │
│                              │                               │
│                              ▼                               │
│              tauri-action 生成并上传到 GitHub Release:        │
│              ┌──────────────────────────────────────┐        │
│              │  CLIManager_2.1.0_aarch64.dmg         │        │
│              │  CLIManager_2.1.0_aarch64.dmg.sig     │        │
│              │  CLIManager_2.1.0_x64.dmg             │        │
│              │  CLIManager_2.1.0_x64.dmg.sig         │        │
│              │  latest.json  ← updater 端点           │        │
│              └──────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────┘
                              │
                   ┌──────────▼──────────┐
                   │  运行中的 CLIManager  │
                   │  tauri-plugin-updater│
                   │  轮询 latest.json   │
                   │  下载+安装新版本     │
                   └─────────────────────┘
```

### Component Responsibilities

| 组件 | 职责 | 实现位置 |
|------|------|----------|
| `scripts/release.sh` | 本地发版：版本号更新三文件 + CHANGELOG + git tag + push | 新增文件 |
| `release.yml` | CI 构建：tag 触发 → 版本注入 → 构建 → 签名 → 上传 Release | `.github/workflows/release.yml` |
| `tauri.conf.json` updater config | 声明更新端点 URL 和 pubkey，开启 createUpdaterArtifacts | 修改现有文件 |
| `src-tauri/Cargo.toml` updater dep | 引入 tauri-plugin-updater 依赖 | 修改现有文件 |
| `src-tauri/src/lib.rs` plugin init | 注册 tauri-plugin-updater 插件 | 修改现有文件 |
| `src-tauri/capabilities/default.json` | 新增 updater 相关权限 | 修改现有文件 |
| `src/lib/updater.ts` | 前端：调用 check() → downloadAndInstall() → relaunch() | 新增文件 |

## Recommended Project Structure

```
CLIManager/
├── .github/
│   └── workflows/
│       └── release.yml          # 新增：tag 触发构建发布
├── scripts/
│   └── release.sh               # 新增：本地发版脚本
├── src/
│   └── lib/
│       └── updater.ts           # 新增：前端更新检查逻辑
├── src-tauri/
│   ├── Cargo.toml               # 修改：新增 tauri-plugin-updater 依赖
│   ├── tauri.conf.json          # 修改：新增 updater 配置块
│   ├── src/
│   │   └── lib.rs               # 修改：注册 updater 插件
│   └── capabilities/
│       └── default.json         # 修改：新增 updater 权限
└── package.json                 # 修改：版本号（发版时更新）
```

### Structure Rationale

- **`.github/workflows/`**: GitHub Actions 约定目录，不可改
- **`scripts/`**: 本地开发辅助脚本，与 src 代码分开
- **`src/lib/updater.ts`**: 更新逻辑独立文件，不污染业务组件

## 数据流：从 tag push 到 updater 生效

### Release 触发流

```
开发者本地执行 scripts/release.sh v2.1.0
    ↓
脚本更新三文件版本号：
  package.json.version = "2.1.0"
  src-tauri/Cargo.toml [package].version = "2.1.0"
  src-tauri/tauri.conf.json.version = "2.1.0"
    ↓
脚本更新 Cargo.lock（cargo update --workspace --precise）
    ↓
脚本生成/更新 CHANGELOG.md
    ↓
脚本 git commit -am "chore: release v2.1.0"
    ↓
脚本 git tag v2.1.0 && git push origin main --tags
    ↓
GitHub Actions release.yml 触发（on.push.tags: v*）
```

### CI 构建流

```
GitHub Actions release.yml
    ↓
步骤 1: actions/checkout@v4
    ↓
步骤 2: 提取版本号
  VERSION="${GITHUB_REF_NAME#v}"  # 从 v2.1.0 提取 2.1.0
    ↓
步骤 3: setup pnpm（actions/setup-node + npm install -g pnpm）
步骤 4: dtolnay/rust-toolchain@stable
        targets: aarch64-apple-darwin,x86_64-apple-darwin
步骤 5: swatinem/rust-cache@v2
步骤 6: pnpm install
    ↓
步骤 7: tauri-apps/tauri-action@v0
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
  with:
    tagName: v__VERSION__           # __VERSION__ 替换为 tauri.conf.json 中的版本
    releaseName: CLIManager v__VERSION__
    releaseDraft: true
    includeUpdaterJson: true         # 生成 latest.json
    args: --target aarch64-apple-darwin  # (matrix 另一个 job 为 x86_64)
    tauriScript: pnpm tauri          # 显式指定，避免 pnpm-lock.yaml 检测问题
    # tauri-action 内部调用：
    # pnpm tauri build --target {arch} --config '{"version":"2.1.0"}'
    # + 环境变量 TAURI_SIGNING_PRIVATE_KEY 驱动 ad-hoc 签名
    ↓
产物上传到 GitHub Release (draft):
  CLIManager_2.1.0_aarch64.dmg
  CLIManager_2.1.0_aarch64.dmg.sig  (tauri updater 签名文件)
  CLIManager_2.1.0_x64.dmg
  CLIManager_2.1.0_x64.dmg.sig
  latest.json                        (updater 端点清单)
```

### Updater 检查流（应用运行时）

```
CLIManager 启动 / 用户触发检查更新
    ↓
tauri-plugin-updater check()
    ↓
GET https://github.com/[owner]/CLIManager/releases/latest/download/latest.json
    ↓
解析 latest.json:
  {
    "version": "2.1.0",
    "platforms": {
      "darwin-aarch64": { "url": "...", "signature": "..." },
      "darwin-x86_64":  { "url": "...", "signature": "..." }
    }
  }
    ↓
比较 latest.json.version > 当前版本？
  否 → 无更新，静默返回
  是 → 提示用户更新
    ↓
用户确认 → update.downloadAndInstall()
    ↓
下载对应平台 .dmg.tar.gz
    ↓
用 tauri.conf.json 中的 pubkey 验证签名
    ↓
验证通过 → 安装 + relaunch()
```

## Architectural Patterns

### Pattern 1: 版本号注入 — 通过 --config 覆盖（不修改文件）

**What:** tauri build 的 `--config` 参数支持 JSON Merge Patch（RFC 7396）。tauri-action 会把 `--config '{"version":"X.Y.Z"}'` 拼接到构建命令里，让 CI 构建使用正确的版本号，而不需要在 CI 里修改源文件。

**When to use:** 版本号已在发版脚本里写入三个文件，CI 只是验证一致性+构建，不需要再次修改文件。

**Trade-offs:**
- 好处：CI 步骤简洁，不产生额外 git diff
- 坏处：如果发版脚本忘记更新文件，CI 产物版本会与 tag 不一致（需发版脚本做 sanity check）

**实际用法（tauri-action 自动处理，不需要手写）:**

```yaml
# tauri-action 内部等效于：
- run: pnpm tauri build --target aarch64-apple-darwin
  env:
    TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
```

**注意：** tauri-action 的 `tagName: v__VERSION__` 的 `__VERSION__` 是从 `tauri.conf.json` 的 `version` 字段读取的，所以 tauri.conf.json 必须已经是正确的版本号（由发版脚本写入）。

### Pattern 2: Ad-hoc 签名 — TAURI_SIGNING_PRIVATE_KEY 驱动

**What:** macOS 应用必须签名。没有 Apple Developer 证书时，用 `signingIdentity: "-"` 进行 ad-hoc 签名。ad-hoc 签名让 macOS 内核接受应用运行，但不建立信任链，用户首次安装仍需在「隐私与安全性」中允许。

tauri-plugin-updater 的签名（用于验证更新包完整性）是独立的机制：用 `TAURI_SIGNING_PRIVATE_KEY` 对 `.dmg` 生成 `.sig` 文件，安装更新时用 tauri.conf.json 里的 pubkey 验证。

**When to use:** 团队内分发 / 开发者自用场景。面向公开用户分发时需要 Apple Developer 证书和公证（notarization）。

**Trade-offs:**
- 好处：无需 Apple Developer 账号（$99/年），CI 配置简单
- 坏处：用户体验差（需要手动授权），macOS Gatekeeper 拦截

**tauri.conf.json 配置：**

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "-"
    }
  }
}
```

**CI secrets 配置：**
- `TAURI_SIGNING_PRIVATE_KEY`：updater 私钥内容（`tauri signer generate` 生成）
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`：私钥密码（生成时设置）

### Pattern 3: tauri-plugin-updater 静态端点 — GitHub Releases latest.json

**What:** 使用 GitHub Releases 的 `latest.json` 文件作为 updater 端点，不需要额外的更新服务器。tauri-action 在构建时生成这个文件并上传到 Release。

**When to use:** 免服务器分发，GitHub 作为唯一基础设施。

**Trade-offs:**
- 好处：零额外服务器成本，运维负担极低
- 坏处：不支持灰度发布（所有用户同时看到新版本）；GitHub raw 访问在某些地区可能较慢

**端点 URL 格式：**

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/[owner]/CLIManager/releases/latest/download/latest.json"
      ]
    }
  }
}
```

**注意：** 这是静态 URL，不含 `{{target}}` / `{{arch}}` 变量。latest.json 文件本身包含各平台的条目，updater 插件会自动选择当前平台对应的条目。

### Pattern 4: Matrix 构建 — 同一 runner 两个 target

**What:** macOS 需要分别为 `aarch64-apple-darwin`（Apple Silicon）和 `x86_64-apple-darwin`（Intel）构建，两个 job 都在 `macos-latest` runner 上运行，通过 matrix 的 `args` 字段区分。

**When to use:** 需要同时支持 M1+ 和 Intel Mac 用户。

**Trade-offs:**
- 好处：原生编译，性能最优，无需 cross-compilation 工具链
- 坏处：需要两个 CI job，运行时间翻倍（Rust 编译约 10-20 分钟/个）
- 替代方案：Universal binary（`lipo` 合并两个架构），但 Tauri 目前不原生支持自动生成 Universal DMG

## New vs Modified Files

### 新增文件

| 文件 | 作用 | 关键内容 |
|------|------|----------|
| `.github/workflows/release.yml` | CI/CD 主工作流 | tag 触发，matrix 构建，tauri-action 调用 |
| `scripts/release.sh` | 本地发版脚本 | 更新三个文件版本号 + Cargo.lock + git tag |
| `src/lib/updater.ts` | 前端更新检查 | check() + downloadAndInstall() + relaunch() |

### 修改现有文件

| 文件 | 修改内容 | 为什么 |
|------|----------|--------|
| `src-tauri/tauri.conf.json` | 新增 `bundle.createUpdaterArtifacts: true`，新增 `bundle.macOS.signingIdentity: "-"`，新增 `plugins.updater.pubkey` + `plugins.updater.endpoints` | updater 插件配置和 ad-hoc 签名 |
| `src-tauri/Cargo.toml` | 新增 `tauri-plugin-updater` 依赖（desktop-only target） | Rust 插件依赖 |
| `src-tauri/src/lib.rs` | `setup()` 中注册 `tauri_plugin_updater::Builder::new().build()` | 插件初始化 |
| `src-tauri/capabilities/default.json` | 新增 `"updater:default"` 权限 | 前端调用 check/install 需要权限 |
| `package.json` | 新增 `@tauri-apps/plugin-updater` 依赖 | 前端 JS 绑定 |

## Integration Points

### External Services

| 服务 | 集成方式 | 注意事项 |
|------|----------|----------|
| GitHub Actions | `on.push.tags: v*` 触发 workflow | 需要在 Settings > Actions > General 开启 Read and write permissions |
| GitHub Releases | tauri-action 自动创建 Draft release 并上传产物 | 手动 publish draft 后才对 updater 可见（推荐：先 publish draft 验证产物，再设置为正式 release） |
| GitHub Releases latest.json | updater 端点（静态文件） | release 必须 publish（非 draft）updater 才能拉到；latest release 变化时所有在线用户下次检查时收到更新 |

### Internal Boundaries

| 边界 | 通信方式 | 注意事项 |
|------|----------|----------|
| CI → tauri.conf.json | `--config` JSON override（tauri-action 内部） | tauri.conf.json 必须提前写好正确版本号（由发版脚本完成） |
| tauri-plugin-updater (Rust) ↔ 前端 | Tauri 命令：`updater:allow-check`、`updater:allow-download-and-install` | 需要在 capabilities/default.json 声明权限 |
| 前端 updater.ts ↔ 用户界面 | 由调用方决定（可以是 Settings 页面的「检查更新」按钮，也可以是启动时静默检查） | v2.1 范围：触发机制交给实现阶段决定 |

## Build Order

基于文件依赖关系，推荐以下构建顺序（每步可独立验证）：

```
步骤 1: 生成 updater 签名密钥对
  命令: pnpm tauri signer generate -w ~/.tauri/climanager.key
  产出: 私钥文件 + 公钥字符串
  验证: 公钥字符串备好，准备写入 tauri.conf.json

步骤 2: 修改 tauri.conf.json
  新增: bundle.createUpdaterArtifacts: true
  新增: bundle.macOS.signingIdentity: "-"
  新增: plugins.updater.pubkey = "<公钥字符串>"
  新增: plugins.updater.endpoints = ["https://...latest.json"]
  验证: pnpm tauri build 本地成功（含 .sig 文件生成）

步骤 3: 集成 tauri-plugin-updater（Rust + 前端）
  修改: src-tauri/Cargo.toml 新增依赖
  修改: src-tauri/src/lib.rs 注册插件
  修改: src-tauri/capabilities/default.json 新增权限
  修改: package.json 新增 @tauri-apps/plugin-updater
  新增: src/lib/updater.ts
  验证: pnpm tauri build 编译通过，无权限报错

步骤 4: 配置 GitHub Actions
  新增: .github/workflows/release.yml
  配置: GitHub repo secrets（TAURI_SIGNING_PRIVATE_KEY + PASSWORD）
  验证: 推送 test tag，观察 CI 流程，确认 artifacts 生成

步骤 5: 编写本地发版脚本
  新增: scripts/release.sh
  测试: 本地 dry-run 验证三个文件版本号更新
  验证: 推送 tag，CI 构建，Release draft 出现正确产物
```

**依赖约束：**
- 步骤 2 必须在步骤 1 之后（需要公钥）
- 步骤 3 可与步骤 4 并行（互不依赖）
- 步骤 5 必须在步骤 4 之后（需要知道 tag 格式约定）

## Anti-Patterns

### Anti-Pattern 1: 在 CI 中修改源文件进行版本注入

**What people do:** 在 CI workflow 里用 `sed` 或 `node -e` 修改 `package.json`/`Cargo.toml`/`tauri.conf.json` 的版本号，然后再构建。

**Why it's wrong:**
- 产生 git dirty state（除非显式 commit，但 commit 回到 main 需要额外权限配置）
- 三个文件修改顺序有讲究，容易脚本写法不一致
- `Cargo.lock` 也需要同步更新，容易遗漏

**Do this instead:** 本地发版脚本统一更新三个文件 + Cargo.lock，commit 后 push tag。CI 只做构建，不修改文件。版本号从 tauri.conf.json 读取（tauri-action 的 `__VERSION__` 机制）。

### Anti-Pattern 2: 用 tauri-action@v0 时不指定 tauriScript: pnpm tauri

**What people do:** 在 pnpm 项目中使用 tauri-action 时省略 `tauriScript` 参数，依赖自动检测。

**Why it's wrong:** tauri-action 通过 lockfile 检测包管理器，但在某些 runner 环境下 `pnpm-lock.yaml` 检测可能失效（尤其是 checkout 深度或工作目录不匹配时），导致 fallback 到 npm，进而找不到 `@tauri-apps/cli`。

**Do this instead:**

```yaml
with:
  tauriScript: pnpm tauri
```

显式指定，消除歧义。

### Anti-Pattern 3: 将 updater 私钥提交到 repo

**What people do:** 将 `TAURI_SIGNING_PRIVATE_KEY` 的内容存为文件并提交到 repo，或写死在 workflow YAML 里。

**Why it's wrong:** 私钥泄露后无法撤销。攻击者可以签发伪造的更新包，通过 updater 推送给所有已安装用户。

**Do this instead:** 私钥只存于 GitHub repo secrets（`Settings > Secrets and variables > Actions`）。本地开发时通过 `TAURI_SIGNING_PRIVATE_KEY` 环境变量传入，不写文件，不提交。

### Anti-Pattern 4: Release draft 未 publish 就以为 updater 生效

**What people do:** tauri-action 创建 Release draft 后，以为 `latest.json` 已经可以被 updater 访问。

**Why it's wrong:** GitHub 的 `releases/latest` 路由只指向最新 published（非 draft）release。draft release 的文件不在 `releases/latest/download/` 路径下。

**Do this instead:** 先手动验证 draft release 中的产物（DMG 可安装、.sig 文件存在、latest.json 格式正确），然后 publish release。publish 后 updater 才能拉到更新。

### Anti-Pattern 5: 不做 macOS Universal Binary 而在 latest.json 中只提供一个架构

**What people do:** 只构建 aarch64，latest.json 里只有 `darwin-aarch64` 条目，Intel Mac 用户无法更新。

**Why it's wrong:** updater 会找当前平台对应的条目，找不到就静默失败（无更新提示）。Intel Mac 用户会永远停留在旧版本。

**Do this instead:** matrix 构建两个架构，tauri-action 的 `includeUpdaterJson: true` 会在第一个完成的 job 上传 latest.json（只有一个平台），第二个 job 会 merge。**注意：** 两个 job 需要同一个 release，tauri-action 会自动 upsert latest.json 里的平台条目。

## 现有文件修改详情

### src-tauri/tauri.conf.json — 新增配置块

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "CLIManager",
  "version": "0.1.0",
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,
    "macOS": {
      "signingIdentity": "-"
    },
    "icon": [...]
  },
  "plugins": {
    "updater": {
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "endpoints": [
        "https://github.com/[owner]/CLIManager/releases/latest/download/latest.json"
      ]
    }
  }
}
```

### src-tauri/capabilities/default.json — 新增 updater 权限

```json
{
  "permissions": [
    "core:default",
    "opener:default",
    "updater:default"
  ]
}
```

`updater:default` 包含 `allow-check`、`allow-download`、`allow-install`、`allow-download-and-install`。

### src-tauri/src/lib.rs — 注册 updater 插件

在现有 `Builder::default()` 链中追加，与 `tauri_plugin_opener::init()` 同层：

```rust
let builder = tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
        #[cfg(desktop)]
        app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
        // ... 现有 setup 代码不变 ...
        Ok(())
    })
```

**注意：** updater 插件用 `#[cfg(desktop)]` 包裹（与现有 tray 模块一致的 desktop-only 模式）。

### src-tauri/Cargo.toml — 新增依赖

```toml
[target.'cfg(any(target_os = "macos", windows, target_os = "linux"))'.dependencies]
tauri-plugin-updater = "2"
```

用 target-specific dependency 限定只在桌面平台引入。

## .github/workflows/release.yml — 结构概览

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            args: --target aarch64-apple-darwin
          - platform: macos-latest
            args: --target x86_64-apple-darwin

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - run: npm install -g pnpm

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - run: pnpm install

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: CLIManager v__VERSION__
          releaseBody: See CHANGELOG.md for details.
          releaseDraft: true
          prerelease: false
          includeUpdaterJson: true
          args: ${{ matrix.args }}
          tauriScript: pnpm tauri
```

## scripts/release.sh — 结构概览

```bash
#!/usr/bin/env bash
# 用法: ./scripts/release.sh 2.1.0
VERSION=$1

# 1. 更新 package.json
node -e "const f='package.json'; const p=JSON.parse(require('fs').readFileSync(f));
  p.version='$VERSION'; require('fs').writeFileSync(f, JSON.stringify(p,null,2)+'\n')"

# 2. 更新 src-tauri/tauri.conf.json
node -e "const f='src-tauri/tauri.conf.json'; const p=JSON.parse(require('fs').readFileSync(f));
  p.version='$VERSION'; require('fs').writeFileSync(f, JSON.stringify(p,null,2)+'\n')"

# 3. 更新 src-tauri/Cargo.toml（只改 [package] 下的第一个 version = 行）
sed -i '' "0,/^version = .*/s/^version = .*/version = \"$VERSION\"/" src-tauri/Cargo.toml

# 4. 更新 Cargo.lock
cargo update --manifest-path src-tauri/Cargo.toml --package cli-manager

# 5. Commit + tag
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: release v$VERSION"
git tag "v$VERSION"
git push origin main --tags
```

## 扩展性考量

| 关注点 | 当前阶段（v2.1） | 未来 |
|--------|------------------|------|
| 代码签名 | Ad-hoc（`signingIdentity: "-"`） | Apple Developer 证书 + 公证（notarization）解锁公开分发 |
| 更新端点 | 静态 GitHub Releases latest.json | 可改为动态服务器实现灰度发布/强制更新 |
| 构建平台 | macOS only（arm64 + x86_64） | 按需加 Linux/Windows job |
| 发版流程 | 手动运行 release.sh | 可加 `release-please` 自动化 CHANGELOG + PR |
| 更新 UI | 基础 check + install | 可加进度条、版本详情展示 |

## Sources

- [Tauri v2 Updater Plugin 官方文档](https://v2.tauri.app/plugin/updater/) — HIGH confidence（直接验证 tauri.conf.json 结构、Rust 注册方式、capabilities 权限）
- [Tauri v2 GitHub Actions 官方文档](https://v2.tauri.app/distribute/pipelines/github/) — HIGH confidence（workflow 结构、tag 触发、tauri-action 参数）
- [macOS 代码签名官方文档](https://v2.tauri.app/distribute/sign/macos/) — HIGH confidence（signingIdentity: "-" 配置）
- [tauri-apps/tauri-action README](https://github.com/tauri-apps/tauri-action) — HIGH confidence（action 参数、pnpm 检测行为、includeUpdaterJson）
- [Tauri v2 Configuration Files 官方文档](https://v2.tauri.app/develop/configuration-files/) — HIGH confidence（--config JSON override 机制验证）
- [DEV: Ship Tauri v2 App Like a Pro (Part 2/2)](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — MEDIUM confidence（社区实践验证，与官方文档一致）

---
*Architecture research for: CLIManager v2.1 Release Engineering*
*Researched: 2026-03-14*
