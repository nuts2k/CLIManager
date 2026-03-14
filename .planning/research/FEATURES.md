# Feature Research

**Domain:** Release Engineering — Tauri 2 macOS 桌面应用的 CI/CD、代码签名与自动更新
**Researched:** 2026-03-14
**Confidence:** HIGH（Tauri 官方文档 + tauri-action GitHub 仓库 + 社区实战文章 + macOS Gatekeeper 官方行为验证）

---

## Feature Landscape

### Table Stakes（用户预期的基本功能）

这些特性是发版工程的基线。缺少任何一项 = 发版流程不完整，app 无法持续迭代分发。

| Feature | 为何必须 | 复杂度 | 备注 |
|---------|---------|--------|------|
| GitHub Actions tag 触发构建 | push git tag → 自动出 DMG，没有这条工程流水线就无法持续分发 | LOW | `on: push: tags: 'v*'`；用 `tauri-apps/tauri-action@v0`；matrix 跑 macOS aarch64 + x86_64 |
| tauri-action 构建 + 发布 Release | 官方 action，自动编译、签名（如有）、上传 artifacts、创建 GitHub Release | LOW | 产出：DMG（Intel + Apple Silicon）、`latest.json` 更新描述文件、`.app.tar.gz` + `.sig` 更新包 |
| Ad-hoc 代码签名 | Apple Silicon Mac 要求所有从网络下载的 app 必须签名才能运行（macOS 15+）；无签名 = 部分设备无法运行 | LOW | `APPLE_SIGNING_IDENTITY="-"`；不需要 Apple Developer 账号；仅需 CI 环境变量一行 |
| 版本号与 git tag 对齐 | `tauri.conf.json` version 必须与 tag 一致，否则 `latest.json` 中版本混乱，自动更新失效 | LOW | 最简方案：省略 `tauri.conf.json` 中的 version 字段，Tauri 自动读取 `Cargo.toml` 的版本；发版脚本统一修改 `Cargo.toml` |
| `latest.json` 自动生成 + 上传 | `tauri-plugin-updater` 的静态端点依赖这个文件；`tauri-action` 自动生成并上传到 Release | LOW | 文件包含各平台的 `url`（下载地址）+ `signature`（`.sig` 文件内容）+ `version` + `pub_date` |
| 更新包签名密钥对（updater keypair） | `tauri-plugin-updater` 强制要求签名验证；没有密钥对 = 无法启用自动更新 | LOW | `tauri signer generate -w ~/.tauri/cli-manager.key`；公钥写入 `tauri.conf.json plugins.updater.pubkey`；私钥存 GitHub Secret 供 CI 使用 |
| tauri-plugin-updater 集成 | app 启动时检查更新，提示用户下载安装 | MEDIUM | 插件注册（Rust + JS 两端）；配置 endpoints 指向 GitHub Releases 的 `latest.json`；需配置 capabilities 权限 |
| Gatekeeper 用户引导 | Ad-hoc 签名不能绕过 Gatekeeper；用户首次打开会被拦截；必须提供清晰说明 | LOW | 在 Release Notes 和 README 中说明：前往「系统设置 → 隐私与安全性」点击「仍要打开」；macOS 15+ 不再支持 Control-click 直接打开 |

---

### Differentiators（竞争优势特性）

提升体验但非必须。与项目定位（macOS 开发者工具）高度相关。

