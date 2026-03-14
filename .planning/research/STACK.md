# Stack Research

**Domain:** Release Engineering — CI/CD, code signing, auto-update for Tauri 2 macOS desktop app
**Researched:** 2026-03-14
**Confidence:** HIGH (核心版本号均经官方文档和 npm/crates.io 多源交叉验证)

## Scope

本文档只覆盖 v2.1 Release Engineering 所需的**增量**技术栈。现有已验证技术栈（Tauri 2.10, React 19, Vite 7, axum 0.8, shadcn/ui, Tailwind CSS v4, Rust 后端 serde/toml_edit/notify 等）不在本文件重新评估。

---

## 核心发现摘要

v2.1 需要四项新能力：**GitHub Actions CI/CD**、**CI 版本注入**、**macOS ad-hoc 签名**、**Tauri 自动更新**。这四项能力的实现成本极低：

- 只新增 2 个 Rust crate（`tauri-plugin-updater` + `tauri-plugin-process`）
- 只新增 2 个 npm 包（`@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`）
- GitHub Actions 用官方 `tauri-apps/tauri-action@v1` 覆盖构建+发布
- 版本注入用 `sed` 脚本（无额外工具依赖）
- Ad-hoc 签名用单个环境变量 `APPLE_SIGNING_IDENTITY=-`（无需证书）

**置信度:** HIGH

---

## Recommended Stack

### 核心新增技术

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `tauri-plugin-updater` | 2.10.0 | Tauri 内置自动更新插件 | 官方插件，与 Tauri 2.10 版本对应。**2.10.0 是关键版本**：`tauri-apps/tauri-action@v1` 生成的新格式 `latest.json`（含 `{os}-{arch}-{installer}` 键）要求 updater >= 2.10.0，低版本无法解析。作为 Rust crate 与同版本 npm 包必须严格同步。 |
| `tauri-plugin-process` | 2.3.1 | 更新完成后重启应用 | 官方插件，updater 的标准配套：`downloadAndInstall()` 后调用 `relaunch()` 重启 app 完成更新。无其他实现方式。 |
| `tauri-apps/tauri-action@v1` | v1 (latest stable) | GitHub Actions 构建 + 发布 Tauri app | 官方维护的 GitHub Action。自动完成 `tauri build`、生成 `latest.json`（updater 需要）、上传 DMG + `.sig` 文件到 GitHub Releases。`@v1` 是当前推荐标签（`@v0` 已过时，部分官方文档引用但 README 已指向 v1）。 |
| `actions/checkout@v4` | v4 | Checkout 代码 | GitHub 官方推荐版本 |
| `actions/setup-node@v4` | v4 | 安装 Node.js | GitHub 官方推荐版本 |
| `dtolnay/rust-toolchain@stable` | stable | 安装 Rust toolchain | Tauri CI 生态标准选择，比官方 `actions-rs/toolchain` 更轻量且维护活跃 |

### 版本注入工具

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `sed` (macOS runner 内置) | 系统内置 | 从 git tag 提取版本号、注入到 Cargo.toml 和 tauri.conf.json | 每次 CI 构建触发时。无需额外安装，macOS runner 内置。 |
| `git describe` / `GITHUB_REF_NAME` | 系统内置 | 从触发 workflow 的 tag 提取版本号 | 推荐用 `GITHUB_REF_NAME` 而非 `git describe`，前者在 tag push 触发时直接获得 tag 名，更可靠。 |

### 签名工具

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `APPLE_SIGNING_IDENTITY=-` | 环境变量（无需安装） | macOS ad-hoc 代码签名 | Tauri bundler 读取此环境变量，使用 macOS 内置 `codesign` 工具以 `-`（dash）身份签名。macOS runner 内置 `codesign`，无需额外安装任何工具。 |
| `TAURI_SIGNING_PRIVATE_KEY` | 环境变量（密钥内容） | 为 updater 生成的 `.sig` 文件签名 | 由本地运行 `pnpm tauri signer generate` 生成的 minisign 私钥，以 base64 内容存入 GitHub Secret。与 Apple 代码签名无关，是 Tauri updater 校验机制。 |

---

## Recommended Stack（汇总表）

