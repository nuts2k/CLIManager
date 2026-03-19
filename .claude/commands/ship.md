# /ship — 一键发版指令

用于发布新版本：版本 bump → CHANGELOG 生成 → git commit → tag → push。

## 使用方式

```
/ship [patch|minor|major]
```

默认 bump 类型为 `patch`。

---

## 执行步骤

使用 Bash 工具按以下步骤执行。每一步完成后输出带 ✅ 的状态。

### 第 0 步：检查工作区状态

```bash
git status --short
```

如果工作区有未提交的更改（不含 `src-tauri/Cargo.toml` 和 `CHANGELOG.md`），**停止执行**，提示用户：
> 工作区有未提交的更改，请先 `git stash` 或提交后再运行 /ship。

### 第 1 步：解析参数

从 `$ARGUMENTS` 获取 bump 类型：
- 有效值：`patch`、`minor`、`major`
- 未指定或空：默认 `patch`
- 无效值：停止执行并提示

### 第 2 步：读取当前版本

```bash
grep '^version = ' src-tauri/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'
```

解析出 `MAJOR.MINOR.PATCH` 三段版本号。

### 第 3 步：计算新版本号

根据 bump 类型：
- `patch`: PATCH + 1
- `minor`: MINOR + 1，PATCH = 0
- `major`: MAJOR + 1，MINOR = 0，PATCH = 0

例：`0.2.0` + `minor` = `0.3.0`

检查 tag 是否已存在：
```bash
git tag -l "v{NEW_VERSION}"
```
如果已存在，**停止执行**并报错：
> Tag v{NEW_VERSION} 已存在，请检查版本号是否正确。

### 第 4 步：bump Cargo.toml 版本号

使用 `sed` 替换 `src-tauri/Cargo.toml` 中的版本字段：

```bash
sed -i '' 's/^version = "'"${OLD_VERSION}"'"/version = "'"${NEW_VERSION}"'"/' src-tauri/Cargo.toml
```

仅修改 `src-tauri/Cargo.toml`，不修改 `tauri.conf.json`（项目决策：版本来源唯一为 Cargo.toml）。

输出：`✅ 版本 bump: {OLD_VERSION} → {NEW_VERSION}`

### 第 5 步：生成 CHANGELOG 条目

**获取提交范围：**

```bash
# 尝试获取上一个 tag
PREV_TAG=$(git describe --tags --abbrev=0 HEAD 2>/dev/null || echo "")
if [ -z "$PREV_TAG" ]; then
  # 无 tag，获取全部提交
  COMMITS=$(git log --pretty=format:"%s" HEAD)
else
  COMMITS=$(git log --pretty=format:"%s" "${PREV_TAG}..HEAD")
fi
```

**按 Conventional Commits 分类（中文标题）：**

从提交信息中提取并分类：
- `feat:` 或 `feat(...):`  → **新功能**
- `fix:` 或 `fix(...):`   → **修复**
- `refactor:` 或 `refactor(...):`  → **重构**
- `docs:` 或 `docs(...):`  → **文档**
- `chore:` 或 `chore(...):`  → **其他**
- 其他不符合规范的提交   → **其他**

提交信息去掉前缀（保留括号内的 scope，或直接保留描述部分）：
- `feat(ui): 添加深色模式` → 新功能：`添加深色模式`
- `fix: 修复崩溃问题` → 修复：`修复崩溃问题`

**生成条目文本（仅包含非空分类）：**

```
## v{NEW_VERSION} ({YYYY-MM-DD})

### 新功能
- 描述1
- 描述2

### 修复
- 描述1

### 重构
- 描述1

### 文档
- 描述1

### 其他
- 描述1
```

日期使用当前日期：
```bash
date +"%Y-%m-%d"
```

**更新 CHANGELOG.md：**

1. 如果 `CHANGELOG.md` 不存在，创建初始文件（含标题和说明）
2. 将新条目插入到 `---` 分隔线之后（现有条目之前）

使用 Python 进行文本操作：
```bash
NEW_ENTRY="$NEW_ENTRY" python3 - <<'PY'
from pathlib import Path
import os

path = Path('CHANGELOG.md')
new_entry = os.environ['NEW_ENTRY']

if path.exists():
    changelog = path.read_text()
else:
    changelog = '# 更新日志\n\n所有重要变更都会记录在这个文件中。\n\n格式基于 [Conventional Commits](https://www.conventionalcommits.org/)。\n\n---\n'

if '---' in changelog:
    parts = changelog.split('---', 1)
    changelog = parts[0] + '---\n\n' + new_entry + '\n\n' + parts[1].lstrip()
else:
    changelog = changelog.rstrip() + '\n\n' + new_entry + '\n'

path.write_text(changelog)
PY

grep -q "## v${NEW_VERSION}" CHANGELOG.md
```

输出：`✅ CHANGELOG.md 已更新`

### 第 6 步：Git 操作

```bash
git add src-tauri/Cargo.toml CHANGELOG.md
git commit -m "chore(release): v{NEW_VERSION}"
git tag "v{NEW_VERSION}"
git push && git push --tags
```

- 提交完成后输出：`✅ 提交: chore(release): v{NEW_VERSION}`
- tag 创建后输出：`✅ Tag: v{NEW_VERSION}`
- push 完成后输出：`✅ 已推送到远程仓库`

**push 失败处理：**

如果 `git push` 失败，输出：
> push 失败，本地 commit 和 tag 已创建。可手动处理：
> - 检查远程连接：`git remote -v`
> - 手动推送：`git push && git push --tags`
> - 如需撤销本地 tag：`git tag -d v{NEW_VERSION}`
> - 如需撤销本地 commit：`git reset HEAD~1`

### 第 7 步：完成提示

```
🚀 发版完成！CI 将自动构建并发布到 GitHub Releases。
   版本：v{NEW_VERSION}
   查看构建进度：https://github.com/{REPO}/actions
```

Repository 路径从 `git remote get-url origin` 解析。

解析示例：
```bash
REMOTE_URL=$(git remote get-url origin)
REPO=$(REMOTE_URL="$REMOTE_URL" python3 - <<'PY'
import os, re
url = os.environ['REMOTE_URL']
for pattern in [r'github\.com[:/](.+?)(?:\.git)?$', r'^https?://github\.com/(.+?)(?:\.git)?$']:
    m = re.search(pattern, url)
    if m:
        print(m.group(1))
        break
PY
)
```

---

## 完整状态输出示例

```
✅ 版本 bump: 0.2.0 → 0.3.0
✅ CHANGELOG.md 已更新
✅ 提交: chore(release): v0.3.0
✅ Tag: v0.3.0
✅ 已推送到远程仓库
🚀 发版完成！CI 将自动构建并发布到 GitHub Releases。
```
