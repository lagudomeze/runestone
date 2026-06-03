# Runestone 作为 Agent 记忆/上下文层

## 概述

Runestone 可以嵌入到各类 AI Agent 中，作为**持久化记忆和上下文层**，提供跨会话的知识积累和自动注入。

核心理念：

- **不依赖外部服务**：Runestone 是本地二进制 + 文件系统，零基础设施
- **自动注入**：Agent 发起对话前自动召回相关记忆，无需手动调用工具
- **自动捕获**：Agent 完成任务后自动提取新知识，写入记忆库
- **Git 版本化**：所有记忆可追溯、可回滚

## 集成模式总览

| 模式 | 复杂度 | 效果 | 适用场景 |
|------|--------|------|----------|
| **CLI + Hooks** | ⭐ 低 | 最佳 | Claude Code、各类 CLI Agent |
| **MCP Tools** | ⭐⭐ 中 | 好 | 需要按需查询的 Agent |
| **Rust Lib** | ⭐️ 最低 | 最佳 | Rust Agent 项目 |
| **CLAUDE.md 注入** | ⭐ 极低 | 中等 | 快速上手、零配置 |

推荐主路径：**CLI + Hooks**，辅以 **CLAUDE.md 注入** 作为兜底。

---

## 一、Claude Code 集成（CLI + Hooks 模式）

### 1.1 整体架构

```
┌───────────────────────────────────────────────────┐
│                   Claude Code                      │
│                                                    │
│  SessionStart  UserPromptSubmit  Stop  SessionEnd  │
└──────┬──────────────┬────────────┬────────┬────────┘
       │              │            │        │
       ▼              ▼            ▼        ▼
  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │ 注入最近 │  │ 语义召回  │  │ 增量提取 │  │ 最终提交 │
  │ 摘要/决策│  │ 相关记忆  │  │ 新知识   │  │ + 写摘要 │
  └────┬────┘  └─────┬────┘  └────┬─────┘  └────┬─────┘
       │             │            │             │
       └─────────────┴────────────┴─────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │     Runestone       │
              │  CLI / 文件系统     │
              └─────────────────────┘
```

### 1.2 Hook 配置

