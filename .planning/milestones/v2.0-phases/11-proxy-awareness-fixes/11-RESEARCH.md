# Phase 11: 代理感知修复与文档同步 - Research

**Researched:** 2026-03-14
**Domain:** Rust/Tauri — tray.rs 代理感知、_update_provider_in 代理感知、文档同步
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| LIVE-01 (integration fix) | 代理模式下切换 Provider 无感知中断 | 托盘菜单切换路径目前直接调用 `_set_active_provider_in`（不感知代理），需改为检查 `proxy_takeover` 并走代理感知分支 |
| LIVE-03 (integration fix) | Provider CRUD 自动更新代理内存 | `_update_provider_in` 内部无条件调用 `patch_provider_for_cli`，代理模式下会短暂覆盖 `PROXY_MANAGED`，需在调用处增加代理模式判断 |
| UX-01 (doc sync) | 端口占用清晰错误提示 | 功能已完整实现，仅需更新 `REQUIREMENTS.md` 复选框和 `10-02-SUMMARY.md` 前言字段 |
</phase_requirements>

---

## Summary

Phase 11 是一个**集成修复 + 文档同步**阶段，不需要新增功能，只需修复两处代码路径的代理感知缺失，并补齐审计发现的文档差距。

**Bug 1（HIGH）：** `tray.rs handle_provider_click` 调用 `_set_active_provider_in`（非代理感知），代理模式下会用真实凭据覆盖 `PROXY_MANAGED`，导致 CLI 绕过代理直接使用真实 API key。该函数在 `spawn_blocking` 同步上下文中运行，修复需要改为 `async` spawn（使用 `tauri::async_runtime::spawn`）。

**Bug 2（MEDIUM）：** `update_provider` Tauri 命令调用 `_update_provider_in`，后者无条件调用 `patch_provider_for_cli`，代理模式下短暂用真实凭据覆盖了 `PROXY_MANAGED`（虽然随后在 `update_provider` 层正确调用了 `proxy_service.update_upstream()`）。修复需在 `_update_provider_in` 层或 `update_provider` 层跳过 patch。

**文档同步：** `REQUIREMENTS.md` 中 `UX-01` 复选框未更新（`[ ]` → `[x]`），`10-02-SUMMARY.md` 前言缺少 `requirements-completed` 字段。

**Primary recommendation:** 按 Bug 1 > Bug 2 > 文档同步顺序逐任务修复，每个 task 有独立自动化验证（`cargo test`），文档更新在最后一个 task 完成。

---

## 现有代码分析（HIGH confidence）

### Bug 1：托盘菜单切换路径

**文件：** `src-tauri/src/tray.rs`，`handle_provider_click` 函数（第 176–215 行）

**当前行为：**
```rust
fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
    tauri::async_runtime::spawn_blocking(move || {
        // ...
        match crate::commands::provider::_set_active_provider_in(  // <-- 不感知代理
            &providers_dir,
            &settings_path,
            cli_id.clone(),
            Some(provider_id.clone()),
            None,
        ) { ... }
    });
}
```

**问题：** `_set_active_provider_in` 内部无条件调用 `patch_provider_for_cli`，在代理模式下会将 `PROXY_MANAGED` 占位值覆盖为真实 API key，导致 CLI 绕过本地代理。

**已有的代理感知函数：** `set_active_provider` Tauri 命令（`commands/provider.rs` 第 572–601 行）已正确实现了代理感知分支：
- 检查 `proxy_takeover.cli_ids` 是否包含该 `cli_id`
- 代理模式下调用 `_set_active_provider_in_proxy_mode`：仅更新 `active_providers` + 调用 `proxy_service.update_upstream()`，不 patch 配置文件

**修复方案：** 将 `handle_provider_click` 从 `spawn_blocking` 改为 `tauri::async_runtime::spawn`（因为 `_set_active_provider_in_proxy_mode` 是 async 函数），在代理模式下调用 `_set_active_provider_in_proxy_mode`，直连模式下调用现有的 `_set_active_provider_in`。

