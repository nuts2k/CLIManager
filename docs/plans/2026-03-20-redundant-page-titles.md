# 移除监控页与设置页冗余页标题 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 移除监控页与设置页内容区左上角的重复页标题，只保留顶部主导航与页面内 Tabs，并顺手统一收紧两页顶部间距。

**Architecture:** 这是一次纯前端展示层微调，不改动状态流、业务逻辑或导航结构。实现方式是在 `TrafficPage` 与 `SettingsPage` 中分别删除顶部 `h2` 标题节点，并把“标题 + Tabs”的容器改成“仅 Tabs”的紧凑布局，确保两页顶部 spacing 保持一致。

**Tech Stack:** React 19、TypeScript、Tailwind CSS 4、react-i18next、Radix Tabs

---

## 实施前说明

- 本仓库可编辑主应用当前未配置独立的前端组件测试基础设施。
- `cc-switch/` 下虽然存在测试配置，但该目录是只读参考代码，**不能修改，也不能把本次实现建立在那套测试基础设施上**。
- 因此本次计划采用最小且合适的验证方式：
  1. 修改后运行 `npm run build`
  2. 启动应用后手动验证监控页与设置页顶部表现
- 本次改动是纯 UI 结构与样式减法，不应引入新的抽象、工具函数或共享组件。

### Task 1: 调整监控页顶部区域

**Files:**
- Modify: `src/components/traffic/TrafficPage.tsx:24-34`
- Verify: `src/components/traffic/TrafficPage.tsx`

**Step 1: 先写失败的验收检查清单**

在开始改代码前，把下面 3 条作为“失败中的验收条件”记录在工作备注里，并以当前界面为基线确认它们尚未满足：

- 监控页左上角仍显示 `traffic.title` 大标题
- Tabs 没有单独左对齐成紧凑顶栏
- 顶部区域仍然比目标状态更松

这一步的目的，是先定义“什么叫改完”，避免边改边想。

**Step 2: 手动确认当前界面确实不满足目标**

运行应用并进入监控页，确认当前仍能看到标题 `t("traffic.title")`，因此这组验收条件当前处于 FAIL 状态。

Run: `npm run dev`

Expected: 监控页顶部仍为“标题 + Tabs”结构，说明改动目标尚未实现。

**Step 3: 写最小实现，删除标题并收紧容器**

把 `TrafficPage` 顶部 JSX 从：

```tsx
<div className="flex items-center px-6 pt-4 pb-2 gap-4">
  <h2 className="text-lg font-bold">{t("traffic.title")}</h2>
  <TabsList variant="line">
    <TabsTrigger value="logs">{t("traffic.tabLogs")}</TabsTrigger>
    <TabsTrigger value="stats">{t("traffic.tabStats")}</TabsTrigger>
  </TabsList>
</div>
```

改成仅保留 Tabs 的紧凑结构，例如：

```tsx
<div className="px-6 pt-3 pb-1.5">
  <TabsList variant="line">
    <TabsTrigger value="logs">{t("traffic.tabLogs")}</TabsTrigger>
    <TabsTrigger value="stats">{t("traffic.tabStats")}</TabsTrigger>
  </TabsList>
</div>
```

要求：

- 删除 `h2`
- 不改 `Tabs` 默认值与切换逻辑
- 不改下面统计卡片、筛选器、表格区域的逻辑
- 只做本地 JSX 与 className 微调

**Step 4: 运行构建，确认最小实现通过**

Run: `npm run build`

Expected: 构建成功，没有因为删除标题或调整 JSX 造成类型错误。

**Step 5: 手动验证监控页通过验收**

重新打开监控页并检查：

- 左上角不再出现 `traffic.title` 大标题
- `logs / stats` Tabs 左对齐显示
- 顶部留白较之前收紧，但视觉不拥挤
- 切换 `logs / stats` 仍正常

**Step 6: 提交这个小步（如果正在按计划逐步提交）**

```bash
git add src/components/traffic/TrafficPage.tsx
git commit -m "refactor: simplify traffic page header"
```

### Task 2: 调整设置页顶部区域

