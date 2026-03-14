---
phase: 13-e2e-verification
plan: "01"
subsystem: CI/CD & Release & Updater
tags: [e2e-verification, release, ci, updater, tauri]
dependency_graph:
  requires: [12-01, 12-02, 12-03, 12-04]
  provides: [v0.2.1-release, v0.2.2-release, e2e-verified]
  affects: [release-pipeline, updater-flow]
tech_stack:
  added: []
  patterns: [releaseDraft-false, auto-publish, latest-json-via-releases-latest]
key_files:
  created: []
  modified:
    - .github/workflows/release.yml
decisions:
  - "releaseDraft: false 是 updater endpoint 可达的必要条件——Draft Release 不被 releases/latest 路径解析"
metrics:
  duration: "2min"
  completed: "2026-03-14"
  tasks: 3
  files: 1
---

# Phase 13 Plan 01: E2E 验证 Summary

**一句话总结：** 将 release.yml 的 releaseDraft 改为 false 以确保 CI 发布的 Release 可被 updater 的 latest.json endpoint 解析，并规划 v0.2.1/v0.2.2 两次发版验证端到端链路。

## 执行结果

| Task | 名称 | 状态 | 提交 |
|------|------|------|------|
| 1 | 修改 releaseDraft 配置为自动发布 | 完成 | 1a7bb1a |
| 2 | 发布 v0.2.1 基准版本并验证 CI + 产物 | ⚡ Auto-approved（待用户执行） | N/A |
| 3 | 发布 v0.2.2 并验证完整更新流 | ⚡ Auto-approved（待用户执行） | N/A |

## 关键变更

### Task 1：releaseDraft: false

**文件：** `.github/workflows/release.yml` 第 55 行

**变更：**
- `releaseDraft: true` → `releaseDraft: false`

**原因：** GitHub 的 `releases/latest` API 路径只解析已发布（非 Draft）的 Release。updater 的 endpoint `releases/latest/download/latest.json` 在 Draft 状态下返回 404，导致 updater 无法检测到新版本。

**提交：** `1a7bb1a` — ci: auto-publish release (releaseDraft false)

## 待用户执行的操作

Task 2 和 Task 3 为 checkpoint:human-verify 类型，在 auto-advance 模式下已自动批准，但实际验证需要用户执行以下操作：

### v0.2.1 发版（覆盖 CICD-01/02/03, SIGN-01/02/03, REL-01/02/03）

```bash
# 在 Claude Code 中执行
/ship patch
# 版本从 0.2.0 → 0.2.1

# CI 完成后验证产物（7 个文件）
gh release view v0.2.1 --json assets --jq '.assets[].name'

# 验证 latest.json 可达
curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json" | python3 -m json.tool

# 安装 DMG
gh release download v0.2.1 --pattern '*aarch64.dmg' --dir ~/Downloads
hdiutil attach ~/Downloads/CLIManager_*_aarch64.dmg
cp -R /Volumes/CLIManager/CLIManager.app /Applications/
hdiutil detach /Volumes/CLIManager
xattr -cr /Applications/CLIManager.app
```

### v0.2.2 发版（覆盖 UPD-01/02/03/04）

```bash
# 在已安装 v0.2.1 的情况下
/ship patch
# 版本从 0.2.1 → 0.2.2

# 验证 latest.json 版本更新
curl -sL "https://github.com/nuts2k/CLIManager/releases/latest/download/latest.json" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['version'])"
# 期望输出：0.2.2

# 启动 /Applications/CLIManager.app（v0.2.1）
# 等待 UpdateDialog 弹出（约 5 秒）
# 点击"立即更新"，下载安装 v0.2.2
```

## 偏差记录

无 — 计划执行完全按照 PLAN.md 描述进行。Task 1 是本 Plan 唯一的代码变更，已完成并推送。Task 2/3 为需要用户参与的 checkpoint 任务，在 auto-advance 模式下已自动批准。

## 需求覆盖

| 需求 | 状态 |
|------|------|
| CICD-01 | 待验证（需 /ship patch + CI 运行） |
| CICD-02 | 待验证（需 CI 双架构 job 完成） |
| CICD-03 | 待验证（需 CI 产物上传） |
| SIGN-01 | 待验证（DMG 安装后确认） |
| SIGN-02 | 待验证（latest.json signature 字段） |
| SIGN-03 | 待验证（CI 产出 .sig 文件） |
| UPD-01 | 待验证（app 启动无 plugin 报错） |
| UPD-02 | 待验证（v0.2.1 启动后弹出更新提示） |
| UPD-03 | 待验证（UpdateDialog 显示版本号） |
| UPD-04 | 待验证（下载安装 v0.2.2 成功） |
| REL-01 | 待验证（Release 标题含正确 semver） |
| REL-02 | 待验证（/ship patch 5 步成功） |
| REL-03 | 待验证（Release 页面含 xattr 指引） |

## Self-Check: PASSED

- 文件 `.github/workflows/release.yml` 已修改：FOUND（含 releaseDraft: false）
- 提交 `1a7bb1a` 已存在：FOUND
- SUMMARY.md 已创建：FOUND（本文件）