**`_set_active_provider_in_proxy_mode` 签名（已存在，可直接调用）：**
```rust
pub(crate) async fn _set_active_provider_in_proxy_mode(
    providers_dir: &Path,
    local_settings_path: &Path,
    cli_id: String,
    provider_id: Option<String>,
    proxy_service: &crate::proxy::ProxyService,
) -> Result<LocalSettings, AppError>
```

**需要从 AppHandle 获取 ProxyService state：**
```rust
let proxy_service = app_handle.state::<crate::proxy::ProxyService>();
```

### Bug 2：编辑活跃 Provider 路径

**文件：** `src-tauri/src/commands/provider.rs`，`_update_provider_in` 函数（第 392–419 行）和 `update_provider` 命令（第 512–549 行）

**当前行为：**
```rust
fn _update_provider_in(...) -> Result<Provider, AppError> {
    // ...
    if is_active {
        if let Err(err) = patch_provider_for_cli(&provider.cli_id, &settings, &provider, adapter) { // <-- 无条件 patch
            // ...
        }
    }
    Ok(provider)
}
```

`update_provider` 命令在调用 `_update_provider_in` 后额外做了代理联动，但 `_update_provider_in` 内部已经发生了问题：在代理模式下，`patch_provider_for_cli` 将真实凭据写入 CLI 配置文件，短暂覆盖 `PROXY_MANAGED`。

**修复位置选择分析：**

两种修复方案：

**方案 A（在 `_update_provider_in` 内部修复）：** 向该函数传入代理模式信息，代理模式下跳过 `patch_provider_for_cli`。
- 优点：彻底修复，future callers 也会受保护
- 缺点：`_update_provider_in` 是内部可测试函数，修改签名影响测试代码

**方案 B（在 `update_provider` 命令层修复）：** 读取 proxy 状态，代理模式下跳过调用 `_update_provider_in` 时的 patch 部分，或把 `_update_provider_in` 改为仅保存文件，patch 留给调用方。

**推荐方案 B 的变体：** 在 `update_provider` 命令层，代理模式下：
1. 直接调用 `normalize_and_validate_provider`
2. 调用 `storage::icloud::save_existing_provider_to` 保存文件（不调用 `_update_provider_in`）
3. 调用 `proxy_service.update_upstream` 更新上游

但这会复制 `_update_provider_in` 的部分逻辑，不推荐。

**最佳方案：** 修改 `_update_provider_in` 接受一个布尔参数 `skip_patch: bool`，或者在 `update_provider` 命令层先检查代理模式，若在代理模式下：先保存文件（自行调用 `save_existing_provider_to`），再更新代理上游；否则走原有 `_update_provider_in` 路径。

审计报告给出的修复描述：
> `update_provider` 代理模式下应跳过 `patch_provider_for_cli`，仅保存 Provider 文件并更新代理上游

**审计推荐方案（最简洁）：** 在 `update_provider` 命令中，读取 `find_proxy_cli_ids_for_provider` 返回值（该函数已存在），若该 Provider 是代理模式 CLI 的活跃 Provider，则在调用 `_update_provider_in` 之前把 adapter 传为 `None` 并用一个"不 patch 的 adapter"，或者直接提前判断并分叉：
- 代理路径：`normalize_validate + save_file_only + update_upstream`
- 直连路径：`_update_provider_in`（现有逻辑不变）

**实际最简洁做法：** 在 `update_provider` 命令层，检查 Provider 是否在代理模式的 CLI 中是活跃 Provider：
```rust
// 代理模式 CLI 列表（已有函数）
let proxy_cli_ids = find_proxy_cli_ids_for_provider(&settings, &result.id);
if !proxy_cli_ids.is_empty() {
    // 代理模式：skip _update_provider_in 中的 patch，仅保存文件
    // ...
} else {
    // 直连模式：原有逻辑
}
```

