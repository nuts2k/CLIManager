---
phase: 20-tab
plan: "01"
subsystem: settings-ui
tags: [tabs, settings, i18n, about-section, logo]
dependency_graph:
  requires: []
  provides: [settings-tab-layout]
  affects: [SettingsPage, AboutSection]
tech_stack:
  added: []
  patterns: [Tabs line-variant, TabsContent overflow-auto]
key_files:
  created:
    - public/icon.png
  modified:
    - src/components/settings/SettingsPage.tsx
    - src/components/settings/AboutSection.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json
decisions:
  - "Tab 栏使用 variant=line 下划线风格，居左对齐（非居中的 default pill 风格）"
  - "通用 Tab 只含语言选择，保持简洁"
  - "高级 Tab 内区块间保留 Separator 分隔，移除原来的跨区块 Separator"
  - "关于 Tab 的 AboutSection 组件挂载时自动触发更新检查（原有行为保留）"
metrics:
  duration: "2m 12s"
  completed_date: "2026-03-15"
  tasks_completed: 2
  files_modified: 5
---

# Phase 20 Plan 01: 设置页 Tab 化 Summary

设置页从单列滚动长页重构为三 Tab 分组布局（通用/高级/关于），关于页顶部新增 64px 应用 Logo + 名称 + 版本号展示。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | 添加 Tab 翻译 key + 关于页 Logo | cf8448c | zh.json, en.json, AboutSection.tsx, public/icon.png |
| 2 | SettingsPage 重构为三 Tab 布局 | 562e2dd | SettingsPage.tsx |

## What Was Built

### Tab 布局重构

`SettingsPage.tsx` 完全重构为 Tabs 布局：
- **通用 Tab**（value="general"，defaultValue）：语言选择
- **高级 Tab**（value="advanced"）：代理模式开关、测试配置、导入 CLI 配置（有条件显示）
- **关于 Tab**（value="about"）：AboutSection 组件（含 Logo + 更新检查）

Tab 栏通过 `<TabsList variant="line">` 实现下划线风格，居左对齐，每个 `TabsContent` 设 `className="flex-1 overflow-auto p-6 space-y-6"` 支持内容独立滚动。

### 关于页 Logo 展示

`AboutSection.tsx` 顶部新增 Logo 区块（`flex flex-col items-center`）：
- `<img src="/icon.png" className="w-16 h-16 rounded-lg">` — 64px 应用图标
- 应用名 "CLIManager"（text-base font-semibold）
- 版本号（text-sm text-muted-foreground）

`public/icon.png` 从 `src-tauri/icons/icon.png` 复制而来，供 Tauri WebView 访问。

### 国际化

zh.json 和 en.json `settings` 对象新增三个翻译 key：
- `tabGeneral`：通用 / General
- `tabAdvanced`：高级 / Advanced
- `tabAbout`：关于 / About

## Deviations from Plan

无 - 计划按原定方案执行完毕。

## Verification

TypeScript 编译：`npx tsc --noEmit` 两次均无错误输出。

## Self-Check: PASSED