### Rust Crates（新增到 src-tauri/Cargo.toml）

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tauri-plugin-updater` | 2.10.0 | 应用内自动更新（检查、下载、安装） | 所有桌面平台（非 iOS/Android），用 `cfg` target 限定 |
| `tauri-plugin-process` | 2.3.1 | 更新安装后重启应用（`relaunch()`） | 与 updater 配合，更新完成后触发重启 |

### npm Packages（新增到 package.json）

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@tauri-apps/plugin-updater` | 2.10.0 | updater 插件前端 bindings | 与 Rust crate 严格同版本 |
| `@tauri-apps/plugin-process` | 2.3.1 | process 插件前端 bindings | 与 Rust crate 严格同版本 |

### GitHub Actions（.github/workflows/release.yml）

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `actions/checkout` | v4 | checkout 仓库代码 | 标准 |
| `actions/setup-node` | v4 | 安装 Node.js LTS | 配合 pnpm cache |
| `dtolnay/rust-toolchain` | stable | 安装 Rust + targets | `targets: aarch64-apple-darwin,x86_64-apple-darwin` |
| `tauri-apps/tauri-action` | v1 | 构建 + 打包 + 发布 GitHub Release | 生成 DMG、`.sig`、`latest.json` |

---

## Installation

```toml
# src-tauri/Cargo.toml — 新增（只覆盖桌面平台）
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-updater = "2.10.0"
tauri-plugin-process = "2.3.1"
```

```bash
# npm 前端 bindings（版本与 Rust crate 严格对齐）
pnpm add @tauri-apps/plugin-updater@2.10.0
pnpm add @tauri-apps/plugin-process@2.3.1
```

---

## Configuration Changes

### tauri.conf.json 新增配置

```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<内容来自 ~/.tauri/climanager.key.pub>",
      "endpoints": [
        "https://github.com/YOUR_USERNAME/CLIManager/releases/latest/download/latest.json"
      ]
    }
  }
}
```

关键点：
- `createUpdaterArtifacts: true` 让 Tauri bundler 生成 `.sig` 签名文件（updater 校验用）
- `pubkey` 必须是 `.pub` 文件内容，不能是文件路径
- `endpoints` 直接指向 GitHub Releases 上的 `latest.json`

### capabilities/default.json 新增权限

```json
{
  "permissions": [
    "updater:default",
    "process:allow-relaunch"
  ]
}
```

### CI 环境变量（GitHub Secrets）

| Secret | Value | Purpose |
|--------|-------|---------|
| `TAURI_SIGNING_PRIVATE_KEY` | minisign 私钥内容（`cat ~/.tauri/climanager.key`） | 对 updater 产物（.sig 文件）签名 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成密钥时设置的密码（可为空） | 解密私钥 |

无需 `APPLE_SIGNING_IDENTITY` secret——只需在 workflow env 段直接写 `-`（非 secret，ad-hoc 无证书无需保密）。

---

## 版本注入方案

**设计决策：以 git tag 为版本唯一来源，CI 注入 Cargo.toml 和 tauri.conf.json，不预先提交版本号。**

```yaml
# .github/workflows/release.yml 版本注入步骤
- name: Inject version from tag
  run: |
    # GITHUB_REF_NAME 在 tag push 时为 "v2.1.0"
    VERSION="${GITHUB_REF_NAME#v}"
    # 注入 Cargo.toml（[package] version 字段）
    sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
    # 注入 tauri.conf.json
    sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json
```

注意：macOS 的 `sed -i` 需要 `''` 参数（空字符串），Linux 不需要。macOS runner 用 `sed -i ''`，无跨平台问题（本项目 CI 仅跑 macOS）。

---

## GitHub Actions Workflow 结构

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            args: '--target aarch64-apple-darwin'
          - platform: macos-latest
            args: '--target x86_64-apple-darwin'

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin

      - name: Install pnpm and dependencies
        run: |
          npm install -g pnpm
          pnpm install

      - name: Inject version from tag
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
          sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" src-tauri/tauri.conf.json

      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_SIGNING_IDENTITY: '-'
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'CLIManager ${{ github.ref_name }}'
          releaseBody: 'See CHANGELOG for details.'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

---

## Ad-hoc 签名说明与局限

**Ad-hoc 签名是什么：** macOS 要求所有代码必须签名，特别是 Apple Silicon（arm64）上运行的 app。Ad-hoc 签名使用 `-` 作为身份，只做 checksum 校验，无证书验证链。

