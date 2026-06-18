#!/bin/bash
# 安装 Runestone 到当前项目的 Claude Code
# 用法: bash install-claude-code.sh
set -euo pipefail

REPO="lagudomeze/runestone"
PROJECT_DIR="${1:-$(pwd)}"
SKILL_DIR="${PROJECT_DIR}/.claude/skills/rune"

echo "=== Runestone Claude Code 安装 ==="
echo "项目目录: ${PROJECT_DIR}"

# 1. 安装二进制
which runestone >/dev/null 2>&1 || {
    echo ""
    echo "正在安装 runestone 命令行..."

    # 获取最新版本
    LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | cut -d'"' -f4)
    if [ -z "$LATEST" ]; then
        # Fallback: 从源码安装
        echo "从源码编译安装..."
        TMP=$(mktemp -d)
        git clone --depth 1 "https://github.com/${REPO}.git" "$TMP" 2>/dev/null
        (cd "$TMP" && cargo install --path crates/runestone-cli --force 2>/dev/null)
        rm -rf "$TMP"
    else
        echo "下载 ${LATEST}..."
        TMP=$(mktemp -d)
        curl -fsSL "https://github.com/${REPO}/releases/download/${LATEST}/runestone-${LATEST}.tar.gz" -o "${TMP}/runestone.tar.gz"
        tar xzf "${TMP}/runestone.tar.gz" -C "${TMP}"
        mkdir -p "${HOME}/.local/bin"
        cp "${TMP}/runestone" "${HOME}/.local/bin/runestone"
        chmod +x "${HOME}/.local/bin/runestone"
        rm -rf "${TMP}"
    fi
    echo "二进制已安装到 ${HOME}/.local/bin/runestone"
}

# 2. 安装 /rune skill 到当前项目
echo ""
echo "安装 /rune skill..."

mkdir -p "${SKILL_DIR}"

# 从当前目录（脚本所在位置）复制 skill
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -d "${SCRIPT_DIR}/.claude/skills/rune" ]; then
    cp -r "${SCRIPT_DIR}/.claude/skills/rune/"* "${SKILL_DIR}/"
else
    # 从 GitHub 下载
    curl -fsSL "https://raw.githubusercontent.com/${REPO}/main/install.sh" -o /dev/null 2>&1 || true
    mkdir -p "${SKILL_DIR}"
    # 内嵌 skill 文件
    cat > "${SKILL_DIR}/SKILL.md" << 'SKILLEOF'
# Rune — 个人记忆管理

跨项目的个人记忆入口。基于 Runestone 读写 `./data/{owner}/memory/`。

## 子命令

### remember — 捕获记忆

```
/rune remember <种子描述>
```

1. 理解用户意图：是什么（现象/偏好/概念/经验）
2. 分类为 `profile` / `preference` / `entity` / `case`
3. 生成标题（简洁、文件名风格，支持中英文）
4. 展开为 3-5 句：是什么、为什么重要、怎么用。200 字内
5. 执行：

```bash
source .envrc 2>/dev/null
runestone --owner "${RUNESTONE_OWNER:-$(whoami)}" memory store \
  --kind <kind> --key "<title>" --content "<content>"
```

| kind | 适用 |
|------|------|
| `profile` | 身份、角色、背景 |
| `preference` | 习惯、风格、工作方式 |
| `entity` | 概念、模式、知识点 |
| `case` | 经验、bug 模式、设计决策 |

### recall — 搜索 + 应用

```
/rune recall <主题>
```

1. 用 `runestone memory list` 列出记忆文件，或用关键词搜索
2. 展示匹配结果（类型 + 标题 + 摘要）
3. **主动问用户**：是否需要应用到当前项目？（写入 CLAUDE.md、生成 skill、设定规则等）
4. 如果需要，根据记忆内容和当前项目上下文生成对应文件

### list — 列出所有

```
/rune list
```

列出所有记忆文件，按类型分组显示路径和内容摘要。

### sync — 同步远程

```
/rune sync
```

```bash
source .envrc && runestone --owner "$RUNESTONE_OWNER" git sync --remote "$RUNESTONE_REMOTE"
```

### clean — 整理

```
/rune clean
```

1. `runestone --owner "$OWNER" memory clean --dry-run` 预览重复
2. 展示给用户确认
3. 确认后执行 `runestone memory clean` 合并
4. 手动检查并提出冗余/过时项供用户决定
SKILLEOF
    echo "  (从内置模板安装)"
fi

echo "  ✓ skill 已安装到 ${SKILL_DIR}"

# 3. 检查配置
echo ""
echo "=== 安装完成 ==="
echo ""
echo "接下来请配置环境变量（添加到 .envrc 或 ~/.profile）："
echo "  export RUNESTONE_OWNER=\"\$(whoami)\""
echo "  export RUNESTONE_REMOTE=\"git@github.com:你的用户名/你的记忆仓库.git\""
echo "  export OPENAI_API_KEY=\"sk-...\""
echo ""
echo "确保 PATH 包含 ${HOME}/.local/bin"
echo ""
echo "在新项目中使用："
echo "  /rune recall \"git 分支命名\""
echo "  /rune remember \"我发现 libgit2 在空仓库上 fetch 会卡死\""
