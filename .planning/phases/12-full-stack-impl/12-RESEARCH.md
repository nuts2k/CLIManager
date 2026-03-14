# Phase 12: 全栈实现 - Research

**Researched:** 2026-03-14
**Domain:** Tauri 2 CI/CD、updater 插件、Ed25519 签名、GitHub Actions 发版自动化
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### 更新提示 UI 交互
- 模态对话框形式（复用已有 Dialog 组件），不用 Toast
- 启动时自动检查更新，发现新版本弹出对话框
- 弹窗内容简洁：只显示当前版本号和新版本号，不展示更新日志
- 用户点「稍后提醒」后本次启动不再弹窗，下次启动再提醒
- 下载过程中显示进度条，完成后可立即安装并重启

#### 关于页面
- 在设置（Settings）中新增「关于」页面
- 显示当前版本号
- 打开页面时自动检查更新，有新版本则显示更新按钮
- 提供按钮链接到 GitHub Releases 页面，用户可查看详细更新信息

#### CHANGELOG 与 Release Notes
- CHANGELOG 自动生成：发版技能内置脚本解析 git log，按 Conventional Commits 规范分类
- 语言：中文（CHANGELOG 和 Release Notes 均为中文）
- Gatekeeper 安装指引：折叠段落放在 Release Notes 底部（`<details>` 标签）
- 零外部依赖（不用 git-cliff 或 conventional-changelog，未来 UPD-05 再考虑）

#### 发版技能工作流
- 命令名：`/ship`（项目局部技能，非全局 `/release`）
- 一键执行：`/ship patch|minor|major`，不逐步确认
- 流程：bump Cargo.toml → 生成 CHANGELOG → commit → tag → push
- 仅 bump Cargo.toml（tauri.conf.json 省略 version 字段，REL-01 已决定）

#### 初始版本号
- 首次 CI 发布版本：v0.2.0（从当前 0.1.0 升级）
- 后续版本升级不严格遵循 semver，0.x 阶段灵活处理

### Claude's Discretion
- 进度条具体实现方式（tauri-plugin-updater 的 download 事件 vs 自行计算）
- 关于页面的布局和样式细节
- CHANGELOG 分类模板的具体格式
- Release Notes 模板的具体排版
- `/ship` 技能的错误处理和回滚逻辑

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| REL-01 | Cargo.toml 作为唯一版本来源，tauri.conf.json 省略 version 字段 | 官方确认：省略 version 字段后 Tauri 自动读取 Cargo.toml |
| SIGN-02 | 生成 updater Ed25519 密钥对并安全备份 | `pnpm tauri signer generate -w ~/.tauri/climanager.key` 命令已验证 |
| SIGN-03 | 私钥存入 GitHub Secrets，公钥写入 tauri.conf.json | TAURI_SIGNING_PRIVATE_KEY secret + plugins.updater.pubkey 配置 |
| SIGN-01 | CI 构建时 macOS ad-hoc 签名（APPLE_SIGNING_IDENTITY="-"） | bundle.macOS.signingIdentity="-" 或同名 env var 已验证 |
| CICD-01 | 三段式 v*.*.* tag 推送触发 GitHub Actions | on.push.tags: ['v[0-9]*.[0-9]*.[0-9]*'] 模式 |
| CICD-02 | macOS 双架构构建（aarch64 + x86_64），生成 DMG | matrix 策略双条目，dtolnay/rust-toolchain 安装双 target |
| CICD-03 | 产物自动上传到 GitHub Release Draft | tauri-action@v1 with releaseDraft: true |
| UPD-01 | 集成 tauri-plugin-updater + tauri-plugin-process（Rust + JS 两端） | 依赖声明、plugin 注册、capability 权限已记录 |
| UPD-02 | App 启动时自动检查 GitHub Releases 的 latest.json | check() 在 app useEffect 中调用，endpoint 配置 |
| UPD-03 | 自定义 React 更新 UI（进度条 + 稍后提醒） | DownloadEvent 类型结构已记录 |
| UPD-04 | 签名验证通过后下载安装并重启 | downloadAndInstall() + relaunch() 模式 |
| REL-02 | 项目专用发版技能 /ship，bump → CHANGELOG → commit → tag → push | Claude skill 文件，纯 bash + git 实现 |
| REL-03 | GitHub Release Notes 包含 Gatekeeper 安装指引 | tauri-action releaseBody 模板 + details 折叠段 |
</phase_requirements>

