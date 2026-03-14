---
phase: 16-responses-api-provider-ui
plan: "03"
subsystem: frontend-provider-ui
tags: [provider, ui, protocol-type, model-mapping, i18n]
dependency_graph:
  requires: []
  provides: [provider-three-protocol-ui, model-mapping-ui, upstream-fields-persistence]
  affects: [ProviderDialog, ProviderTabs, useProviders, provider-types]
tech_stack:
  added: []
  patterns: [react-controlled-form, conditional-rendering, array-to-record-transform]
key_files:
  created: []
  modified:
    - src/types/provider.ts
    - src/components/provider/ProviderDialog.tsx
    - src/components/provider/ProviderTabs.tsx
    - src/hooks/useProviders.ts
    - src/i18n/locales/en.json
    - src/i18n/locales/zh.json
decisions:
  - ProtocolType 删除旧 open_ai_compatible，新增 open_ai_chat_completions 和 open_ai_responses 两变体
  - 旧值 open_ai_compatible 在编辑模式 useEffect 中自动映射为 open_ai_chat_completions（向前兼容）
  - 模型映射 UI 放在高级设置 Collapsible 内，protocol_type Select 之后，条件渲染
  - Array → Record 转换在 ProviderTabs handleSave 中完成，Record → Array 在 ProviderDialog useEffect 中完成
  - placeholder 使用技术名称（gpt-4o / claude-sonnet-4-20250514），中英文相同
metrics:
  duration: "3 minutes"
  completed_date: "2026-03-14"
  tasks_completed: 2
  files_modified: 7
---

# Phase 16 Plan 03: Provider UI 三协议选择 + 模型映射配置 Summary

ProviderDialog 扩展为三协议下拉（Anthropic / OpenAI Chat Completions / OpenAI Responses），OpenAI 类型时显示默认目标模型输入框和动态模型名映射对列表，数据通过 upstream_model / upstream_model_map 字段传递至 Rust 后端。

## Tasks Completed

| # | Task | Commit | Key Files |
|---|------|--------|-----------|
| 1 | 前端类型更新 + ProviderDialog UI + 保存逻辑 | e7610a0 | provider.ts, ProviderDialog.tsx, ProviderTabs.tsx, useProviders.ts, en.json, zh.json |
| 2 | 验证 Provider UI 三协议选择 + 模型映射交互 | — | ⚡ Auto-approved (auto_advance=true) |

## What Was Built

### 1. 类型系统更新（`src/types/provider.ts`）

- `ProtocolType` 从二变体扩展为三变体：`"anthropic" | "open_ai_chat_completions" | "open_ai_responses"`
- `Provider` 接口新增 `upstream_model?: string | null` 和 `upstream_model_map?: Record<string, string> | null`

### 2. ProviderDialog UI 扩展（`src/components/provider/ProviderDialog.tsx`）

- `ProviderFormData` 新增 `upstreamModel: string` 和 `upstreamModelMap: ModelMapEntry[]`
- 协议下拉替换为三选项
- 编辑模式初始化：upstream 字段回显，`Record<string,string>` → `ModelMapEntry[]` 转换
- 旧值兼容：`open_ai_compatible` 自动映射为 `open_ai_chat_completions`
- 条件渲染模型映射区域（`showModelMapping` 变量，openai 两协议显示）
- 辅助函数：`addModelMapEntry`、`removeModelMapEntry`、`updateModelMapEntry`
- 映射列表每行：source 输入框 + target 输入框 + X 删除按钮；底部添加按钮

### 3. 保存逻辑扩展（`src/components/provider/ProviderTabs.tsx`）

- `handleSave` 中计算 `upstreamModel` 和 `upstreamModelMap`（Array → Record 转换，过滤空行）
- 创建模式：createProvider 后 updateProvider 时传入 upstream 字段
- 编辑模式：updateProvider 直接传入 upstream 字段

### 4. useProviders 扩展（`src/hooks/useProviders.ts`）

- `copyProvider` 和 `copyProviderTo` 中 updateProvider 调用传递 `upstream_model` 和 `upstream_model_map`

### 5. i18n 更新

新增中英文翻译键：
- `protocol.openAiChatCompletions`、`protocol.openAiResponses`
- `provider.upstreamModel`、`provider.modelMapping`、`provider.addMapping`、`provider.sourceModel`、`provider.targetModel`
- 删除旧 `protocol.openAiCompatible` key

## Deviations from Plan

None — 计划执行完全按预期进行，TypeScript 编译一次通过无错误。

## Self-Check: PASSED

- `src/types/provider.ts` — 包含 `open_ai_chat_completions` 三变体 ProtocolType
- `src/components/provider/ProviderDialog.tsx` — 模型映射 UI 已实现（> 300 行）
- `src/components/provider/ProviderTabs.tsx` — 保存逻辑传递 upstream_model 和 upstream_model_map
- 提交 e7610a0 存在
- TypeScript 编译无错误（`pnpm tsc --noEmit` 输出为空）