在项目根目录的 `.claude/settings.local.json` 中配置：

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "matcher": "",
        "command": "~/.local/bin/runestone-hook recall --owner '{user}' --query '{prompt}'"
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "command": "~/.local/bin/runestone-hook capture --owner '{user}' --agent '{agent}'"
      }
    ],
    "SessionStart": [
      {
        "matcher": "",
        "command": "~/.local/bin/runestone-hook inject --owner '{user}' --agent '{agent}'"
      }
    ],
    "SessionEnd": [
      {
        "matcher": "",
        "command": "~/.local/bin/runestone-hook commit --owner '{user}' --agent '{agent}'"
      }
    ]
  }
}
```

### 1.3 各 Hook 职责

#### SessionStart — 冷启动注入

```bash
# 注入最近 5 次会话摘要 + 上次 SessionEnd 手记
runestone context inject --owner alice --agent mybot --limit 5
```

输出 `<runestone-context>` 块注入到系统提示中：

```markdown
<runestone-context>
## Recent sessions
- 2026-06-02: Fixed authentication bug in login flow (PR #42)
- 2026-06-02: Added rate limiting middleware for API

## Key decisions
- Use Redis for session store (not PostgreSQL)
- API versioning via URL prefix (/v1/)

## Active patterns
- Error handling: exn + thiserror pattern (see src/error.rs)
</runestone-context>
```

**原则**：注入内容控制在 ~500 tokens，只给"目录"，不给全文。Agent 需要详情时自行检索。

#### UserPromptSubmit — 语义召回

```bash
# 基于用户提示语义搜索相关记忆
runestone memory recall --owner alice --query "how to handle database migration" --limit 5
```

输出 `<runestone-recall>` 块：

```markdown
<runestone-recall>
## Related memories for "database migration"
1. [case] Diesel migration with rollback (src/db/migrations/)
2. [pattern] Run migration in transaction, verify before commit
3. [event] 2026-05-15: Production migration caused 3min downtime — root cause: missing index
</runestone-recall>
```

#### Stop — 增量提取

```bash
# 读取本轮新增消息，增量提取记忆（不阻塞）
runestone memory capture --owner alice --agent mybot --session s1 --async
```

流程：
1. 读取本轮对话 transcript（只读新增行，偏移量增量）
2. 过滤系统注入块（防止自污染）
3. LLM 提取：新决策、新偏好、新案例、新实体
4. 写入对应文件（profile.md、events/、cases/ 等）
5. 更新 .abstract.md（会话摘要）
6. Commit offset（不阻塞 Agent 响应）

**关键设计**：Stop hook 采用**异步写路径**——先 `exit 0` 放行 Agent，后台子进程完成提取和写入。用户不等待。

#### SessionEnd — 最终提交 + 手记

```bash
# 最终 commit + 生成 CLAUDE.md 摘要块
runestone memory finalize --owner alice --agent mybot --session s1
```

1. 确保所有增量已提交
2. 生成 ~300 token 会话手记
3. 追加到 `CLAUDE.md` 的 runestone 区块（下次 SessionStart 自动加载）
4. Git commit

### 1.4 自污染防护

Auto-capture 时必须过滤掉自己注入的内容：

```rust
fn strip_injected_blocks(transcript: &str) -> String {
    transcript
        .replace_all(r"<runestone-context>.*?</runestone-context>", "")
        .replace_all(r"<runestone-recall>.*?</runestone-recall>", "")
        .replace_all(r"<system-reminder>.*?</system-reminder>", "")
}
```

否则 Agent 会把"相关记忆"当做"用户要求"再次处理，形成循环污染。

### 1.5 递归防护

Hook 脚本内部如果调用了 `claude -p`，必须设置环境变量防止无限递归：

```bash
if [ -n "$RUNESTONE_HOOK_ACTIVE" ]; then
    exit 0
fi
export RUNESTONE_HOOK_ACTIVE=1
```

---

## 二、CLAUDE.md 注入（兜底方案）

即使没有配置 Hooks，Runestone 也能通过 CLAUDE.md 提供基础记忆：

```markdown
<!-- CLAUDE.md 中的 Runestone 区块 -->
<!-- RUNESTONE:START -->
## Recent activity (auto-generated)
- 2026-06-03 14:30: Added rate limiting middleware (session s3)
- 2026-06-03 10:15: Fixed auth bug in login flow (session s2)
- 2026-06-02 16:00: Initial project setup (session s1)

## Key decisions
- Use Redis for session store — see docs/decisions/001-session-store.md

## Recurring patterns
- Error handling: always use .into_exn()? for foreign errors
<!-- RUNESTONE:END -->
```

每次 SessionEnd 自动更新此区块，控制在 ~500 tokens 内。下次会话 CLAUDE.md 被自动加载，Agent 不需要任何工具调用就能获取上下文。

**优势**：零配置、零延迟、无需额外工具。**局限**：静态注入，无法语义检索。

---

## 三、通用 CLI 模式

任何能调用 shell 命令的 Agent 都可以通过 CLI 使用 Runestone：

```bash
# 创建会话
runestone session create --owner {user} --agent {agent} --session {uuid}

# 追加消息
runestone session add --owner {user} --agent {agent} --session {uuid} \
    --role user --content "..."

# 提交（提取记忆）
runestone session commit --owner {user} --agent {agent} --session {uuid}

# 召回相关记忆
runestone memory recall --owner {user} --query "..." --limit 5

# 导入资源
runestone resource add --owner {user} --uri "https://..." --wait
```

使用方式（Agent 侧）：

```python
# Python Agent 示例
import subprocess, json

def recall_memory(owner: str, query: str) -> str:
    result = subprocess.run([
        "runestone", "memory", "recall",
        "--owner", owner,
        "--query", query,
        "--limit", "5",
        "--format", "json"
    ], capture_output=True, text=True)
    memories = json.loads(result.stdout)
    return format_as_context(memories)

def capture_session(owner: str, agent: str, session: str):
    subprocess.Popen([  # 异步，不阻塞
        "runestone", "session", "commit",
        "--owner", owner,
        "--agent", agent,
        "--session", session
    ])
```

---

## 四、Rust Lib 嵌入

如果 Agent 本身是 Rust 项目，直接集成库：

```rust
use runestone::{SessionManager, MemoryRetriever, ResourceManager};

// 初始化
let mgr = SessionManager::new("./data".into());
let retriever = MemoryRetriever::new(&mgr);

// 创建会话
let mut session = mgr.get_or_create("alice", "mybot", "session-1")?;

// 追加消息
mgr.add_message(&session, "user".into(), "Hello".into()).await?;

// 召回相关记忆
let memories = retriever.recall("alice", "how to handle auth", 5).await?;
for m in &memories {
    println!("[{}] {}", m.memory_type, m.snippet);
}

// 提交（提取记忆）
mgr.commit_session(&mut session).await?;
```

---

## 五、与 OpenViking 插件架构的对比

| 维度 | OpenViking Plugin | Runestone 方案 |
|------|------------------|----------------|
| Hook 数量 | 7 个 | 4 个（SessionStart/UserPromptSubmit/Stop/SessionEnd） |
| 存储后端 | OpenViking Server (HTTP) | 本地 Git + 文件系统 |
| 召回方式 | 向量检索（服务端） | CLI 调用 + 文件直读 |
| 异步写入 | detach-worker 模式 | 同样支持（子进程分离） |
| 自污染防护 | 剥离注入块 | 同样剥离 |
| 递归防护 | 不复现（hook 脚本不调 claude） | 同样 |
| 部署依赖 | 需要 OV Server 运行 | **零依赖**，单二进制 |
| MCP 工具 | 9 个 | 延后（Phase 4，优先级降低） |

Runestone 的独特优势：

- **无服务依赖**：不需要额外运行 daemon，纯文件系统 + CLI
- **Git 原生**：记忆变更即 Git commit，天然支持回滚和同步
- **单一二进制**：复制即安装，适合 CI/CD 和容器化场景
- **CLAUDE.md 兜底**：即使 hook 失败，静态注入确保基础上下文可用

---

## 六、实现优先级

| 优先级 | 功能 | 说明 |
|--------|------|------|
| P0 | `memory recall` CLI 命令 | 语义召回，Phase 3 |
| P0 | `memory capture` CLI 命令 | 增量提取，Phase 2 |
| P1 | `context inject` CLI 命令 | SessionStart 注入，Phase 4 |
| P1 | `memory finalize` CLI 命令 | SessionEnd 提交 + CLAUDE.md 更新 |
| P1 | Hook 脚本模板 | 4 个 hook 的 shell 脚本 |
| P2 | `resource add` CLI 命令 | 资源导入，Phase 2 |
| P3 | Rust lib 公开 API | 文档 + 示例 |
| P4 | MCP Server | 延后（Phase 4，优先级降低） |