---

## Summary

Phase 12 实现 CLIManager 的完整发版基础设施：Ed25519 密钥生成与配置（Wave 1）→ CI 流水线、Updater 插件、发版脚本三路并行（Wave 2）。

技术核心是 Tauri 2 的 `tauri-plugin-updater`（签名验证 + 下载进度 API）和 `tauri-action@v1`（GitHub Actions 自动构建双架构 DMG + 上传 Release + 生成 latest.json）。关键已知风险：ad-hoc 签名 DMG 打包随机失败（Bug #13804，AppleScript 超时），以及私钥密码 env var 注入 Bug（#13485，已决定不设密码规避）。

**Primary recommendation:** 使用 `tauri-action@v1` + 分离矩阵（aarch64/x86_64 独立 job）+ `APPLE_SIGNING_IDENTITY="-"` ad-hoc 签名；updater 使用 `downloadAndInstall()` 回调模式追踪进度；Cargo.toml 省略 tauri.conf.json 中的 version 字段。

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tauri-plugin-updater | ^2 (需 ≥2.10.0) | Rust 端 updater 插件，Ed25519 验签 + 下载 | Tauri 官方插件，唯一支持的 updater 方案 |
| @tauri-apps/plugin-updater | ^2 | 前端 JS updater API（check/download/install） | 与 Rust 端配套 |
| tauri-plugin-process | ^2 | App 重启（relaunch）支持 | updater 安装后重启必须依赖此插件 |
| @tauri-apps/plugin-process | ^2 | 前端 JS relaunch() | 与 Rust 端配套 |
| tauri-apps/tauri-action@v1 | v1 | GitHub Actions 构建 + Release 上传 + latest.json | 官方 action，@v1 的 latest.json 格式需 updater ≥2.10.0 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| dtolnay/rust-toolchain@stable | stable | CI Rust 工具链安装 | 每次 CI 构建 |
| actions/cache@v4 | v4 | pnpm store + Rust target 缓存 | 提速 CI |
| pnpm/action-setup@v4 | v4 | CI pnpm 安装 | 项目使用 pnpm |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| tauri-action@v1 | 手写 tauri build + softprops/action-gh-release | 参考项目 cc-switch 的做法，但需要手动组装 latest.json，工作量大且易出错 |
| 分离矩阵（aarch64/x86_64） | universal-apple-darwin 单次构建 | universal 一次构建更简单，但 cc-switch 采用分离矩阵；本项目选分离矩阵以匹配 tauri-action 官方示例 |
| ad-hoc 签名 | Developer ID 证书签名 | 正式签名需 $99/年 Apple 开发者账号，v2.1 阶段超出范围 |

**Installation:**
```bash
# Rust 端（src-tauri/Cargo.toml）
cargo add tauri-plugin-updater
cargo add tauri-plugin-process

# 前端
pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-process
```

---

## Architecture Patterns

### Recommended Project Structure

```
.github/
└── workflows/
    └── release.yml          # CI 流水线（Wave 2 产物）
src-tauri/
├── Cargo.toml               # 唯一版本来源（移除后 tauri.conf.json 无 version）
├── tauri.conf.json          # 添加 plugins.updater + bundle.createUpdaterArtifacts
├── capabilities/
│   └── default.json         # 添加 updater:default + process:default 权限
└── src/
    └── lib.rs               # 注册 tauri_plugin_updater + tauri_plugin_process
src/
└── components/
    ├── settings/
    │   └── SettingsPage.tsx  # 关于页面 tab 集成入口
    └── updater/
        ├── UpdateDialog.tsx  # 更新确认模态框（复用 Dialog 组件）
        └── useUpdater.ts     # updater hook（check/download 逻辑）
.claude/
└── commands/
    └── ship.md              # /ship 发版技能
CHANGELOG.md                 # 自动生成，/ship 命令维护
```