| Feature | 价值主张 | 复杂度 | 备注 |
|---------|---------|--------|------|
| 发版一键脚本（bump + tag + push） | 消除手动改多文件版本号的烦恼；保证 `Cargo.toml` / `tauri.conf.json` / `package.json` 三者一致 | LOW | Shell 脚本：读取新版本号 → 更新 Cargo.toml → 提交 → 打 tag → push；触发 CI 自动构建 |
| CHANGELOG 自动生成（git-cliff） | 每次 Release 自动生成 changelog；规范 commit message（Conventional Commits） | LOW | `git-cliff` 是 Rust 编写的 changelog 生成器，与 Tauri 生态契合；配置 `cliff.toml`；生成内容填入 GitHub Release body |
| 自动更新 UI（进度展示）| 内置对话框简陋（无进度条、无"稍后提醒"）；自定义 UI 提升体验 | MEDIUM | 关闭 `tauri.conf.json` 中的内置 dialog；前端 React 实现进度条 + "现在更新/稍后提醒" 按钮；监听 `tauri-plugin-updater` 的 download progress 事件 |
| Release Draft 审核 | `releaseDraft: true` 先创建草稿，人工审核后手动 publish；防止构建错误直接对用户可见 | LOW | tauri-action 默认支持；Review artifacts → publish → 更新端点生效 |
| 双架构 DMG（Intel + Apple Silicon）| 同一 Release 分发两个架构的 DMG；用户无需关心架构 | LOW | matrix 中配置两个 macOS runner：`--target aarch64-apple-darwin` 和 `--target x86_64-apple-darwin` |

---

### Anti-Features（明确不做）

| Anti-Feature | 为何不做 | 替代方案 |
|--------------|---------|---------|
| Apple Developer 账号代码签名 + 公证（Notarization） | 需要付费 Developer 账号（$99/年）；CI 配置复杂（APPLE_CERTIFICATE / APPLE_ID / APPLE_TEAM_ID 等多个 secret）；公证需要网络往返 Apple 服务器，构建时间增加 5-10 分钟；v2.1 是内部工具阶段，ad-hoc 签名够用 | Ad-hoc 签名解决 Apple Silicon 运行问题；正式分发 v3.x 里程碑再升级 |
| Windows / Linux 构建 | 项目明确 macOS 优先（iCloud Drive 依赖）；跨平台构建 matrix 增加 CI 复杂度和构建时间 | 仅构建 macOS，CI 矩阵保持简单 |
| 自动推送至 macOS App Store | App Store 需要 Notarization + 沙箱限制；本地 HTTP 代理（axum 监听 127.0.0.1）在沙箱中受限 | GitHub Releases 直接分发；绕开 App Store 沙箱 |
| Tauri 内置 updater dialog（默认对话框） | 没有进度条、没有"稍后提醒"；UX 粗糙；且启用内置 dialog 后会禁用 JS 侧的更新事件 | 自定义更新 UI（Differentiator 中列出） |
| Sparkle 框架集成（tauri-plugin-sparkle-updater）| 虽然 native macOS 体验更好，但增加一个额外 native 依赖；`tauri-plugin-updater` 对于 macOS 单平台工具已足够；Sparkle 主要优势在多平台跨平台场景 | 标准 `tauri-plugin-updater` + 自定义 React UI |
| release-please / semantic-release 自动化 | 这类工具解决大团队版本号管理问题；单人/小团队项目手写脚本更直接可控 | 简单 bump 脚本（< 30 行 shell） |
| CDN / S3 托管 latest.json | GitHub Releases 原生支持 + tauri-action 自动上传；额外 CDN 增加运维复杂度而无收益 | GitHub Releases 直接作为 update endpoint |

---

## Feature Dependencies