然而 `_update_provider_in` 还包含了 `normalize_validate`、`updated_at` 时间戳更新、Provider 文件保存等逻辑，不能完全跳过。最干净的方法是增加 `is_proxy_mode_active: bool` 参数给 `_update_provider_in`：代理模式下跳过 `patch_provider_for_cli` 部分。

### 文档同步需求

**UX-01 文档差距（两个位置）：**

1. `.planning/REQUIREMENTS.md` 第 36 行：
   - 当前：`- [ ] **UX-01**: 启动代理时检测端口占用，端口冲突给出清晰错误提示`
   - 目标：`- [x] **UX-01**: 启动代理时检测端口占用，端口冲突给出清晰错误提示`

2. `.planning/phases/10-live-switching-ui/10-02-SUMMARY.md` 前言：
   - 当前：缺少 `requirements-completed` 字段
   - 目标：在 frontmatter 中添加 `requirements-completed: [UX-01]`

---

## Architecture Patterns

### 代理感知检测模式（HIGH confidence，来自现有代码）

项目中已有两种稳定的代理感知模式：

**模式 1：通过 proxy_takeover.cli_ids 检查（同步）**
```rust
// 用于检查某个 cli_id 是否在代理模式下
let in_proxy_mode = settings
    .proxy_takeover
    .as_ref()
    .map_or(false, |t| t.cli_ids.contains(&cli_id));
```

**模式 2：通过 find_proxy_cli_ids_for_provider 检查（同步）**
```rust
// 用于检查某个 provider 是否被代理模式 CLI 使用
let proxy_cli_ids = find_proxy_cli_ids_for_provider(&settings, &provider_id);
// 此函数已在 commands/provider.rs 中定义，pub(crate) 可见
```

**模式 3：spawn async vs spawn_blocking 选择**
- `spawn_blocking`：用于纯同步操作（文件读写、adapter.patch）
- `tauri::async_runtime::spawn`：用于包含 async 操作的代码块（proxy_service.update_upstream 是 async）

修复 Bug 1 时，因为需要调用 `_set_active_provider_in_proxy_mode`（async），必须从 `spawn_blocking` 改为 `tauri::async_runtime::spawn`。

### 测试模式（HIGH confidence，来自现有代码）

项目所有代理感知逻辑都通过提取纯函数来实现可测试性（`_in` 后缀模式）：
- 业务逻辑提取为带路径参数的 `_xxx_in` 函数
- Tauri 命令层只解析路径、调用 `_in` 函数、emit 事件
- 测试直接使用 `TempDir` 模拟文件系统

新测试需要：
1. Bug 1 修复：测试 `handle_provider_click` 代理感知逻辑（提取内部逻辑为可测试的纯函数）
2. Bug 2 修复：测试 `_update_provider_in` 代理模式下跳过 patch（扩展现有测试）

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 代理模式检测 | 自己实现 proxy_takeover 读取逻辑 | `find_proxy_cli_ids_for_provider`（已存在于 commands/provider.rs） | 复用现有逻辑，一致性 |
| 代理模式下切换 Provider | 重新实现切换逻辑 | `_set_active_provider_in_proxy_mode`（已存在） | 已验证正确，直接复用 |
| 上游更新 | 自己调用 ProxyService API | `proxy_service.update_upstream(&cli_id, upstream)` | 标准模式，10-01 已确立 |
| 从 AppHandle 获取 State | 自己管理 State | `app_handle.state::<ProxyService>()` | Tauri 标准模式 |

---

## Common Pitfalls

### 陷阱 1：spawn_blocking vs spawn async 混用
**什么情况出错：** `spawn_blocking` 内部调用 `.await` 会编译错误；`spawn` 内部调用 blocking 操作（如文件 I/O）会阻塞 tokio executor。
**为什么发生：** 现有 `handle_provider_click` 是 `spawn_blocking`，但修复后需要调用 async 函数 `_set_active_provider_in_proxy_mode`。
**如何避免：** 整个 `handle_provider_click` 改为 `tauri::async_runtime::spawn`（async block），因为 `_set_active_provider_in` 本身是同步的，混合在 async block 里没有问题。