### Pattern 1: Cargo.toml 作为唯一版本来源（REL-01）

**What:** 删除 `tauri.conf.json` 中的顶级 `version` 字段，Tauri 自动回退读取 `Cargo.toml` 中的版本号。

**When to use:** 始终。这是 REL-01 的核心要求。

**Example:**
```json
// tauri.conf.json — 删除 "version": "0.1.0" 这行
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "CLIManager",
  "identifier": "com.climanager.app",
  // ... 其余不变，无 version 字段
}
```

```toml
# src-tauri/Cargo.toml — 版本的唯一真相来源
[package]
name = "cli-manager"
version = "0.2.0"  # /ship 命令修改这里
```

### Pattern 2: tauri.conf.json updater 与 ad-hoc 签名配置（SIGN-01、SIGN-03）

**What:** 在 `tauri.conf.json` 中配置 updater 公钥、endpoint、artifact 生成，以及 ad-hoc 签名 identity。

**Example:**
```json
// src-tauri/tauri.conf.json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,
    "macOS": {
      "signingIdentity": "-"
    },
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  },
  "plugins": {
    "updater": {
      "pubkey": "<<公钥文件内容，pnpm tauri signer generate 生成后粘贴>>",
      "endpoints": [
        "https://github.com/<<OWNER>>/CLIManager/releases/latest/download/latest.json"
      ]
    }
  }
}
```

注意：endpoint 使用 `releases/latest/download/latest.json` 静态 URL 格式，tauri-action@v1 会自动上传此文件。

### Pattern 3: Ed25519 密钥生成（SIGN-02）

**What:** 用 Tauri CLI 生成签名密钥对，私钥不设密码（规避 Bug #13485）。

**Example:**
```bash
# 生成密钥对（-p 不填则无密码，直接回车）
pnpm tauri signer generate -w ~/.tauri/climanager.key

# 输出：
# ~/.tauri/climanager.key      → 私钥（存入 GitHub Secrets）
# ~/.tauri/climanager.key.pub  → 公钥（粘贴到 tauri.conf.json）
```

私钥内容格式为两行：
```
untrusted comment: tauri autoupdater secret key
<base64 encoded key>
```

将私钥文件内容完整存入 GitHub Secrets 的 `TAURI_SIGNING_PRIVATE_KEY`。

### Pattern 4: GitHub Actions CI 流水线（CICD-01/02/03、SIGN-01）

**What:** 双架构矩阵构建 + tauri-action@v1 自动发 Release Draft + latest.json。

**Example:**
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v[0-9]*.[0-9]*.[0-9]*'  # 仅匹配三段式 tag，不响应 GSD v2.1 tag

permissions:
  contents: write

concurrency:
  group: release-${{ github.ref_name }}
  cancel-in-progress: true

jobs:
  publish-tauri:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: 'macos-latest'
            args: '--target aarch64-apple-darwin'
          - platform: 'macos-latest'
            args: '--target x86_64-apple-darwin'
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with:
          version: 10
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: 'pnpm'
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin
      - name: Install frontend deps
        run: pnpm install --frozen-lockfile
      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          APPLE_SIGNING_IDENTITY: "-"
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'CLIManager ${{ github.ref_name }}'
          releaseDraft: true
          releaseBody: |
            ## CLIManager ${{ github.ref_name }}

            <<CHANGELOG 内容由 /ship 维护>>

            <details>
            <summary>首次安装 Gatekeeper 指引</summary>

            macOS 对未经公证的应用会提示"已损坏"或"无法验证开发者"，执行以下命令解除限制：

            ```bash
            xattr -cr "/Applications/CLIManager.app"
            ```

            然后在「系统设置 → 隐私与安全性」中点击「仍要打开」。
            </details>
          args: ${{ matrix.args }}
