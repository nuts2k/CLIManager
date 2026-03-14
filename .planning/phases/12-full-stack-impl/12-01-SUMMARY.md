---
phase: 12-full-stack-impl
plan: 01
subsystem: infra
tags: [tauri, updater, ed25519, signing, release-engineering, tauri-plugin-updater, tauri-plugin-process]

# Dependency graph
requires: []
provides:
  - Cargo.toml 作为唯一版本来源（v0.2.0），tauri.conf.json 删除顶级 version 字段
  - tauri-plugin-updater 和 tauri-plugin-process Rust 端依赖 + lib.rs 插件注册
  - @tauri-apps/plugin-updater 和 @tauri-apps/plugin-process 前端依赖
  - capabilities/default.json 声明 updater:default 和 process:default 权限
  - Ed25519 密钥对（~/.tauri/climanager.key + ~/.tauri/climanager.key.pub）
  - tauri.conf.json 包含 pubkey、endpoints、createUpdaterArtifacts、macOS ad-hoc 签名配置
affects: [12-02-ci-cd, 12-03-updater-ui, 12-04-release-script]

# Tech tracking
tech-stack:
  added:
    - tauri-plugin-updater@2.10.0（Rust + 前端）
    - tauri-plugin-process@2.3.1（Rust + 前端）
  patterns:
    - Cargo.toml 唯一版本来源模式：tauri.conf.json 省略 version 字段，Tauri 自动回退读 Cargo.toml
    - Ed25519 minisign 签名模式：无密码私钥 + GitHub Secrets 存储
    - ad-hoc macOS 签名：signingIdentity "-"（开发阶段绕过 Apple 代码签名）

key-files:
  created: []
  modified:
    - src-tauri/Cargo.toml（版本 0.1.0→0.2.0，添加 updater/process 依赖）
    - src-tauri/tauri.conf.json（删除 version，添加 plugins.updater + bundle.createUpdaterArtifacts）
    - src-tauri/capabilities/default.json（添加 updater:default、process:default）
    - src-tauri/src/lib.rs（注册 tauri_plugin_updater 和 tauri_plugin_process）
    - package.json（添加前端 plugin-updater 和 plugin-process 依赖）

key-decisions:
  - "版本号统一：Cargo.toml version=0.2.0 是唯一来源，tauri.conf.json 不设 version"
  - "Ed25519 密钥无密码（规避 tauri-cli Bug #13485，交互式 tty 崩溃问题）"
  - "updater endpoints 指向 GitHub Releases latest.json，遵循 tauri-action@v1 格式"
  - "macOS signingIdentity '-' 实现 ad-hoc 签名，不依赖 Apple 开发者证书"

patterns-established:
  - "Cargo.toml 唯一版本来源：省略 tauri.conf.json version 字段，Tauri CLI 自动回退"
  - "无密码 Ed25519 密钥生成：pnpm tauri signer generate -p '' 绕过交互式 tty 崩溃"

requirements-completed: [REL-01, SIGN-02, SIGN-03]

# Metrics
duration: 12min
completed: 2026-03-14
---

# Phase 12 Plan 01: 发版基础设施 Summary

**Ed25519 minisign 密钥对生成配置完毕，tauri-plugin-updater/process 双端注册，Cargo.toml 0.2.0 成为唯一版本来源，cargo check 通过**

## Performance

- **Duration:** 12 min
- **Started:** 2026-03-14T08:00:00Z
- **Completed:** 2026-03-14T08:12:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Cargo.toml 版本升级至 0.2.0，成为唯一版本来源；tauri.conf.json 删除 version 字段实现回退
- tauri-plugin-updater 和 tauri-plugin-process 在 Rust 端依赖安装、lib.rs 注册、capabilities 权限声明、前端 npm 包全部到位
- Ed25519 密钥对生成（无密码，规避 Bug #13485），公钥写入 tauri.conf.json，cargo check 编译通过
- 输出 GitHub Secrets 配置指引（TAURI_SIGNING_PRIVATE_KEY），提醒用户备份私钥

## Task Commits

每个 task 均独立原子提交：

1. **Task 1: 版本来源统一 + 依赖安装 + 插件注册** - `b712462` (chore)
2. **Task 2: Ed25519 密钥生成与公钥写入** - `9aa8bc6` (feat)

## Files Created/Modified

- `src-tauri/Cargo.toml` - 版本 0.1.0→0.2.0，添加 tauri-plugin-updater、tauri-plugin-process 依赖
- `src-tauri/tauri.conf.json` - 删除顶级 version 字段，添加 bundle.createUpdaterArtifacts、bundle.macOS.signingIdentity、plugins.updater（pubkey + endpoints）
- `src-tauri/capabilities/default.json` - 追加 updater:default 和 process:default 权限
- `src-tauri/src/lib.rs` - 插入 tauri_plugin_updater::Builder::new().build() 和 tauri_plugin_process::init() 注册
- `package.json` - 添加 @tauri-apps/plugin-updater@2.10.0 和 @tauri-apps/plugin-process@2.3.1
- `pnpm-lock.yaml` - 锁文件更新

## Decisions Made

- **无密码密钥**：`-p ""` 参数绕过交互式 tty，规避 tauri-cli 已知 Bug #13485（`echo "" | ...` 方式会导致 panic）
- **ad-hoc 签名**：`signingIdentity: "-"` 在 macOS 上使用开发者本地签名，无需 Apple 证书
- **单一版本来源**：删除 tauri.conf.json version 字段后，Tauri 自动回退到 Cargo.toml，避免版本不一致

## Deviations from Plan

计划提示用 `echo "" | pnpm tauri signer generate ...` 绕过密码交互，但实测此方法导致 panic（Bug #13485）；改用 `-p ""` 参数直接传空密码，成功解决。这是已知 Bug 的已知规避方式，与计划意图一致，无范围扩大。

## Issues Encountered

- `echo "" | pnpm tauri signer generate` 方式触发 panic（`Device not configured`），改用 `pnpm tauri signer generate -p ""` 解决，与计划中备注的 Bug #13485 规避策略一致

## User Setup Required

**需要手动配置 GitHub Secrets：**

1. 读取私钥：`cat ~/.tauri/climanager.key`（两行内容：注释行 + base64 密钥行）
2. 前往：https://github.com/nuts2k/CLIManager/settings/secrets/actions
3. 新建 secret：Name = `TAURI_SIGNING_PRIVATE_KEY`，Value = 私钥文件完整内容（两行都粘贴）
4. **备份私钥**：将 `~/.tauri/climanager.key` 内容存入密码管理器或安全备份位置（私钥丢失将永久无法签名更新包）

## Next Phase Readiness

Wave 2 三个并行 plan 所需的全部依赖已就绪：
- `12-02-ci-cd`：可读取 Cargo.toml 版本、使用 TAURI_SIGNING_PRIVATE_KEY secret
- `12-03-updater-ui`：可引入 @tauri-apps/plugin-updater 前端包
- `12-04-release-script`：版本来源明确（Cargo.toml），可自动化发版流程

唯一待完成的人工步骤：用户需手动配置 GitHub Secrets（见上方 User Setup Required）。

---
*Phase: 12-full-stack-impl*
*Completed: 2026-03-14*

## Self-Check: PASSED

- SUMMARY.md: FOUND
- tauri.conf.json: FOUND
- Cargo.toml: FOUND
- Commit b712462: FOUND
- Commit 9aa8bc6: FOUND