### 陷阱 2：send bound（`Box<dyn CliAdapter + Send>`）
**什么情况出错：** `tauri::async_runtime::spawn` 的 future 必须满足 `Send` bound，如果 adapter 是 `Box<dyn CliAdapter>`（无 Send bound），会编译失败。
**为什么发生：** Phase 9 已遇到此问题，决策记录为 `adapter 参数使用 Box<dyn CliAdapter + Send>`。
**如何避免：** 托盘点击路径传 `adapter: None`（不需要 adapter），不受影响。`_set_active_provider_in_proxy_mode` 不接受 adapter 参数，直接调用即可。

### 陷阱 3：`_update_provider_in` 修改影响测试
**什么情况出错：** 修改 `_update_provider_in` 签名会破坏现有测试。
**为什么发生：** `_update_provider_in` 有专门测试。
**如何避免：** 修改方式使用默认值或在调用层（`update_provider` 命令）进行判断，保持 `_update_provider_in` 签名不变，或者添加参数时检查所有调用处（目前仅 `update_provider` 命令调用）。

### 陷阱 4：文档同步时的 frontmatter 格式
**什么情况出错：** SUMMARY.md frontmatter 格式不一致（已有字段不正确包含 `requirements-completed`）。
**为什么发生：** 不同 SUMMARY.md 的 frontmatter 格式略有不同。
**如何避免：** 参考 `10-01-SUMMARY.md` 的格式——该文件已有 `requirements-completed: [LIVE-01, LIVE-02, LIVE-03]`。

### 陷阱 5：tray.rs async block 中的生命周期问题
**什么情况出错：** 从 `app_handle.state::<ProxyService>()` 获取 State 的引用在 async block 中可能无法满足 `'static` bound。
**为什么发生：** `State<'_, T>` 的生命周期与 `AppHandle` 绑定，而 async block 可能需要 `'static`。
**如何避免：** 使用 `proxy_service.inner()` 获取 `Arc<T>`，或者 clone AppHandle 然后在 async block 内部调用 `app_handle.state::<ProxyService>()`。参考现有的 `update_provider` 命令（line 516）：`proxy_service: State<'_, crate::proxy::ProxyService>`——在 Tauri 命令中 State 被 tauri 框架管理，但在手动调用中需要从 `AppHandle` 获取。

正确模式（参考 `commands/provider.rs disable_proxy_for_deleted_providers` 第 701 行）：
```rust
let proxy_service = app_handle.state::<crate::proxy::ProxyService>();
```
该函数是 `pub(crate) async fn`，传入 `&tauri::AppHandle`，在内部用这种方式获取 state。`handle_provider_click` 已有 `app: &AppHandle`，只需将其移进 async block 前 clone，然后在 block 内调用。

---

## Code Examples

### 修复 Bug 1：tray.rs handle_provider_click