```

### Pattern 5: tauri-plugin-updater JS API（UPD-02/03/04）

**What:** 启动时检查更新，DownloadEvent 回调追踪进度，installAndRelaunch。

**Example:**
```typescript
// Source: https://v2.tauri.app/plugin/updater/
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

type DownloadEvent =
  | { event: 'Started'; data: { contentLength?: number } }
  | { event: 'Progress'; data: { chunkLength: number } }
  | { event: 'Finished' };

export async function checkAndInstallUpdate(
  onProgress: (downloaded: number, total: number | undefined) => void
): Promise<void> {
  const update = await check();
  if (!update) return;

  let downloaded = 0;
  let contentLength: number | undefined;

  await update.downloadAndInstall((event: DownloadEvent) => {
    switch (event.event) {
      case 'Started':
        contentLength = event.data.contentLength;
        break;
      case 'Progress':
        downloaded += event.data.chunkLength;
        onProgress(downloaded, contentLength);
        break;
      case 'Finished':
        onProgress(downloaded, contentLength);
        break;
    }
  });

  await relaunch();
}
```

注意：`contentLength` 可能为 `undefined`（服务器未返回 Content-Length）；进度条在无总量时可用不确定态（indeterminate）展示。

### Pattern 6: Rust 端插件注册

**What:** 在 `lib.rs` 的 Builder 中注册两个插件。

**Example:**
```rust
// src-tauri/src/lib.rs
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        // ... 其余不变
```

### Pattern 7: /ship 发版技能（REL-02）

**What:** Claude skill 文件，纯 bash 实现 semver bump + CHANGELOG + git 操作。

**Example:**
```bash
#!/usr/bin/env bash
# .claude/commands/ship.md 中的脚本逻辑

BUMP="${1:-patch}"  # patch | minor | major
CARGO="src-tauri/Cargo.toml"

# 读取当前版本
CURRENT=$(grep '^version = ' "$CARGO" | head -1 | sed 's/version = "\(.*\)"/\1/')

# bump 版本
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"
case "$BUMP" in
  major) MAJOR=$((MAJOR+1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR+1)); PATCH=0 ;;
  patch) PATCH=$((PATCH+1)) ;;
esac
NEW="$MAJOR.$MINOR.$PATCH"

# 修改 Cargo.toml
sed -i.bak "s/^version = \"$CURRENT\"/version = \"$NEW\"/" "$CARGO" && rm "$CARGO.bak"

# 生成 CHANGELOG（按 Conventional Commits 分类）
# ... git log 解析逻辑