**Tauri 支持方式：** 将 `APPLE_SIGNING_IDENTITY=-` 设入 CI 环境，Tauri bundler 调用系统内置 `codesign` 工具完成 ad-hoc 签名，无需额外安装任何工具。

**已知局限（HIGH 置信度，官方文档明确说明）：**

| 场景 | 结果 |
|------|------|
| 用户下载后首次从 Finder 双击打开 | macOS GateKeeper 弹出"无法验证开发者"对话框，用户需在系统偏好设置 > 安全性中手动点"仍要打开" |
| 自动更新（updater）下载新版本后重启 | updater 会替换 app bundle，但新版本同样是 ad-hoc 签名，用户需再次手动授权 |
| 终端直接运行 `open CLIManager.app` | 正常，无弹窗 |
| 复制到其他 Mac 后运行 | 被 GateKeeper 拦截，需同样手动授权 |

**对 v2.1 的影响：** ad-hoc 签名足够让开发者和早期用户测试，但正式向公众分发前需要 Apple Developer 证书（$99/年）+ notarization。v2.1 里程碑 scope 明确是 ad-hoc，这是合理的阶段性目标。

---

## Alternatives Considered

| 推荐 | 备选 | 不选原因 |
|------|------|---------|
| `tauri-apps/tauri-action@v1` | 自写 `cargo tauri build` + `gh release create` | 官方 action 自动处理 `latest.json` 生成、多平台 artifact 上传、updater `.sig` 收集，自写至少 50 行 YAML 且易出错 |
| `APPLE_SIGNING_IDENTITY=-`（ad-hoc） | Apple Developer 证书签名 + notarization | v2.1 无 Apple 账号；ad-hoc 对内部/开发者分发足够，避免 $99/年费用和 notarization 流程复杂性 |
| `sed` 版本注入 | `cargo-release`、`release-plz`、`standard-version` | v2.1 只需 tag → 版本号，`sed` 两行脚本无新工具依赖；`cargo-release` 功能完整但学习和配置成本高于需求 |
| `GITHUB_REF_NAME` 取 tag | `git describe --tags` | `git describe` 在浅克隆（`actions/checkout` 默认）下可能失败，`GITHUB_REF_NAME` 是 GitHub Actions 原生环境变量更可靠 |
| GitHub Releases（静态 JSON） | CrabNebula Cloud / 自建更新服务器 | GitHub Releases 零成本、无额外账号、`tauri-action@v1` 原生支持；自建服务器增加运维复杂度，v2.1 不值得 |

---

## What NOT to Use

| 避免 | 原因 | 用什么替代 |
|------|------|-----------|
| Apple 代码签名证书工具（`xcrun altool`、`notarytool`）| v2.1 明确用 ad-hoc，这些工具需要 Apple Developer 账号和 secret 管理 | `APPLE_SIGNING_IDENTITY=-` 环境变量（无需任何工具安装） |
| `apple-actions/import-codesign-certs` | 导入真实 P12 证书的 Action，v2.1 无证书 | 不需要 |
| `tauri-action@v0` | 部分旧文档还引用，但 `@v1` 已是官方 README 推荐版本，`@v0` 的 `latest.json` 格式为旧格式，与 `tauri-plugin-updater 2.10.0` 新格式不完全兼容 | `tauri-apps/tauri-action@v1` |
| `cargo-tauri-action` / 非官方 Action | 社区维护，版本滞后，缺乏对 Tauri 2 新特性的同步更新 | `tauri-apps/tauri-action@v1` |
| `standard-version` / `semantic-release` | 为 npm 生态设计，在 Rust/Tauri 混合项目中配置复杂，超出 v2.1 scope | `sed` 脚本 + `CHANGELOG.md` 手写或 `git-cliff` |
| `tauri-plugin-updater` < 2.10.0 | `tauri-action@v1` 生成的 `latest.json` 新格式（`{os}-{arch}-{installer}` 键）需要 >= 2.10.0 才能正确解析，低版本 updater 无法识别更新 | `tauri-plugin-updater = "2.10.0"` |

---

## Stack Patterns by Variant