**Files:**
- Modify: `src/components/settings/SettingsPage.tsx:221-232`
- Verify: `src/components/settings/SettingsPage.tsx`

**Step 1: 先写失败的验收检查清单**

在改设置页前，先定义当前未满足的目标：

- 设置页左上角仍显示 `settings.title` 大标题
- Tabs 还依附在“标题 + Tabs”的混合容器里
- 顶部 spacing 尚未与监控页目标样式对齐

**Step 2: 手动确认当前界面确实处于 FAIL 状态**

运行应用并进入设置页，确认当前仍可见 `t("settings.title")`。

Run: `npm run dev`

Expected: 设置页顶部仍为“标题 + Tabs”结构，因此验收条件当前未满足。

**Step 3: 写最小实现，删除标题并统一 spacing**

把 `SettingsPage` 顶部 JSX 从：

```tsx
<div className="flex items-center px-6 pt-4 pb-2 gap-4">
  <h2 className="text-lg font-bold">{t("settings.title")}</h2>
  <TabsList variant="line">
    <TabsTrigger value="general">{t("settings.tabGeneral")}</TabsTrigger>
    <TabsTrigger value="advanced">{t("settings.tabAdvanced")}</TabsTrigger>
    <TabsTrigger value="about">{t("settings.tabAbout")}</TabsTrigger>
  </TabsList>
</div>
```

改成与监控页一致的紧凑布局，例如：

```tsx
<div className="px-6 pt-3 pb-1.5">
  <TabsList variant="line">
    <TabsTrigger value="general">{t("settings.tabGeneral")}</TabsTrigger>
    <TabsTrigger value="advanced">{t("settings.tabAdvanced")}</TabsTrigger>
    <TabsTrigger value="about">{t("settings.tabAbout")}</TabsTrigger>
  </TabsList>
</div>
```

要求：

- 删除 `h2`
- 不改 Tabs 的 `defaultValue="general"`
- 不动通用 / 高级 / 关于三个 tab 内容
- 与监控页顶部 spacing 保持同一节奏

**Step 4: 运行构建，确认通过**

Run: `npm run build`

Expected: 构建成功，没有 JSX 或类型问题。

**Step 5: 手动验证设置页通过验收**

检查：

- 左上角不再出现 `settings.title` 大标题
- `general / advanced / about` Tabs 左对齐显示
- 顶部留白相对之前更紧凑
- 切换三个 tab 时内容正常显示

**Step 6: 提交这个小步（如果正在按计划逐步提交）**

```bash
git add src/components/settings/SettingsPage.tsx
git commit -m "refactor: simplify settings page header"
```

### Task 3: 统一两页顶部节奏并做最终验证

**Files:**
- Verify: `src/components/traffic/TrafficPage.tsx`
- Verify: `src/components/settings/SettingsPage.tsx`
- Verify: `src/components/layout/Header.tsx`

**Step 1: 对照检查两页顶部 className 是否一致**

逐项确认：

- 两页都不再含有顶部 `h2`
- 两页顶部容器使用同一套 `px / pt / pb`
- Tabs 都保持左对齐

如果不一致，只允许做最小的 className 对齐，不要抽象公共组件。

**Step 2: 运行最终构建验证**

Run: `npm run build`

Expected: 构建成功。

**Step 3: 做最终手动回归**

启动应用后依次验证：

Run: `npm run dev`

Expected:

- 顶部主导航仍可清晰表达当前位于“主页 / 监控 / 设置”哪个主视图
- 监控页顶部没有重复页面标题
- 设置页顶部没有重复页面标题
- 主页不受影响
- 两页顶部视觉节奏一致，没有出现新的空白、挤压或错位

**Step 4: 整理变更并提交最终结果**

```bash
git add src/components/traffic/TrafficPage.tsx src/components/settings/SettingsPage.tsx
git commit -m "refactor: remove redundant page titles"
```

## 完成定义

完成后应满足：

- 监控页与设置页不再在内容区顶部重复显示页面名
- 顶部主导航承担主视图定位职责
- 页面顶部只保留真正有用的二级 Tabs
- 两页顶部 spacing 一致且更紧凑
- `npm run build` 成功
- 手动验证通过