# commit + tag + push
git add "$CARGO" CHANGELOG.md
git commit -m "chore(release): v$NEW"
git tag "v$NEW"
git push && git push --tags
```

### Anti-Patterns to Avoid

- **在 tauri.conf.json 保留 version 字段：** 会导致版本不一致，REL-01 违规。删除即可，Tauri 自动读 Cargo.toml。
- **给私钥设置密码：** Bug #13485 导致 env var 传入密码时解码失败，私钥丢失也无法回滚已发布版本。不设密码是已记录决策。
- **使用 TAURI_PRIVATE_KEY（Tauri v1 变量名）：** Tauri 2 必须用 `TAURI_SIGNING_PRIVATE_KEY`。
- **手动构建 latest.json：** tauri-action@v1 自动生成，手工维护容易签名不对。
- **在前端 env prefix 中暴露 TAURI_SIGNING_*：** vite.config.ts 的 envPrefix 不能包含 TAURI_，否则私钥泄漏到 bundle（已知安全漏洞）。
- **把私钥提交到仓库：** 私钥一旦泄露必须重新生成密钥对并更新所有已发布 latest.json。

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Updater 签名验证 | 自己用 sha256 或 HMAC 验签 | tauri-plugin-updater（内置 Ed25519 + minisign） | 签名格式非 trivial，密钥管理有安全边界 |
| App 重启 | `std::process::Command::new(executable).spawn()` | tauri-plugin-process relaunch() | 跨平台差异、macOS 沙箱限制、tauri context 感知 |
| CI 上传 Release 资产 | 手写 curl + GitHub API | tauri-action@v1 | 处理 artifact 重试、draft 幂等、latest.json 聚合 |
| semver bump | 自写版本解析 | sed 单行替换即可 | 项目只需修改 Cargo.toml 一行，无需依赖额外工具 |

**Key insight:** Tauri updater 的签名是 minisign 格式（Ed25519），不是标准 X.509；自建验签会绕过整个安全模型，且 tauri-plugin-updater 2.10.0 以上才支持 tauri-action@v1 的新 latest.json 格式。

---

## Common Pitfalls

### Pitfall 1: Ad-hoc DMG 打包随机失败（Bug #13804）
**What goes wrong:** `bundle_dmg.sh` 内部调用 AppleScript（Finder layout），GitHub Actions macOS runner 的 Finder 响应超时，导致 CI 随机失败。
**Why it happens:** `macos-latest`（Apple Silicon runner）通常比 `macos-13`（Intel runner）更稳定，但仍有概率失败。
**How to avoid:** 固定使用 `macos-latest`（即 `macos-15` 或 Apple Silicon runner）；失败时重试 CI job 即可。备选：若连续失败 3 次以上，降级到 `macos-13`。
**Warning signs:** 错误日志出现 `AppleEvent timed out (-1712)` 或 `bundle_dmg.sh failed`。

### Pitfall 2: TAURI_SIGNING_PRIVATE_KEY 格式问题
**What goes wrong:** 私钥粘贴到 GitHub Secrets 后格式损坏（换行被剥除或被 base64 再次编码），导致 CI 签名失败。
**Why it happens:** GitHub Secrets 界面对多行文本处理不一致。
**How to avoid:** 将私钥文件**整体内容**（两行）粘贴为 secret 值；tauri-action 会正确处理。可参考 cc-switch workflow 中的多种格式兼容逻辑。
**Warning signs:** CI 报 `failed to decode secret key` 或 `incorrect updater private key password`。

### Pitfall 3: latest.json 格式与 updater 版本不匹配
**What goes wrong:** tauri-action@v1 生成的 latest.json 包含 `{os}-{arch}-{installer}` 复合键，但 tauri-plugin-updater <2.10.0 只支持旧格式 `{os}-{arch}`。
**Why it happens:** tauri-action@v1 引入了新的 latest.json schema。
**How to avoid:** 确保 `tauri-plugin-updater` 版本 ≥2.10.0（在 Cargo.toml 和 package.json 中）。
**Warning signs:** App 检查更新时返回空或解析错误。

### Pitfall 4: vite.config.ts envPrefix 泄漏私钥
**What goes wrong:** 若 `vite.config.ts` 的 `envPrefix` 配置包含 `'TAURI_'`，构建时 `TAURI_SIGNING_PRIVATE_KEY` 会被打包进前端 bundle。
**Why it happens:** Vite 默认只暴露 VITE_ 前缀；若误拷贝 cc-switch 的配置可能引入此问题。
**How to avoid:** 确认 vite.config.ts 的 envPrefix 只有 `'VITE_'`，不包含 `'TAURI_'`。
**Warning signs:** 构建产物体积异常增大；安全扫描发现密钥泄露。

### Pitfall 5: GSD milestone tag 与产品 tag 混淆
**What goes wrong:** GSD 用两段式 tag（如 `v2.1`）标记里程碑，若 CI 匹配 `v*` 会误触发。
**Why it happens:** 宽泛的 tag 匹配模式。
**How to avoid:** CI on.push.tags 使用精确的三段式匹配 `v[0-9]*.[0-9]*.[0-9]*`，不匹配 `v2.1`。

### Pitfall 6: 首次安装 Gatekeeper 拦截
**What goes wrong:** 用户下载 DMG 安装后，macOS 提示"应用已损坏"或"无法验证开发者"，无法启动。
**Why it happens:** Ad-hoc 签名不能通过 Gatekeeper 自动放行，需用户手动允许。
**How to avoid:** Release Notes 底部放置 `<details>` 折叠的 Gatekeeper 指引（`xattr -cr "/Applications/CLIManager.app"`），并在关于页面提供链接到 GitHub Releases。

---

## Code Examples

Verified patterns from official sources:

### Capabilities 权限配置
```json
// src-tauri/capabilities/default.json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "updater:default",
    "process:default"
  ]
}
```

### Rust 端依赖（Cargo.toml）
```toml
# src-tauri/Cargo.toml — 新增两行
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

