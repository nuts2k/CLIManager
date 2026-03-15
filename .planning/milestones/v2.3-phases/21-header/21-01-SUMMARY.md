---
phase: 21-header
plan: "01"
subsystem: ui-layout
tags: [header, animation, brand, css-variables, transition]
dependency_graph:
  requires: []
  provides: [header-brand-visual, page-transition, card-state-transition]
  affects: [src/components/layout/Header.tsx, src/components/layout/AppShell.tsx, src/index.css]
tech_stack:
  added: []
  patterns: [css-custom-properties, tailwind-v4-theme, opacity-fade-transition]
key_files:
  created: []
  modified:
    - src/index.css
    - src/components/layout/Header.tsx
    - src/components/layout/AppShell.tsx
decisions:
  - "--header-bg 色值 0.160 0.02 275 介于 --background(0.145) 和 --card(0.178) 之间，微深即可，不需要强对比度"
  - "AppShell 改为始终渲染两个视图（absolute 叠放），用 opacity+pointer-events 实现过渡，避免卸载导致状态丢失"
  - "ProviderCard.tsx 不需要改动，已有 transition-all duration-200 覆盖 border/bg 过渡"
metrics:
  duration: "约 5 分钟"
  completed_date: "2026-03-15"
  tasks_completed: 2
  files_modified: 3
requirements: [VISU-02, VISU-04]
---

# Phase 21 Plan 01: Header 品牌视觉提升 + 全局微动效过渡 Summary

**一句话总结：** 为 Header 添加 Logo + 双色品牌名（CLI 橙色/Manager 白色）+ 层次背景色，并用纯 CSS opacity transition 实现页面切换 150ms 淡入淡出过渡。

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Header 品牌视觉提升 + --header-bg CSS 变量 | 52e26c7 | src/index.css, src/components/layout/Header.tsx |
| 2 | 页面切换淡入淡出 + ProviderCard 状态过渡确认 | f22f9d5 | src/components/layout/AppShell.tsx |

## What Was Built

### Task 1: Header 品牌视觉提升

**index.css:**
- `:root` 新增 `--header-bg: 0.160 0.02 275` — 介于 `--background`(0.145) 和 `--card`(0.178) 之间，提供轻微层次感
- `@theme inline` 新增 `--color-header-bg: oklch(var(--header-bg))` — 使 Tailwind 可用 `bg-header-bg` 工具类

**Header.tsx:**
- `bg-background` 改为 `bg-header-bg`，Header 背景比内容区略深
- 左侧添加 `<img src="/icon.png" className="size-5" />` 品牌 Logo（20px）
- "CLIManager" 拆为 `<span className="text-brand-accent">CLI</span><span>Manager</span>`：CLI 橙色，Manager 白色
- `font-semibold` 提升为 `font-bold`

### Task 2: 页面切换淡入淡出

**AppShell.tsx:**
- `<main>` 加 `relative`，高度由 `flex-1` 控制
- 两个视图（ProviderTabs / SettingsPage）改为**始终渲染**，用 `absolute inset-0` 叠放在同一位置
- `transition-opacity duration-150 ease-out` 实现 150ms 淡入淡出
- 隐藏视图加 `pointer-events-none` 防止拦截点击事件
- 保持两个视图的组件状态（Tab 选择、滚动位置等不因视图切换丢失）

**ProviderCard.tsx（无需改动）:**
- 已有 `transition-all duration-200` 覆盖 border-color 和 background-color 过渡
- 激活/取消激活的 border/bg 变化有 200ms 平滑过渡，无需额外处理

## Deviations from Plan

无 — 计划执行完全按预期进行。ProviderCard.tsx 确认无需修改，符合计划中"**无需实际代码改动**"的结论。

## Self-Check

- [x] `src/index.css` 已修改（--header-bg 变量）
- [x] `src/components/layout/Header.tsx` 已修改（Logo + 双色名称 + bg-header-bg）
- [x] `src/components/layout/AppShell.tsx` 已修改（opacity fade transition）
- [x] 两个 commits 已创建：52e26c7、f22f9d5
- [x] TypeScript 编译无错误（npx tsc --noEmit 无输出）
