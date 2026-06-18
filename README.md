# Runestone

**跨项目的个人 AI 记忆系统。** Claude Code 的记忆是项目级的——项目 A 的约定和偏好，到了项目 B 就得重新教。Runestone 把记忆提升到**人**的层面，所有项目共享。

基于 Rust + Git，数据完全由你控制。可通过 Claude Code `/rune` skill 使用。

## 安装

```bash
# 在当前项目的 Claude Code 中安装
bash install-claude-code.sh

# 或者指定项目目录
bash install-claude-code.sh /path/to/your-project
```

安装脚本会：
1. 下载 `runestone` 二进制到 `~/.local/bin/`
2. 把 `/rune` skill 复制到项目的 `.claude/skills/rune/`

## 配置

在项目的 `.envrc` 中添加：

```bash
export RUNESTONE_OWNER="$(whoami)"
export RUNESTONE_REMOTE="git@github.com:你的用户名/记忆仓库.git"
export OPENAI_API_KEY="sk-..."
```

记忆仓库需要提前在 GitHub 创建（建议设为私有）。首次使用前再执行一次初始同步：

```bash
runestone --owner "$RUNESTONE_OWNER" git sync --remote "$RUNESTONE_REMOTE"
```

## 使用 /rune

在 Claude Code 中：

### 记住经验

当你发现一个值得跨项目保留的经验或偏好时：

```
/rune remember "我对 Git 分支命名偏好 kebab-case，用 conventional commits 格式"
```

Claude 会：
1. 理解你的意思，分类为 `preference`
2. 生成标题：`git 分支命名和提交规范`
3. 展开为结构化内容并写入记忆
4. 存储到 `./data/yubiao/memory/preferences/git 分支命名和提交规范.md`

### 在新项目中回忆

打开新项目，直接问：

```
/rune recall "git 分支命名"
```

Claude 从你的个人记忆中搜索，返回匹配结果，并**主动问你是否要应用到当前项目**（写入 CLAUDE.md、生成项目配置等）。

### 查看和整理

```
/rune list          # 列出所有记忆
/rune clean         # 扫描重复，交互式整理
/rune sync          # 推送到 GitHub
```

## 更多例子

```
# 记录技术踩坑
/rune remember "libgit2 的 credential 回调在 SSH 失败时死循环——因为 Cred::ssh_key 返回 Ok 不代表 key 可用，需要轮换凭据源"

# 记录设计决策
/rune remember "选了 git2 而不是 gitoxide——git2 更成熟稳定，代码量少，适合当前阶段"

# 记录个人偏好
/rune remember "跟我解释用中文，代码注释用英文，回复简洁不要 emoji"

# 在新项目查
/rune recall "Rust 错误处理"
/rune recall "SSH 认证"
/rune recall "Git 工作流"
```

## 记忆类型

| 类型 | 说明 | 例子 |
|------|------|------|
| `preference` | 习惯、风格、工作方式 | kebab-case 分支、conventional commits |
| `case` | 踩坑经验、设计决策、bug 模式 | git2 SSH 死循环根因 |
| `entity` | 概念、模式、知识点 | credential provider 模式 |
| `profile` | 个人身份和背景 | 后端开发、Rust 主语言 |

## 命令参考

```
runestone --owner <name> memory store   --kind <k> --key <k> --content "..."
runestone --owner <name> memory load    --kind <k> --key <k>
runestone --owner <name> memory list
runestone --owner <name> memory search  --query "..."
runestone --owner <name> memory clean   --dry-run
runestone --owner <name> memory clean

runestone --owner <name> git init   --remote <url>
runestone --owner <name> git sync   --remote <url>
```

## 开发

```
cargo build
cargo test
cargo clippy -- -D warnings
```

详见 [CLAUDE.md](CLAUDE.md)。

## License

MIT