```
[GitHub Actions 工作流]
    └──requires──> [tauri-action 工具链]
    └──requires──> [Cargo.toml 版本是 source of truth]
    └──requires──> [GitHub Token write 权限（contents: write）]

[tauri-action 构建]
    └──produces──> [DMG 安装包]
    └──produces──> [.app.tar.gz + .app.tar.gz.sig（更新包）]
    └──produces──> [latest.json（更新描述文件）]
    └──requires──> [APPLE_SIGNING_IDENTITY="-"（ad-hoc 签名）]
    └──requires──> [TAURI_SIGNING_PRIVATE_KEY（更新包签名私钥）]

[tauri-plugin-updater]
    └──requires──> [latest.json 可访问端点]
    └──requires──> [更新包签名密钥对（public key in tauri.conf.json）]
    └──requires──> [capabilities 权限配置（updater:default, process:allow-restart）]
    └──produces──> [用户更新提示 → 下载 → 安装 → 重启]

[自定义更新 UI]
    └──requires──> [tauri-plugin-updater 集成]
    └──requires──> [内置 dialog 关闭（tauri.conf.json plugins.updater.dialog: false）]
    └──requires──> [@tauri-apps/plugin-updater JS 绑定]
    └──requires──> [@tauri-apps/plugin-process（用于 relaunch）]

[发版脚本]
    └──requires──> [Conventional Commits 规范（git-cliff 依赖）]
    └──produces──> [版本 bump commit + git tag]
    └──triggers──> [GitHub Actions 工作流]

[CHANGELOG 生成（git-cliff）]
    └──requires──> [git 历史中的 Conventional Commits]
    └──produces──> [CHANGELOG.md + GitHub Release body]

[Ad-hoc 签名] ──enhances──> [macOS Apple Silicon 可运行性]
    └──does NOT bypass──> [Gatekeeper（用户仍需手动批准）]

[Apple Developer 签名 + Notarization] ──conflicts──> [Ad-hoc 签名]
    （两者互斥；v2.1 只做 ad-hoc）
```

### 依赖关键说明

- **更新包签名密钥对与 macOS 代码签名完全独立**：`TAURI_SIGNING_PRIVATE_KEY` 用于 tauri updater 验证更新包完整性（Ed25519 签名），与 `APPLE_SIGNING_IDENTITY` 的 macOS 代码签名是两个不同的签名机制，互不干扰。前者必须做，后者 ad-hoc 足够。
- **latest.json 是 tauri-plugin-updater 的依赖**：updater 插件从配置的 endpoint 拉取 `latest.json`，比较版本号，决定是否提示更新。此文件由 tauri-action 自动生成并上传到 GitHub Release。
- **tauri.conf.json version 必须与 git tag 一致**：`tauri-action` 读取 `tauri.conf.json`（或 fallback 到 `Cargo.toml`）的版本号生成 tag 占位符（`__VERSION__`）。如果版本号与 tag 不一致，`latest.json` 中的版本会错乱，自动更新无效。

---

## MVP Definition

### v2.1 Launch With（必做）

- [ ] **GitHub Actions 工作流** — tag 触发，macOS aarch64 + x86_64 双架构构建，创建 Release Draft
- [ ] **Ad-hoc 代码签名** — `APPLE_SIGNING_IDENTITY="-"`，Apple Silicon 运行能力，1 行 env var 搞定
- [ ] **更新包签名密钥对** — `tauri signer generate`，公钥入 `tauri.conf.json`，私钥入 GitHub Secret
- [ ] **tauri-plugin-updater 集成** — endpoints 指向 GitHub Releases 的 `latest.json`；app 启动时检查更新
- [ ] **发版脚本** — bump `Cargo.toml` → commit → tag → push；单命令触发全流程
- [ ] **Gatekeeper 用户引导文档** — Release Notes 中说明 macOS 安全提示处理步骤

### Add After Validation（v2.1.x）

- [ ] **CHANGELOG 自动生成（git-cliff）** — 条件：团队已稳定遵守 Conventional Commits 规范后才有意义
- [ ] **自定义更新 UI** — 条件：v2.1 核心更新流程验证可用后；内置 dialog 暂时够用
- [ ] **Release Draft → 审核发布流程** — 已通过 tauri-action `releaseDraft: true` 内建支持，不额外开发

### Future Consideration（v3.x）

- [ ] **Apple Developer 账号签名 + Notarization** — 条件：app 需要更广泛发布（非开发者用户），或 App Store 分发
- [ ] **Windows / Linux 分发** — 条件：iCloud 同步依赖解耦之后

---

## Feature Prioritization Matrix