```rust
// 修复前（spawn_blocking，不感知代理）：
fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
    let app_handle = app.clone();
    let cli_id = cli_id.to_string();
    let provider_id = provider_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        // ...
        match crate::commands::provider::_set_active_provider_in(...) { ... }
    });
}

// 修复后（spawn async，感知代理）：
fn handle_provider_click(app: &AppHandle, cli_id: &str, provider_id: &str) {
    let app_handle = app.clone();
    let cli_id = cli_id.to_string();
    let provider_id = provider_id.to_string();
    tauri::async_runtime::spawn(async move {
        let providers_dir = match crate::storage::icloud::get_icloud_providers_dir() {
            Ok(d) => d,
            Err(e) => { log::error!("..."); update_tray_menu(&app_handle); return; }
        };
        let settings_path = crate::storage::local::get_local_settings_path();
        let settings = match crate::storage::local::read_local_settings_from(&settings_path) {
            Ok(s) => s,
            Err(e) => { log::error!("..."); update_tray_menu(&app_handle); return; }
        };
        let in_proxy_mode = settings
            .proxy_takeover
            .as_ref()
            .map_or(false, |t| t.cli_ids.contains(&cli_id));

        let result = if in_proxy_mode {
            let proxy_service = app_handle.state::<crate::proxy::ProxyService>();
            crate::commands::provider::_set_active_provider_in_proxy_mode(
                &providers_dir,
                &settings_path,
                cli_id.clone(),
                Some(provider_id.clone()),
                &proxy_service,
            ).await.map(|_| ())
        } else {
            crate::commands::provider::_set_active_provider_in(
                &providers_dir,
                &settings_path,
                cli_id.clone(),
                Some(provider_id.clone()),
                None,
            ).map(|_| ())
        };

        match result {
            Ok(()) => {
                log::info!("Tray: switched {cli_id} to {provider_id}");
                update_tray_menu(&app_handle);
                let _ = app_handle.emit("active-provider-changed", ...);
            }
            Err(e) => {
                log::error!("Tray switch failed: {e}");
                update_tray_menu(&app_handle);
            }
        }
    });
}
```

### 修复 Bug 2：update_provider 代理模式下跳过 patch

```rust
// update_provider 命令层（修复后）：
#[tauri::command]
pub async fn update_provider(
    app_handle: tauri::AppHandle,
    provider: Provider,
    proxy_service: State<'_, crate::proxy::ProxyService>,
) -> Result<Provider, AppError> {
    let provider = normalize_and_validate_provider(provider)?;
    let dir = crate::storage::icloud::get_icloud_providers_dir()?;
    let settings_path = crate::storage::local::get_local_settings_path();

    let tracker = app_handle.state::<crate::watcher::SelfWriteTracker>();
    tracker.record_write(dir.join(format!("{}.json", provider.id)));

    // 检查是否代理模式下的活跃 Provider
    let settings = crate::storage::local::read_local_settings_from(&settings_path)?;
    let proxy_cli_ids = find_proxy_cli_ids_for_provider(&settings, &provider.id);

    let result = if !proxy_cli_ids.is_empty() {
        // 代理模式：仅保存文件，跳过 patch_provider_for_cli
        _update_provider_in_without_patch(&dir, &settings_path, provider)?
    } else {
        // 直连模式：原有逻辑
        _update_provider_in(&dir, &settings_path, provider, None)?
    };

    // 代理上游更新（和原来一样）
    for cli_id in proxy_cli_ids { ... }

    Ok(result)
}
```

**注意：** 不建议新建 `_update_provider_in_without_patch`，而是在 `_update_provider_in` 中增加 `skip_patch: bool` 参数，或者直接在 `update_provider` 命令中复用文件保存逻辑（调用 `save_existing_provider_to`）。

最简洁的实际做法是修改 `_update_provider_in` 签名增加 `skip_patch` 参数。现有测试中 `_update_provider_in` 的调用需要同步更新。

---

## State of the Art

| 现有实现 | Phase 11 修复 | 影响 |
|---------|--------------|------|
| tray.rs 使用 spawn_blocking + 非代理感知函数 | 改为 spawn async + 代理感知分支 | LIVE-01 托盘路径完整 |
| _update_provider_in 无条件 patch | 代理模式下跳过 patch | LIVE-03 编辑路径不再短暂覆盖 PROXY_MANAGED |
| UX-01 文档标记 Pending | REQUIREMENTS.md [x] + 10-02-SUMMARY requirements-completed | 文档与实现一致 |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust: cargo test（lib tests） |
| Config file | `src-tauri/Cargo.toml` |
| Quick run command | `cd src-tauri && cargo test --lib commands::provider -- --test-threads=1` |
| Full suite command | `cd src-tauri && cargo test --lib -- --test-threads=1` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| LIVE-01 (tray fix) | 托盘菜单代理模式下不覆盖 PROXY_MANAGED | unit | `cargo test --lib tray -- --test-threads=1` | 需要新建测试（Wave 0 任务） |
| LIVE-03 (update fix) | 编辑活跃 Provider 代理模式下不 patch CLI 配置 | unit | `cargo test --lib commands::provider -- --test-threads=1` | 扩展现有测试 |
| UX-01 (doc) | 文档复选框和 SUMMARY 字段正确 | manual | N/A（文档检查） | N/A |