**如果需要 macOS Universal Binary（arm64 + x86_64 合一）：**
- 在 `tauri-action` 的 `args` 中传 `--target universal-apple-darwin`
- 需要 `rustup target add x86_64-apple-darwin aarch64-apple-darwin`
- Universal Binary 体积约为单架构的 2x，但用户只需下载一个文件
- 当前方案选择分架构构建（matrix），产物更小，updater `latest.json` 分别记录两个 target

**如果 CI 只构建 arm64（节省时间）：**
- matrix 中去掉 `x86_64-apple-darwin`，仅保留 `aarch64-apple-darwin`
- Intel Mac 用户仍可通过 Rosetta 运行，但非原生性能

**如果将来升级到 Apple Developer 证书签名：**
- 在 CI env 中添加 `APPLE_CERTIFICATE`（base64 P12）、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`（真实 identity string，非 `-`）
- 另外添加 notarization 环境变量：`APPLE_API_ISSUER`、`APPLE_API_KEY`、`APPLE_API_KEY_PATH`
- `APPLE_SIGNING_IDENTITY=-` 直接替换为真实 identity，其他 workflow 结构不变

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `tauri-plugin-updater@2.10.0` (Rust) | `@tauri-apps/plugin-updater@2.10.0` (npm) | Rust crate 和 npm 包必须严格同版本，patch 版本也不能错位 |
| `tauri-plugin-process@2.3.1` (Rust) | `@tauri-apps/plugin-process@2.3.1` (npm) | 同上 |
| `tauri-plugin-updater@2.10.0` | `tauri-apps/tauri-action@v1` | `@v1` 生成的新格式 `latest.json` 要求 updater >= 2.10.0 |
| `tauri-plugin-updater@2.10.0` | Tauri 2.10 | 同 minor 版本，无兼容性问题 |

---

## 密钥生成（一次性操作，本地执行）

```bash
# 在 CLIManager 项目根目录执行，生成 updater 签名密钥对
pnpm tauri signer generate -w ~/.tauri/climanager.key

# 将公钥内容复制到 tauri.conf.json 的 plugins.updater.pubkey
cat ~/.tauri/climanager.key.pub

# 将私钥内容存入 GitHub Secrets（TAURI_SIGNING_PRIVATE_KEY）
cat ~/.tauri/climanager.key
```

**重要提醒：** 私钥一旦丢失无法为已安装的用户推送更新（updater 校验会拒绝）。备份 `~/.tauri/climanager.key` 到安全位置（如密码管理器）。

---

## Sources

- [Tauri GitHub Actions 官方文档](https://v2.tauri.app/distribute/pipelines/github/) — workflow 结构、runner 选择、env 变量 (HIGH)
- [tauri-apps/tauri-action GitHub](https://github.com/tauri-apps/tauri-action) — v0 vs v1 差异、inputs 说明 (HIGH)
- [tauri-plugin-updater 官方文档](https://v2.tauri.app/plugin/updater/) — 配置格式、endpoints、pubkey、权限 (HIGH)
- [tauri-plugin-updater@2.10.0 on npm](https://www.npmjs.com/package/@tauri-apps/plugin-updater) — 当前版本确认 (HIGH)
- [tauri-plugin-updater 2.10.0 on docs.rs](https://docs.rs/crate/tauri-plugin-updater/latest) — Rust crate 版本确认 (HIGH)
- [tauri-plugin-process 2.3.1 on docs.rs](https://docs.rs/crate/tauri-plugin-process/latest) — Rust crate 版本确认 (HIGH)
- [@tauri-apps/plugin-process 2.3.1 on npm](https://www.npmjs.com/package/@tauri-apps/plugin-process) — npm 版本确认 (HIGH)
- [macOS Code Signing 官方文档](https://v2.tauri.app/distribute/sign/macos/) — ad-hoc signing 环境变量、局限说明 (HIGH)
- [tauri-apps/tauri issue #8763](https://github.com/tauri-apps/tauri/issues/8763) — ad-hoc signing 必要性讨论 (MEDIUM)
- [Tauri Discussion #6347 — version sync](https://github.com/tauri-apps/tauri/discussions/6347) — Cargo.toml 和 tauri.conf.json 版本同步策略 (MEDIUM)

---
*Stack research for: CLIManager v2.1 Release Engineering*
*Researched: 2026-03-14*