| Feature | 用户价值 | 实现成本 | 优先级 |
|---------|---------|---------|--------|
| GitHub Actions 工作流（tauri-action） | HIGH | LOW | P1 |
| Ad-hoc 代码签名 | HIGH | LOW | P1 |
| 更新包签名密钥对生成 | HIGH | LOW | P1 |
| tauri-plugin-updater 集成（基本） | HIGH | MEDIUM | P1 |
| 发版 bump 脚本 | HIGH | LOW | P1 |
| Gatekeeper 用户引导文档 | HIGH | LOW | P1 |
| CHANGELOG 生成（git-cliff） | MEDIUM | LOW | P2 |
| Release Draft 审核流程 | MEDIUM | LOW | P2 |
| 自定义更新 UI（进度条 + 稍后提醒） | MEDIUM | MEDIUM | P2 |
| 双架构 DMG 分发 | MEDIUM | LOW | P1（matrix 配置即可） |
| Apple Developer 签名 + Notarization | LOW（当前阶段） | HIGH | P3 |

---

## 关键技术细节

### GitHub Actions 工作流结构

```yaml
on:
  push:
    tags: ['v*']

jobs:
  publish-tauri:
    permissions:
      contents: write        # 必须，否则 GITHUB_TOKEN 无权创建 Release
    strategy:
      matrix:
        include:
          - platform: macos-latest
            args: '--target aarch64-apple-darwin'
          - platform: macos-latest
            args: '--target x86_64-apple-darwin'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin,x86_64-apple-darwin
      - uses: swatinem/rust-cache@v2
      - uses: actions/setup-node@v4
        with: { node-version: lts/*, cache: npm }
      - run: npm ci
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_SIGNING_IDENTITY: '-'
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__          # __VERSION__ 自动替换为 tauri.conf.json 中的版本
          releaseName: 'CLIManager v__VERSION__'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

**关键点：**
- `__VERSION__` 占位符由 tauri-action 自动替换为 `tauri.conf.json`（或 fallback `Cargo.toml`）中的版本号
- 如果省略 `tauri.conf.json` 中的 `version` 字段，Tauri 自动读取 `Cargo.toml [package] version`——这是最简洁的单一来源策略
- 两个 macOS matrix entry 分别针对 Apple Silicon 和 Intel，生成独立的 DMG 和更新包
- `APPLE_SIGNING_IDENTITY: '-'` 是 ad-hoc 签名的完整配置，不需要任何证书文件

### tauri-plugin-updater 端点配置

`tauri.conf.json` 中：
```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/{owner}/{repo}/releases/latest/download/latest.json"
      ],
      "pubkey": "内容来自 ~/.tauri/cli-manager.key.pub",
      "dialog": false
    }
  }
}
```

`latest.json` 格式（由 tauri-action 自动生成）：
```json
{
  "version": "v2.1.0",
  "notes": "Release notes text",
  "pub_date": "2026-03-14T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "url": "https://github.com/.../CLIManager_2.1.0_aarch64.app.tar.gz",
      "signature": ".sig 文件的内容（字符串）"
    },
    "darwin-x86_64": {
      "url": "https://github.com/.../CLIManager_2.1.0_x86_64.app.tar.gz",
      "signature": ".sig 文件的内容（字符串）"
    }
  }
}
```

### Ad-hoc 签名 vs 不签名的实际差异

| 场景 | 无签名 | Ad-hoc 签名 |
|------|--------|------------|
| Apple Silicon 运行 | 无法运行（macOS 15+ 强制要求签名） | 可运行（签名验证通过） |
| Intel Mac 运行 | 可运行，但被 Gatekeeper 拦截 | 可运行，但被 Gatekeeper 拦截 |
| Gatekeeper 警告 | "无法验证开发者" | "无法验证开发者"（警告措辞略有不同，但同样需要手动批准） |
| 用户操作 | 系统设置 → 隐私与安全性 → 仍要打开 | 系统设置 → 隐私与安全性 → 仍要打开 |
| macOS 15 Control-click | 已移除（Sequoia 15.0 取消此快捷方式） | 已移除 |
| Gatekeeper 彻底绕过 | 不可（需 Notarization） | 不可（需 Notarization） |

**结论：** Ad-hoc 签名是 Apple Silicon 的运行前提，但两种情况都需要用户在「系统设置」中手动批准一次。首次安装摩擦不可避免，靠清晰文档引导解决。

### 更新用户体验流程

**使用 tauri-plugin-updater（基本流程）：**
1. app 启动时 JS 调用 `check()` → 请求 endpoints 中的 `latest.json`
2. 比较 `latest.json` 中的 version 与当前 app version（semver 比较，必须是更大的版本才触发）
3. 有更新 → 下载 `.app.tar.gz`（macOS），验证 `.sig` 签名（Ed25519）
4. 签名验证通过 → 解压替换 `.app`，调用 `relaunch()` 重启
5. 无更新 → 静默

**内置 dialog 模式（`dialog: true`）：**
- 显示 release notes + "安装并重启" / "取消" 两个按钮
- 没有进度条，没有"稍后提醒"
- 一旦启用内置 dialog，JS 端的更新事件不触发

**自定义 UI 模式（`dialog: false`）：**
- 完全自定义：进度条、稍后提醒、更新通知徽章等
- 需要自己处理 `downloadProgress` 事件和 `relaunch()`

---

## Sources

- [Tauri v2 GitHub Actions 官方文档](https://v2.tauri.app/distribute/pipelines/github/) — 工作流结构、matrix 配置 [HIGH]
- [tauri-apps/tauri-action GitHub 仓库](https://github.com/tauri-apps/tauri-action) — `__VERSION__` 占位符、`tagName`/`releaseDraft` 参数 [HIGH]
- [tauri-plugin-updater 官方文档](https://v2.tauri.app/plugin/updater/) — endpoints 配置、JSON 格式、签名机制 [HIGH]
- [Tauri v2 macOS 代码签名官方文档](https://v2.tauri.app/distribute/sign/macos/) — Ad-hoc 签名配置、Apple Developer 签名对比 [HIGH]
- [Ship Your Tauri v2 App Like a Pro: Part 1（代码签名）](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-code-signing-for-macos-and-windows-part-12-3o9n) — 实战签名配置 [MEDIUM]
- [Ship Your Tauri v2 App Like a Pro: Part 2（发布自动化）](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — release bump 脚本 + CI 触发 [MEDIUM]
- [How to make automatic updates work with Tauri v2 and GitHub](https://thatgurjot.com/til/tauri-auto-updater/) — latest.json 端点配置实例 [MEDIUM]
- [Native macOS Updates in Tauri Shouldn't Be This Hard](https://yuexun.me/native-macos-updates-in-tauri/) — 内置 dialog 局限性分析；Sparkle 对比 [MEDIUM]
- [tauri-apps/tauri discussions #6347: version sync](https://github.com/tauri-apps/tauri/discussions/6347) — `tauri.conf.json` version 省略后 fallback 到 `Cargo.toml` 的官方确认 [HIGH]
- [Tauri 2 Configuration Files 官方文档](https://v2.tauri.app/develop/configuration-files/) — version 字段语义 [HIGH]
- [git-cliff 官方文档](https://git-cliff.org/) — changelog 生成工具，Rust 生态原生 [HIGH]
- [macOS Sequoia Gatekeeper 变更 (iDownloadBlog)](https://www.idownloadblog.com/2024/08/07/apple-macos-sequoia-gatekeeper-change-install-unsigned-apps-mac/) — Control-click bypass 移除确认 [HIGH]
- [macOS 15.1 unsigned app discussion (MacRumors)](https://forums.macrumors.com/threads/macos-15-1-completely-removes-ability-to-launch-unsigned-applications.2441792/) — 当前 Gatekeeper 行为 [MEDIUM]
- [HackMD: Generating TAURI_SIGNING_PRIVATE_KEY](https://hackmd.io/@zo-el/S1vu7IFrxx) — 密钥对生成步骤 [MEDIUM]

---
*Feature research for: Release Engineering (v2.1 milestone)*
*Researched: 2026-03-14*