或更简单（仅 macOS）：
```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

### 前端 UpdateDialog 集成模式
```typescript
// 启动时检查（App.tsx 或顶层 useEffect）
useEffect(() => {
  const checkUpdate = async () => {
    try {
      const update = await check();
      if (update) {
        setUpdateInfo({ version: update.version, update });
        setShowUpdateDialog(true);
      }
    } catch {
      // 静默失败，更新检查不阻断主流程
    }
  };
  void checkUpdate();
}, []);
```

### 「稍后提醒」状态管理（本次启动不再弹窗）
```typescript
// 使用 useRef 而非 useState，不触发重渲染
const dismissedThisSession = useRef(false);

const handleRemindLater = () => {
  dismissedThisSession.current = true;
  setShowUpdateDialog(false);
};
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tauri-action@v0 | tauri-action@v1 | 2024-2025 | latest.json 新格式，需 updater ≥2.10.0 |
| tauri.conf.json 中的 version 字段 | 省略 version，读取 Cargo.toml | Tauri 2 官方推荐 | 版本单一来源，REL-01 直接满足 |
| TAURI_PRIVATE_KEY（v1 变量名） | TAURI_SIGNING_PRIVATE_KEY | Tauri 2 | 变量名更改，v1 名称在 v2 无效 |
| universal-apple-darwin 单次构建 | aarch64 + x86_64 分离矩阵 | tauri-action 推荐模式 | 更灵活，每架构独立上传签名 |

**Deprecated/outdated:**
- `TAURI_PRIVATE_KEY`：Tauri 2 中改为 `TAURI_SIGNING_PRIVATE_KEY`
- `createUpdaterArtifacts: "v1Compatible"`：仅从 v1 迁移时需要，全新 v2 项目用 `true`

---

## Open Questions

1. **tauri-action@v1 的确切 latest.json 上传行为**
   - What we know: 自动生成并上传到 Release，URL 在 release draft 创建后即可访问
   - What's unclear: 两个矩阵 job 分别上传时是否会产生竞态（aarch64 和 x86_64 各自调用 action）
   - Recommendation: 参考 tauri-action 官方文档，使用 `releaseDraft: true` 后两个 job 追加到同一 draft release，tauri-action 处理幂等性

