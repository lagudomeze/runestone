# Runestone 快速上手

## 准备工作

```bash
# 确保 API key 已配置
cat .envrc

# 如果没有，复制模板
cp .envrc.example .envrc
# 编辑 .envrc 填入你的 key
direnv allow
```

## 一、会话管理（不需要 LLM）

```bash
# 创建会话
runestone --owner alice session --agent mybot create --session s1

# 追加消息
runestone --owner alice session --agent mybot add \
    --session s1 --role user --content "Hi, I'm Bob. I code in Go and use vscode."

runestone --owner alice session --agent mybot add \
    --session s1 --role assistant --content "Hi Bob! Go is great."

# 提交（没有 API key 时使用 NoopExtractor，不会提取记忆）
runestone --owner alice session --agent mybot commit --session s1

# 查看历史
runestone --owner alice session --agent mybot history --session s1
```

## 二、配置 LLM 后（自动记忆提取）

```bash
# 确保 .envrc 已配置 API key，然后：
runestone --owner alice session --agent mybot commit --session s1

# 输出会显示提取的记忆：
# Commit successful: 2 messages processed, 3 changes extracted, offset now 2
#   → GlobalProfile { content: "Bob, Go developer, uses vscode" }
#   → GlobalPreference { key: "editor", value: "vscode" }
#   → GlobalPreference { key: "language", value: "Go" }
```

## 三、查看提取的记忆

```bash
# 列出所有记忆文件
runestone --owner alice memory list
# alice/memory/.abstract.md      ← L0 摘要
# alice/memory/.overview.md      ← L1 概览
# alice/memory/profile.md        ← 个人档案
# alice/memory/preferences/editor.md
# alice/memory/preferences/language.md

# 读取 profile
runestone --owner alice memory load --kind profile
# → Bob is a Go developer who uses vscode.

# 读取 preference
runestone --owner alice memory load --kind preference --key editor
# → vscode

# 读取 L0 摘要
cat ./data/alice/memory/.abstract.md
# → This directory contains a profile for Bob, a Go developer who uses VS Code.

# 读取 L1 概览
cat ./data/alice/memory/.overview.md
```

## 四、直接写入记忆（不需要会话）

```bash
# 写入偏好
runestone --owner alice memory store \
    --kind preference --key theme --content "dark mode"

# 写入实体
runestone --owner alice memory store \
    --kind entity --key redis --content "Redis cluster for caching layer"

# 写入事件/决策
runestone --owner alice memory store \
    --kind event --key chose-postgres --content "Decided to use PostgreSQL for primary DB"

# 验证
runestone --owner alice memory load --kind event --key chose-postgres
```

## 五、搜索记忆

```bash
# 关键词搜索
runestone --owner alice memory search --query "Go developer"
# [1.00] alice/memory/profile.md — bob is a go developer who uses vscode.

runestone --owner alice memory search --query "postgres caching"
# [1.00] alice/memory/events/chose-postgres.md — decided to use postgresql for primary db
# [0.50] alice/memory/entities/redis.md — redis cluster for caching layer
```

## 六、多 Agent 隔离

```bash
# Agent mybot 有自己的记忆
runestone --owner alice memory store \
    --kind case --agent mybot --key fix-timeout \
    --content "Added 30s timeout with exponential backoff for HTTP client"

# Agent otherbot 独立
runestone --owner alice session --agent otherbot create --session s2
runestone --owner alice session --agent otherbot add \
    --session s2 --role user --content "How to handle 401 errors?"

# 查看各 Agent 的记忆
runestone --owner alice memory list --agent mybot
runestone --owner alice memory list --agent otherbot
```

## 七、完整工作流演示

```bash
# 1. 开始新会话
runestone -o alice session --agent mybot create --session demo

# 2. 多轮对话
runestone -o alice session --agent mybot add --session demo \
    --role user --content "I need to deploy a Rust web service with Docker"
runestone -o alice session --agent mybot add --session demo \
    --role assistant --content "Let me help. What's your current setup?"
runestone -o alice session --agent mybot add --session demo \
    --role user --content "I use GitHub Actions for CI, and want multi-stage builds"

# 3. 提交（LLM 自动提取记忆）
runestone -o alice session --agent mybot commit --session demo

# 4. 查看提取结果
runestone -o alice memory list --agent mybot
runestone -o alice memory search --query "Docker Rust deployment"

# 5. 查看会话历史
runestone -o alice session --agent mybot history --session demo
```

## 八、检查当前状态

```bash
# 查看所有文件
runestone -o alice memory list

# 查看特定 Agent
runestone -o alice memory list --agent mybot

# 搜索
runestone -o alice memory search --query "Docker"
```

## 环境变量参考

| 变量 | 必需 | 默认值 | 说明 |
|------|:---:|------|------|
| `OPENAI_API_KEY` | ✅ | — | API 密钥 |
| `OPENAI_API_BASE` | — | `https://api.openai.com/v1` | 自定义端点 |
| `RUNESTONE_MODEL` | — | `gpt-4o-mini` | 模型名 |

**DeepSeek 示例：**
```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_API_BASE="https://api.deepseek.com/v1"
export RUNESTONE_MODEL="deepseek-chat"
```

**Ollama 示例：**
```bash
export OPENAI_API_KEY="ollama"
export OPENAI_API_BASE="http://localhost:11434/v1"
export RUNESTONE_MODEL="llama3.2"
```