### Wave 0 Gaps

- [ ] tray 模块代理感知测试 — 需要提取 `handle_provider_click` 内部逻辑为可测试的纯函数（参考项目 `_in` 模式），测试代理模式下不调用 `patch_provider_for_cli`
- [ ] `_update_provider_in` 代理模式参数扩展测试 — 若修改签名，需更新现有测试覆盖 `skip_patch=true` 的分支

若 tray 测试难以单元化（涉及 `AppHandle` mock），可以接受 integration 级别的手工验证，但推荐至少提取 `is_proxy_mode_active` 逻辑为可测试的独立函数。

### Sampling Rate

- 每个 task commit 后：`cd src-tauri && cargo test --lib commands -- --test-threads=1`
- 每个 wave 合并后：`cd src-tauri && cargo test --lib -- --test-threads=1`
- Phase gate（verify-work 前）：全套测试通过

---

## Open Questions

1. **`_update_provider_in` 修改策略**
   - 已知：该函数在代理模式下无条件调用 `patch_provider_for_cli`
   - 不确定：最优修改方式——增加 `skip_patch` 参数 vs 在 `update_provider` 命令层完全分叉路径
   - 建议：增加 `skip_patch: bool` 参数，代理模式下传 `true`。现有唯一调用处是 `update_provider` 命令（直连路径传 `false`）。

2. **tray.rs 单元测试策略**
   - 已知：`handle_provider_click` 依赖 `AppHandle`，难以直接单元测试
   - 不确定：是否需要提取纯逻辑函数
   - 建议：提取 `determine_tray_switch_mode(settings: &LocalSettings, cli_id: &str) -> TraySwithMode` 这样的纯函数，配合 `integration` 手工验证；或者接受代理感知行为的测试由 `_set_active_provider_in_proxy_mode` 的已有测试覆盖（该函数已在 Phase 9/10 测试）。

---

## Sources

### Primary（HIGH confidence）

- `src-tauri/src/tray.rs` — 直接审查 Bug 1 的代码位置和修复上下文
- `src-tauri/src/commands/provider.rs` — 直接审查 Bug 2 的代码位置、`find_proxy_cli_ids_for_provider`、`_set_active_provider_in_proxy_mode` 接口
- `src-tauri/src/commands/proxy.rs` — 直接审查代理模式感知函数签名和模式
- `.planning/v2.0-MILESTONE-AUDIT.md` — 官方审计报告，定义两个 integration gap 及修复方案
- `.planning/REQUIREMENTS.md` — 直接确认 UX-01 文档状态
- `.planning/phases/10-live-switching-ui/10-02-SUMMARY.md` — 直接确认缺少 `requirements-completed` 字段

### Decisions from History（HIGH confidence）

- `[09-01]`：`proxy_enable` 失败时回滚 CLI 配置为真实凭据（不留半成品）
- `[09-01]`：`set_active_provider` 代理模式判断放在 Tauri 命令层（非 `_in` 函数层）
- `[10-01]`：代理联动失败仅 log 不阻塞正常流程
- `[v2.0 research]`：takeover 标志持久化实现崩溃恢复

---

## Metadata

**Confidence breakdown:**
- Bug 1 修复方案（tray.rs）: HIGH — 代码已读，修复路径清晰，参考函数已存在
- Bug 2 修复方案（update_provider）: HIGH — 代码已读，修复策略明确
- 文档同步: HIGH — 两处文档位置已确认，改动内容确定
- 测试策略: MEDIUM — tray 模块单元测试策略尚需在 plan 阶段最终确定

**Research date:** 2026-03-14
**Valid until:** 稳定（代码基础确定，修复目标固定）