2. **ad-hoc 签名后 updater 重启权限**
   - What we know: Issue #2273 报告 relaunch() 在 ad-hoc 签名 app 中出现 `Operation not permitted (os error 1)`
   - What's unclear: 是否影响本项目（Tauri 2.10+）
   - Recommendation: 实现后在本地 ad-hoc 签名 build 中测试 relaunch()；备选：引导用户手动重启

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test（Rust），无独立前端测试框架 |
| Config file | src-tauri/Cargo.toml（dev-dependencies） |
| Quick run command | `cd src-tauri && cargo test --lib 2>&1 \| tail -5` |
| Full suite command | `cd src-tauri && cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REL-01 | tauri.conf.json 无 version 字段，Cargo.toml 有 | manual-only | 人工检查两文件 | N/A |
| SIGN-02 | Ed25519 密钥对已生成，.pub 文件存在 | manual-only | `ls ~/.tauri/climanager.key*` | N/A |
| SIGN-03 | GitHub Secret 存在，pubkey 写入 tauri.conf.json | manual-only | GitHub Settings UI 检查 | N/A |
| SIGN-01 | CI 构建产物有 ad-hoc 签名 | smoke | CI job 成功运行即验证 | ❌ Wave 1 CI 创建 |
| CICD-01 | 推送 tag 触发 CI | smoke | 推送 v0.2.0 tag 观察 | ❌ Wave 2 CI 创建 |
| CICD-02 | 双架构 DMG 生成 | smoke | CI artifacts 检查 | ❌ Wave 2 CI 创建 |
| CICD-03 | Release Draft 含产物 | smoke | GitHub Release UI 检查 | ❌ Wave 2 CI 创建 |
| UPD-01 | Rust 插件注册无编译错误 | unit | `cd src-tauri && cargo build 2>&1` | ❌ Wave 2 |
| UPD-02 | check() 可调用（集成测试需网络） | manual-only | 启动 app 观察日志 | N/A |
| UPD-03 | UpdateDialog 渲染正确 | manual-only | 本地 mock update 测试 | N/A |
| UPD-04 | downloadAndInstall + relaunch 流程 | manual-only | 完整 CI 发版后验证 | N/A |
| REL-02 | /ship 命令正确 bump 版本 | unit | `bash .claude/commands/ship.sh --dry-run patch` | ❌ Wave 2 |
| REL-03 | Release Notes 包含 Gatekeeper 指引 | manual-only | 检查 release.yml 模板 | N/A |

### Sampling Rate
- **Per task commit:** `cd /Users/kelin/Workspace/CLIManager/src-tauri && cargo check`（编译检查，30 秒内）
- **Per wave merge:** `cd /Users/kelin/Workspace/CLIManager/src-tauri && cargo test`（221 个现有测试）
- **Phase gate:** 全套测试绿色 + 手动验证清单通过后运行 `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `.github/workflows/release.yml` — 覆盖 CICD-01/02/03、SIGN-01（Wave 2 Plan 12-02 创建）
- [ ] `src/components/updater/UpdateDialog.tsx` — 覆盖 UPD-03（Wave 2 Plan 12-03 创建）
- [ ] `src/components/updater/useUpdater.ts` — 覆盖 UPD-02（Wave 2 Plan 12-03 创建）
- [ ] `.claude/commands/ship.md` — 覆盖 REL-02（Wave 2 Plan 12-04 创建）

---

## Sources

### Primary (HIGH confidence)
- [Tauri Updater Plugin 官方文档](https://v2.tauri.app/plugin/updater/) — 配置结构、API、密钥生成命令
- [Tauri Process Plugin 官方文档](https://v2.tauri.app/plugin/process/) — relaunch() 注册与权限
- [tauri-apps/plugins-workspace guest-js/index.ts](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/guest-js/index.ts) — DownloadEvent TypeScript 类型定义
- [Tauri macOS 签名文档](https://v2.tauri.app/distribute/sign/macos/) — ad-hoc signingIdentity="-" 配置

### Secondary (MEDIUM confidence)
- [tauri-action GitHub 仓库 README](https://github.com/tauri-apps/tauri-action) — tauri-action@v1 输入参数
- [Tauri GitHub Pipeline 文档](https://v2.tauri.app/distribute/pipelines/github/) — 双架构矩阵 YAML 示例
- [Tauri orgs Discussion #6347](https://github.com/orgs/tauri-apps/discussions/6347) — Cargo.toml 作为唯一版本源社区验证

### Tertiary (LOW confidence)
- [Bug #13804](https://github.com/tauri-apps/tauri/issues/13804) — ad-hoc DMG 随机失败，workaround 来自社区，官方未修复
- [Bug #13485](https://github.com/tauri-apps/tauri/issues/13485) — 私钥密码 env var 注入失败，官方确认 bug，project 已决策规避

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — 官方文档直接验证，插件版本明确
- Architecture: HIGH — 模式来自官方 YAML 示例，已在 cc-switch 参考实现中见过
- Pitfalls: MEDIUM-HIGH — Bug #13804/#13485 有官方 issue 确认，workaround 来自社区

**Research date:** 2026-03-14
**Valid until:** 2026-06-14（Tauri 生态活跃，tauri-action 版本变更可能影响 latest.json 格式）
