---
phase: 19-provider
plan: "01"
subsystem: provider-dialog
tags: [ui, i18n, layout, form, validation]
dependency_graph:
  requires: []
  provides: [ProviderDialog-重构布局]
  affects: [src/components/provider/ProviderDialog.tsx]
tech_stack:
  added: []
  patterns: [三分区平铺表单, 固定Header/Footer滚动表单, i18n-placeholder命名空间]
key_files:
  created: []
  modified:
    - src/components/provider/ProviderDialog.tsx
    - src/i18n/locales/zh.json
    - src/i18n/locales/en.json
decisions:
  - "移除 Collapsible 改为三分区平铺，避免高级字段被折叠隐藏导致验证错误时不可见"
  - "upstreamModel 验证失败不再需要 setAdvancedOpen(true)，因为分区始终展开"
  - "Input 组件已内置 aria-invalid:border-destructive，无需在 ProviderDialog 中额外条件 className"
metrics:
  duration: "约 2 分钟"
  completed_date: "2026-03-15"
  tasks_completed: 2
  files_changed: 3
---

# Phase 19 Plan 01: ProviderDialog 布局重构 Summary

**一句话总结：** ProviderDialog 加宽至 max-w-xl + 移除 Collapsible 改三分区（基础信息/协议设置/模型配置）平铺 + 固定 Header/Footer 中间滚动 + 所有字段 placeholder 国际化。

## 完成情况

| 任务 | 名称 | 状态 | Commit |
| ---- | ---- | ---- | ------ |
| 1 | 对话框加宽 + 移除 Collapsible + 三分区平铺 + 固定 Header/Footer 滚动 | 完成 | 2621ffa |
| 2 | 字段 placeholder 国际化 + 验证错误红色边框 | 完成 | 57143d9 |

## 验证结果

- `npm run build` 编译通过（无 TypeScript/import 错误）
- ProviderDialog.tsx 中无 Collapsible 相关 import 或使用（count: 0）
- ProviderDialog.tsx 包含 `max-w-xl`（count: 1）
- ProviderDialog.tsx 包含三个分区标题（section.basic/protocol/model，count: 3）
- ProviderDialog.tsx 包含 `overflow-y-auto`（count: 1）
- ProviderDialog.tsx 包含 `aria-invalid`（count: 4）
- zh.json 和 en.json 均包含 section.* 和 placeholder.* 翻译 key

## Deviations from Plan

### Auto-fixed Issues

无 — 计划按原方案执行。

### 补充说明

验证错误红色边框无需在 ProviderDialog.tsx 中额外添加条件 className，因为 Input 组件（`src/components/ui/input.tsx` 第 13 行）已内置 `aria-invalid:border-destructive aria-invalid:ring-destructive/20` Tailwind 变体，只需保持 `aria-invalid={!!errors.xxx}` 属性即可自动触发。

## Self-Check: PASSED
