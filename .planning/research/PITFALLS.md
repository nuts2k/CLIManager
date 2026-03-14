# Pitfalls Research

**Domain:** Tauri 2 macOS 应用发布工程（CI/CD + Ad-hoc 签名 + 自动更新）
**Researched:** 2026-03-14
**Confidence:** HIGH（结合 Tauri 官方文档、GitHub Issues #13804/#13485/#10217/#2610、官方 tauri-action README、macOS Gatekeeper 文档、GitHub Actions 计费文档）

---

## Critical Pitfalls

### Pitfall 1: Ad-hoc 签名在 CI 随机失败

**What goes wrong:**
在 GitHub Actions macOS runner 上使用 ad-hoc 签名（`signingIdentity: "-"`）时，`tauri build` 有时成功、有时以 `Signing with identity "-"` 后直接报错退出，没有明确的失败原因，且复现不稳定。由于每次 macOS CI 构建耗时 15-30 分钟，随机失败会严重浪费资源。

**Why it happens:**
这是已报告的 Tauri 已知 bug（[Issue #13804](https://github.com/tauri-apps/tauri/issues/13804)）。根本原因尚未完全确认，疑似与 GitHub macOS runner 环境差异（Intel vs ARM runner 行为不同）或 Tauri CLI 的 `codesign` 调用时序有关。在 macOS 15.x runner 上出现概率更高。

**How to avoid:**
- 优先选择稳定的替代方案：将 `signingIdentity` 设为 `"-"` 并在 `tauri.conf.json` 的 `bundle.macOS` 中声明，不要依赖环境变量传入。
- 在 CI workflow 中明确设置 `APPLE_SIGNING_IDENTITY="-"` 环境变量作为双保险。
- 添加 workflow 级别的重试逻辑：`continue-on-error: false`，但可以手动重跑失败的 job。
- 长期方案：随 Tauri 版本更新关注此 bug 的修复状态，可升级至修复版本。

**Warning signs:**
- CI 日志出现 `Signing with identity "-"` 后紧跟错误，没有其他上下文。
- 相同配置的 CI 构建有时通过有时失败。
- 本地构建成功但 CI 构建失败。

**Phase to address:**
Phase 1（GitHub Actions 基础配置）— 第一次配置 CI 签名时就需要考虑此 bug，预留重试预算。

---

### Pitfall 2: Ad-hoc 签名无法绕过 Gatekeeper，用户收到"已损坏"警告

**What goes wrong:**
用户下载 DMG 后双击打开，macOS 弹出 "CLIManager 已损坏，无法打开。你应该将它移到废纸篓。" 或 "无法打开 CLIManager，因为 Apple 无法检查其是否包含恶意软件。" 用户无法直接双击打开应用。即使用户尝试右键选择"打开"，部分 macOS 版本（Sequoia/Tahoe）已经移除了这个绕过选项，用户完全卡住。

**Why it happens:**
Ad-hoc 签名（`-`）仅解决了 Apple Silicon 上"必须有签名才能运行"的硬性要求，但**不会移除 Gatekeeper 的网络下载检查**。从互联网下载的文件会携带 `com.apple.quarantine` 扩展属性，Gatekeeper 检测到 quarantine xattr 后会验证签名链是否可信。Ad-hoc 签名不在 Apple 的可信开发者 ID 链中，Gatekeeper 拒绝运行并展示"已损坏"错误（实际是"未认证"的误导性文案）。

没有 $99/年的 Apple Developer Program 账号，就无法：
1. 申请 Developer ID Application 证书
2. 对 app 进行 Apple 公证（Notarization）
3. 在 Gatekeeper 眼中让 app 成为"可信来源"

**How to avoid:**
接受这个限制，并为用户提供明确的安装指引。在发布说明和 README 中提供以下步骤：

```bash
# 方法 1：用 xattr 移除 quarantine 标记（推荐给技术用户）
xattr -r -d com.apple.quarantine /Applications/CLIManager.app

# 方法 2：系统设置 > 隐私与安全性 > 安装来源 > 仍要打开
```

同时：
- 在 GitHub Release 的 release notes 中**第一行**就说明此问题和解决方法。
- 考虑提供一个 `.command` 脚本帮助用户自动清除 quarantine 属性。
- 不要让用户靠猜来解决，这是导致用户放弃使用的首要原因。

**Warning signs:**
- 用户报告"下载后无法打开"。
- 在全新 macOS 机器上测试时出现"已损坏"对话框。
- 忘记在 release notes 中添加安装说明。

**Phase to address:**
Phase 1（基础 CI 配置）+ Phase 3（发版流程）— Phase 1 明确 ad-hoc 签名的边界，Phase 3 确保 release notes 包含安装指引。

---

### Pitfall 3: TAURI_SIGNING_PRIVATE_KEY 私钥丢失导致无法推送更新

**What goes wrong:**
在 CI 中生成 `TAURI_SIGNING_PRIVATE_KEY` 后，开发者将私钥存放在 GitHub Secret 中，但没有在其他安全位置备份。后来：重新生成 GitHub Secret（因为旧 secret 被删除）、项目迁移到新仓库、或忘记了私钥内容。此时新构建出的 `latest.json` 的 signature 与旧版本用的不是同一个密钥对，**已安装旧版本的用户将永久无法自动更新**（因为旧 app 内嵌的 `pubkey` 与新 signature 不匹配，签名验证失败）。

**Why it happens:**
`tauri-plugin-updater` 使用 Ed25519 密钥对进行更新包签名验证。公钥（pubkey）在构建时编译进 `tauri.conf.json` 并嵌入到可执行文件中，私钥用于签名每次发布的 `.app.tar.gz`。一旦密钥对更换，旧 app 内嵌的公钥就无法验证新私钥签出的签名，更新检查会报 `signature verification failed`，导致 app 卡在旧版本无法更新。

**How to avoid:**
- **生成密钥时立即在两处存储私钥**：
  1. GitHub Repository Secret (`TAURI_SIGNING_PRIVATE_KEY`)
  2. 项目密码管理器（1Password、Bitwarden 等）或加密文档
- 将公钥（`.key.pub` 文件内容）commit 到仓库的 `docs/` 或 `.tauri/` 目录（公钥可以公开）。
- 在 CI 中使用私钥字符串而不是文件路径（`TAURI_SIGNING_PRIVATE_KEY` 可以是密钥内容本身）。
- 私钥生成命令：`pnpm tauri signer generate -w ~/.tauri/climanager.key`，然后立即备份。

**Warning signs:**
- 无法找到生成私钥时使用的 `.key` 文件。
- 新构建出的 `.app.tar.gz.sig` 与旧版本不同但找不到原因。
- 用户报告自动更新失败但下载链接有效。

**Phase to address:**
Phase 1（基础 CI 配置）— 第一次运行 CI 前就必须生成并备份密钥对。

---

### Pitfall 4: TAURI_SIGNING_PRIVATE_KEY 密码从环境变量读取失败

**What goes wrong:**
使用 `tauri signer generate` 生成密钥时设置了密码，并将密码存入 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 环境变量。CI 构建时签名步骤失败，报错 `failed to decode secret key: incorrect updater private key password: Wrong password for that key`，但同样的密码通过命令行参数传入时却能工作。

**Why it happens:**
这是 Tauri 已知 bug（[Issue #13485](https://github.com/tauri-apps/tauri/issues/13485)）。从环境变量读取密码时存在解码问题（疑似有额外的转义或换行符处理差异），而从命令行参数读取时正常工作。

**How to avoid:**
- **生成密钥时不设置密码**（`tauri signer generate` 时直接按回车跳过密码）。私钥安全通过 GitHub Secret 的访问控制来保证，额外密码带来的复杂度高于安全收益。
- 如果必须用密码，使用命令行参数传入而非环境变量：`tauri build -- --sign-key-password "${{ secrets.KEY_PASS }}"` 而非设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。
- 验证方法：本地先用 `TAURI_SIGNING_PRIVATE_KEY=<content> tauri build` 测试，确认能正常签名再配置 CI。

**Warning signs:**
- CI 签名步骤报 "Wrong password for that key"。
- 相同的密码在本地命令行正常但 CI 失败。
- 调试时删除密码后签名成功。

**Phase to address:**
Phase 1（基础 CI 配置）— 密钥生成策略在配置 CI 时就要确定。

---

### Pitfall 5: 版本号三文件不同步导致构建失败或版本混乱

**What goes wrong:**
**场景 A（构建失败）**：在 `tauri.conf.json` 的 `version` 字段注入了 `"2.1"` 而非合法 semver `"2.1.0"`。Tauri 的版本解析器报 `"Failed to parse version '2.1'"` 并中断构建。

**场景 B（版本显示混乱）**：用 `jq` 更新了 `tauri.conf.json` 的版本字段，但 `Cargo.toml` 的 `version` 字段仍是旧版本。`tauri-plugin-updater` 调用 `app.package_info().version` 时读取的是 Tauri config 中的版本，而 `cargo metadata` 读到的是 Cargo.toml 的版本，两处不一致。这会导致 `latest.json` 中记录的版本号与实际二进制文件内报告的版本不同，自动更新逻辑（比较当前版本与 latest.json 版本）可能误判。

**Why it happens:**
Tauri 2 中版本号需要在三个文件中维护：`tauri.conf.json`（顶层 `version` 字段）、`src-tauri/Cargo.toml`（`[package] version`）、和可选的前端 `package.json`。官方建议只维护 `tauri.conf.json`（Cargo.toml 作为 fallback），但 CI 版本注入脚本通常只更新其中一个。

**How to avoid:**
- **单一数据源策略**：在 CI 中**只更新 `tauri.conf.json`**，并在 `tauri.conf.json` 中明确写入版本，让 `Cargo.toml` 的版本通过 fallback 机制自动使用（即删除 `tauri.conf.json` 中的版本让它 fallback，还是在 CI 中同步更新两个文件 — 选一种并坚持）。

  推荐的 CI 版本注入方式（更新两个文件确保一致）：
  ```yaml
  - name: 注入版本号
    run: |
      VERSION="${GITHUB_REF#refs/tags/v}"
      # 验证 semver 格式
      echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || { echo "Tag 必须是 vX.Y.Z 格式"; exit 1; }
      # 更新 tauri.conf.json
      jq --arg v "$VERSION" '.version = $v' src-tauri/tauri.conf.json > /tmp/tauri.conf.json && mv /tmp/tauri.conf.json src-tauri/tauri.conf.json
      # 更新 Cargo.toml
      sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml
  ```
- 在 CI 注入版本后，立即 `cat src-tauri/tauri.conf.json | jq .version` 和 `grep '^version' src-tauri/Cargo.toml` 打印确认。
- Git tag 格式必须是 `vX.Y.Z`（例如 `v2.1.0`），workflow 触发规则写 `tags: ['v[0-9]+.[0-9]+.[0-9]+'` 进行格式验证。

**Warning signs:**
- CI 报 `"Failed to parse version"` 错误。
- `tauri-plugin-updater` 自动更新逻辑永远触发（认为有更新）或永远不触发（认为已是最新）。
- Release 页面显示的版本与 app 内 "关于" 界面版本不符。

**Phase to address:**
Phase 1（GitHub Actions 基础配置）— 版本注入逻辑是 CI 的核心，第一次写 workflow 时就要考虑。

---

### Pitfall 6: latest.json 格式错误导致整个更新检查静默失败

**What goes wrong:**
自动更新完全不工作 — app 每次检查都认为是最新版本，或每次都报"无法获取更新信息"，但 GitHub Release 上的文件明明存在。调试困难，因为错误信息不明确。

**Why it happens:**
`tauri-plugin-updater` 在检查版本之前会**验证整个 `latest.json` 文件结构**。只要文件中有任何字段格式错误（哪怕是一个平台的条目出问题），整个更新检查就失败，而不只是跳过出问题的平台。常见的格式错误：

1. **`pubkey` 字段填的是文件路径而非文件内容**：`"pubkey": "/Users/xxx/.tauri/app.key.pub"` — 错误。必须填 `.key.pub` 文件的实际文本内容。
2. **`signature` 字段填的是 `.sig` 文件路径而非文件内容**：必须是签名字符串本身，不是路径。
3. **`pub_date` 格式不是 ISO 8601**：必须是 `"2026-03-14T10:00:00Z"` 格式。
4. **平台 key 格式错误**：正确格式是 `"darwin-aarch64"`、`"darwin-x86_64"`，不是 `"macos-arm64"` 或 `"mac-m1"`。
5. **`version` 字段含 `v` 前缀**：`"v2.1.0"` 在某些版本的插件中会导致比较失败，应使用 `"2.1.0"`。

**How to avoid:**
- **优先使用 `tauri-apps/tauri-action` 自动生成 `latest.json`**。该 Action 知道正确格式，避免手动构造文件。
- 在 `tauri.conf.json` 中将 endpoint 指向 GitHub Release 的 `latest.json`：
  ```json
  "plugins": {
    "updater": {
      "endpoints": ["https://github.com/USERNAME/REPO/releases/latest/download/latest.json"],
      "pubkey": "（.key.pub 文件的完整文本内容）"
    }
  }
  ```
  注意 `pubkey` 值是文件**内容**，不是路径。
- 手动验证已发布的 `latest.json` 结构：`curl -L <url> | jq .` 确认能正确解析。
- 在 CI 中添加 `tauri-plugin-updater` 的 Rust 测试用例，用 mock server 验证更新检查逻辑（非必须但有价值）。

**Warning signs:**
- App 内更新检查无错误但永远显示"已是最新"。
- CI 发布后 `latest.json` 文件内容不符合预期格式。
- 报错 `"Could not fetch a valid release JSON from the remote"`（[Issue #2610](https://github.com/tauri-apps/plugins-workspace/issues/2610)）。

**Phase to address:**
Phase 2（Tauri updater 集成）— 首次集成时验证 `latest.json` 结构，不要等到发布后才测试。

---

### Pitfall 7: Ad-hoc 签名 + updater 组合 — 更新包被 Gatekeeper 二次拦截

**What goes wrong:**
自动更新下载成功（Tauri signature 验证通过），但更新安装后启动新版本时，macOS Gatekeeper 再次弹出警告或拒绝运行，导致更新无法完成。用户反映"更新后 app 打不开"。

**Why it happens:**
Tauri updater 的签名验证（Ed25519）和 macOS 的 Gatekeeper 代码签名验证是**两个独立的机制**。Tauri 的签名只保证"更新包来自可信发布者"（防篡改），而 Gatekeeper 检查的是"Apple 是否认可这个 app 开发者"（代码签名/公证）。

更新流程中：
1. `tauri-plugin-updater` 下载 `.app.tar.gz`
2. 用 `tar` 解压替换现有 `.app` bundle
3. App 重启

问题在于 `tar` 解压的文件**不会自动继承 `com.apple.quarantine` xattr**，所以解压后的新版本可能绕过也可能不绕过 Gatekeeper（取决于系统版本和安全策略）。另一个问题是：如果更新包本身在下载过程中被 macOS 标记了 quarantine，解压后的 app 就会触发 Gatekeeper 检查，而 ad-hoc 签名无法通过 Gatekeeper 信任验证。

**How to avoid:**
- 测试完整的更新流程（不仅测试"检测到更新"，还要测试"安装并重启后新版能打开"）。
- 如果发现更新后无法启动，需要在 updater 完成安装后自动运行：
  ```rust
  // 安装完成后清除 quarantine 标记
  std::process::Command::new("xattr")
      .args(["-r", "-d", "com.apple.quarantine", &new_app_path])
      .status()
      .ok();
  ```
  注意这个 workaround 需要谨慎，完整方案是获取 Apple Developer ID（消除根本原因）。
- 在 release notes 中为用户提供备用的手动更新方式（下载新 DMG + 清除 quarantine）。

**Warning signs:**
- 测试更新流程时，更新安装成功但重启后新版本无法打开。
- 用户报告"更新后需要重新运行 xattr 命令"。

**Phase to address:**
Phase 2（Tauri updater 集成）— 必须端到端测试更新安装流程，不仅测试下载。

---

### Pitfall 8: GitHub Actions 工作流缺少 `contents: write` 权限导致 Release 创建失败

**What goes wrong:**
`tauri-apps/tauri-action` 在创建 GitHub Release 或上传 artifacts 时失败，报错 `Resource not accessible by integration`（HTTP 403/422）。CI 构建步骤成功但 Release 创建失败，artifacts 未上传。

**Why it happens:**
GitHub 在 2023 年后对新建仓库将 `GITHUB_TOKEN` 的默认权限改为只读（read-only）。`tauri-action` 需要 `contents: write` 权限才能创建 Release 和上传文件，但默认 token 没有这个权限。

**How to avoid:**
在 workflow YAML 的 job 级别显式声明权限（仅 job 级别有效，workflow 级别可能不够）：
```yaml
jobs:
  publish-tauri:
    permissions:
      contents: write  # 必须，用于创建 Release 和上传 artifacts
    runs-on: macos-latest
    steps:
      ...
```
同时在仓库设置中确认：Settings → Actions → General → Workflow permissions → "Read and write permissions"（全局 fallback 设置）。

**Warning signs:**
- CI 日志出现 `Resource not accessible by integration` 或 `HTTP 403`。
- Tauri 构建成功但 GitHub Release 未被创建。
- Release 创建成功但 artifacts（DMG、tar.gz）未上传。

**Phase to address:**
Phase 1（GitHub Actions 基础配置）— 第一次配置 workflow 时就加上，不要等到报错后才加。

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| 只构建 `aarch64-apple-darwin`，不构建 `x86_64-apple-darwin` | CI 时间减半，费用减半 | Intel Mac 用户无法使用，安装时报 "wrong architecture" | 确认目标用户全部是 Apple Silicon 时可接受。否则**不可接受** |
| 不备份 TAURI_SIGNING_PRIVATE_KEY | 省去密码管理器操作 | 私钥丢失后已安装用户永久无法自动更新 | **绝不可接受** |
| Cargo.lock 不 commit 到仓库 | 减少 PR diff 噪音 | CI 每次构建拉取不同版本依赖，导致构建不可复现 | 桌面应用应 commit Cargo.lock（库不 commit，应用 commit）|
| `tauri.conf.json` 版本不注入，保持 `0.1.0` | 省去版本注入步骤 | 自动更新永远不触发（`latest.json` 里版本比已安装版本低） | **绝不可接受** |
| 使用 GitHub Release 的 HTML URL 作为 updater endpoint | 不用记 raw URL 格式 | `tauri-plugin-updater` 无法解析 HTML 页面，更新检查静默失败 | **绝不可接受**，必须使用 raw download URL |
| 不为 macOS 两个架构分别测试 | 节省测试时间 | ARM 版 DMG 在 Intel Mac 上启动失败（或反之），无法发现架构问题 | 首次发布后抽样验证两个架构即可 |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `tauri-action` | 不设 `tagName` 的 `__VERSION__` 占位符，导致 Release tag 固定为字符串 | 使用 `tagName: "app-v__VERSION__"`，让 tauri-action 替换为实际版本 |
| GitHub Release URL as updater endpoint | 指向 `github.com/user/repo/releases/latest`（HTML 页面） | 指向 `github.com/user/repo/releases/latest/download/latest.json`（raw 文件）|
| `pubkey` in `tauri.conf.json` | 填写 `.key.pub` 文件路径 | 必须填写 `.key.pub` 文件的**文本内容**（不是路径）|
| `TAURI_SIGNING_PRIVATE_KEY` | 放入 `.env` 文件 | `.env` 文件对 Tauri CLI 无效；必须是 shell 环境变量或 GitHub Secret 映射到 `env:` |
| `bundle.createUpdaterArtifacts` | 不设置（默认不生成 updater artifacts）| 必须设为 `true`（新应用）。注意 `"v1Compatible"` 在 macOS 上有 bug，不要用 |
| macOS runner 架构 | 只用 `macos-latest` 而不指定 `--target` | 必须在 matrix 中分别设置 `--target aarch64-apple-darwin` 和 `--target x86_64-apple-darwin` |
| GitHub Token 权限 | 依赖仓库全局设置，不在 workflow 中声明 | 在每个需要写权限的 job 中显式声明 `permissions: contents: write` |
| `pub_date` in `latest.json` | 使用非 ISO 8601 格式或完全省略 | 如果包含 `pub_date`，必须是 ISO 8601 格式（`2026-03-14T10:00:00Z`）；省略更安全 |
| `latest.json` 平台 key | 使用 `"macos-aarch64"` 或 `"mac-arm64"` | Tauri 只识别 `"darwin-aarch64"` 和 `"darwin-x86_64"`（以 darwin 开头）|

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| 不使用 Rust 缓存 | 每次 CI 全量编译，耗时 15-25 分钟，macOS runner 费用快速累积 | 使用 `swatinem/rust-cache@v2` 缓存 `target/` 目录 | 每次 CI 运行（无缓存时费用约 $0.08/min × 20min = $1.6/次）|
| 为 macOS 构建所有 bundle targets | `targets: "all"` 生成 DMG + .app + .pkg + updater artifacts，时间增加 30% | 指定只构建 `["dmg"]`，`createUpdaterArtifacts: true` 单独控制 updater 产物 | 生产构建中（开发构建影响不大）|
| macOS 10x 分钟计费倍率 | 少量 CI 运行快速消耗月度免费配额 | 只在 tag push 时触发 macOS 构建；PR 验证只用 Linux runner 跑 lint/test | 私有仓库 + 频繁 CI 触发时 |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| 将 TAURI_SIGNING_PRIVATE_KEY 明文写入 workflow YAML | 私钥暴露在 git 历史中，任何有仓库访问权的人都能看到 | 始终通过 GitHub Secrets 注入，在 `env:` 中使用 `${{ secrets.KEY }}` |
| 公开仓库中打印 secrets 到 CI 日志 | 私钥内容出现在公开 CI 日志中 | 不要 `echo $TAURI_SIGNING_PRIVATE_KEY`；使用 `echo "Key length: ${#TAURI_SIGNING_PRIVATE_KEY}"` 验证 |
| 不验证 tag 格式就注入版本 | 非 semver 的 tag（如 `v2-beta`）注入后导致构建失败或版本号格式混乱 | workflow 触发时校验 tag 格式 `v[0-9]+.[0-9]+.[0-9]+`，不符合格式则 fail fast |
| DMG 不做 ad-hoc 签名（完全不签名）| 在 Apple Silicon 上完全无法运行（"已损坏，无法打开" 且没有任何绕过方法）| 至少配置 ad-hoc 签名 `signingIdentity: "-"` |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Release notes 不包含 macOS 安装说明 | 用户下载后无法打开，以为是 bug 或应用本身有问题 | Release notes 第一行就写安装注意事项和 `xattr` 命令 |
| 自动更新失败时无用户提示 | 用户不知道更新失败，也不知道如何手动更新 | 更新失败时展示 toast 通知，提供"下载新版本"的链接 |
| 更新下载中没有进度展示 | 大文件（DMG 约 10-30MB）下载时 UI 无反馈，用户不知道是否在工作 | 使用 `tauri-plugin-updater` 的进度回调更新 UI |
| 版本号在 "关于" 界面显示 `0.1.0` | 用户无法判断是否需要更新 | 确保 CI 版本注入后 app 内版本号与 Release tag 一致 |

---

## "Looks Done But Isn't" Checklist

- [ ] **版本注入：** 发布后在 app 内 "关于" 界面确认版本号是否与 git tag 一致（而不是 `0.1.0`）
- [ ] **两个架构都测试：** 在 Intel Mac 上验证 `x86_64` DMG 能打开；在 Apple Silicon Mac 上验证 `aarch64` DMG 能打开
- [ ] **更新检查 end-to-end：** 安装 v2.1.0，发布 v2.1.1，确认 app 内收到更新提示
- [ ] **更新安装完成：** 不只是检测到更新，还要确认点击"安装"后新版本能正常启动
- [ ] **ad-hoc 签名验证：** 在新机器（或全新用户账号）上下载 DMG，按照 release notes 的 xattr 步骤确认能打开
- [ ] **latest.json 格式：** `curl -L <release_latest_json_url> | jq .` 确认 JSON 结构正确，`version`、`platforms`、`signature` 字段都存在
- [ ] **私钥已备份：** 确认 `TAURI_SIGNING_PRIVATE_KEY` 在 GitHub Secret 以外的位置有备份
- [ ] **Cargo.lock 已 commit：** `git status` 确认 `Cargo.lock` 在仓库中
- [ ] **macOS runner 费用：** 确认 CI 只在 tag push 时触发 macOS 构建，不在每个 PR 上触发

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 私钥丢失 | HIGH | 重新生成密钥对 → 更新 `tauri.conf.json` 中的 `pubkey` → 构建新版本并发布 → 用户必须**手动**下载安装新版（旧版 app 内嵌旧 pubkey，无法自动更新到新密钥签名的版本）|
| `latest.json` 格式错误 | LOW | 修复 `latest.json` 内容（或修复 CI 生成逻辑）→ 重新上传到 GitHub Release → 用户下次更新检查自动恢复 |
| 版本注入错误（版本号对不上）| MEDIUM | 删除错误的 Release tag → 修复 CI 脚本 → 重新 tag 触发构建。注意：如果 `latest.json` 已被下载缓存，需要等 CDN 缓存过期 |
| Ad-hoc 签名 CI 随机失败 | LOW | 重跑失败的 CI job。如果连续失败，检查 Tauri 版本是否有相关 bugfix，或临时改用不同的 runner 版本（`macos-13` 替代 `macos-latest`）|
| GitHub Token 权限不足 | LOW | 在 workflow YAML 添加 `permissions: contents: write`，重新推 tag 触发构建 |
| 更新包被 Gatekeeper 拦截 | MEDIUM | 在 updater 安装后添加 `xattr` 清除步骤（临时 workaround）。根本解决方案是获取 Apple Developer ID |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Ad-hoc 签名 CI 随机失败 | Phase 1: GitHub Actions 基础配置 | CI 连续 3 次构建成功，没有签名失败 |
| 用户无法打开 App（Gatekeeper）| Phase 1 + Phase 3 发版 | 新机器按 release notes 安装步骤确认能打开 |
| TAURI_SIGNING_PRIVATE_KEY 丢失 | Phase 1: GitHub Actions 基础配置 | 确认私钥在密码管理器中有备份，能在本地用私钥手动签名 |
| TAURI_SIGNING_PRIVATE_KEY 密码失败 | Phase 1: GitHub Actions 基础配置 | 无密码生成私钥，CI 签名步骤成功 |
| 版本号三文件不同步 | Phase 1: GitHub Actions 基础配置 | 发布后 app 内版本号与 tag 一致 |
| `latest.json` 格式错误 | Phase 2: Tauri updater 集成 | `curl latest.json \| jq .` 结构正确；更新检查返回预期结果 |
| 更新包被 Gatekeeper 二次拦截 | Phase 2: Tauri updater 集成 | 端到端更新流程：安装旧版 → 触发更新 → 安装成功 → 新版能打开 |
| GitHub Token 权限不足 | Phase 1: GitHub Actions 基础配置 | CI 成功创建 Release 并上传所有 artifacts |

---

## Sources

- [Tauri macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/) — ad-hoc 签名配置方法，Gatekeeper 限制（HIGH confidence）
- [Tauri Plugin Updater](https://v2.tauri.app/plugin/updater/) — `latest.json` 格式、`pubkey` 配置、`createUpdaterArtifacts`（HIGH confidence）
- [Tauri GitHub CI Pipeline](https://v2.tauri.app/distribute/pipelines/github/) — workflow 权限、secrets、matrix 配置（HIGH confidence）
- [Tauri Issue #13804: Ad-hoc signing fails randomly in CI](https://github.com/tauri-apps/tauri/issues/13804) — 已知随机失败 bug（HIGH confidence）
- [Tauri Issue #13485: Updater signing with ENV password broken](https://github.com/tauri-apps/tauri/issues/13485) — 密码从 ENV 读取失败 bug（HIGH confidence）
- [Tauri Issue #10217: v1Compatible not creating macOS artifacts](https://github.com/tauri-apps/tauri/issues/10217) — `v1Compatible` 在 macOS 上的 bug（HIGH confidence）
- [Tauri plugins-workspace Issue #2610: Static JSON fetch error](https://github.com/tauri-apps/plugins-workspace/issues/2610) — `latest.json` 获取失败（HIGH confidence）
- [Tauri Discussion #8265: Sync version across three files](https://github.com/tauri-apps/tauri/issues/8265) — 版本同步社区讨论（HIGH confidence）
- [GitHub Actions runner pricing docs](https://docs.github.com/en/billing/reference/actions-runner-pricing) — macOS runner 10x 计费倍率（HIGH confidence）
- [GitHub changelog: 2025-12-16 pricing changes](https://github.blog/changelog/2025-12-16-coming-soon-simpler-pricing-and-a-better-experience-for-github-actions/) — 2026 年计费变更（HIGH confidence）
- [Tauri Discussion #7703: Notarization effect on updater signature](https://github.com/orgs/tauri-apps/discussions/7703) — 公证不影响 updater 签名（MEDIUM confidence）
- [macOS Gatekeeper bypass via tar extraction](https://unit42.paloaltonetworks.com/gatekeeper-bypass-macos/) — tar 解压不传播 quarantine xattr（MEDIUM confidence）
- [Dev.to: Ship Tauri v2 app – Code Signing](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-code-signing-for-macos-and-windows-part-12-3o9n) — 实践经验，entitlements 配置（MEDIUM confidence）
- [Dev.to: Ship Tauri v2 app – GitHub Actions](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) — matrix strategy，架构矩阵（MEDIUM confidence）

---
*Pitfalls research for: CLIManager v2.1 Release Engineering（CI/CD + Ad-hoc 签名 + 自动更新）*
*Researched: 2026-03-14*
