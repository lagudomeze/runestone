# Runestone 设计文档

## 一、项目定位

**Runestone** 是一个面向个人 AI 智能体的、基于 Rust + Git 的长期记忆系统。

- **核心价值**：为 AI 应用提供会话式记忆存储、增量式记忆提取、版本化持久化，以及基于本地 Embedding 的语义检索。
- **设计原则**：本地优先、Git 原生、Rust 实现、兼容 OpenViking 的核心语义。
- **目标用户**：希望拥有完全自控、跨设备同步、可审计的个人知识库的开发者。

## 二、整体架构

```text
┌─────────────────────────────────────────────────────────┐
│                    CLI (runestone)                       │
├─────────────────────────────────────────────────────────┤
│                     HTTP API (可选)                      │
├─────────────────────────────────────────────────────────┤
│                      Core Components                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│  │ Session  │ │ Memory   │ │ Retriever│ │  Index   │   │
│  │ Manager  │ │ Extractor│ │          │ │ Manager  │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
├─────────────────────────────────────────────────────────┤
│              Storage Backend (Git + FS)                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │  GitRepo: 每个 owner 独立仓库，文件系统映射        │  │
│  │  + 文件锁 + 增量 commit + 可选远程同步             │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## 三、目录结构

```
/{owner}/
├── .git/
├── agents/
│   └── {agent_id}/
│       ├── sessions/
│       │   └── {session_id}/
│       │       ├── messages.jsonl        # 追加式写入，永不删除
│       │       ├── .commit_offset        # 已处理的最后一行号
│       │       ├── .abstract.md          # 当前 L0 摘要（增量更新）
│       │       ├── .overview.md          # 当前 L1 概览（增量更新）
│       │       └── tools/                # 可选：会话级工具调用记录
│       │           └── {tool_id}.json
│       └── memory/                       # Agent 专属长期记忆
│           ├── cases/                    # 案例库（问题+解法）
│           ├── patterns/                 # 可复用模式
│           ├── tools/                    # 工具使用知识
│           └── skills/                   # 技能工作流
└── memory/                               # 用户全局共享记忆
    ├── profile.md                        # 个人档案（姓名、职业等）
    ├── preferences/                      # 偏好（如语言、风格）
    ├── entities/                         # 实体（项目名、人名）
    └── events/                           # 重要事件/决策
```

## 四、核心组件

### 4.1 Session Manager

负责会话生命周期管理：创建、追加消息、基于偏移量的提交、读取完整历史。

**Commit 流程**：
1. 加文件锁
2. 读取 `messages.jsonl` 总行数
3. 计算增量消息
4. 调用 Memory Extractor 提取记忆（异步，Phase 2）
5. 应用变更：更新全局/Agent 记忆文件，更新摘要
6. 写入新 offset 到 `.commit_offset`
7. `git add` + `git commit`
8. 释放锁

### 4.2 Memory Extractor（Phase 2）

调用 LLM 从新消息中提取结构化记忆，支持增量摘要合并。

**MemoryChange 类型**：
- `GlobalProfile` / `GlobalPreference` / `GlobalEntity` / `GlobalEvent`
- `AgentCase` / `AgentPattern` / `AgentTool` / `AgentSkill`
- `UpdateAbstract` / `UpdateOverview`

### 4.3 L0/L1/L2 分层上下文

参考 OpenViking，Runestone 实现了三层上下文加载策略。写入时生成摘要，检索时逐级展开，用 LLM 做路由决策而非纯向量匹配。

#### 目录结构

```
{owner}/
├── memory/                              ← .abstract.md + .overview.md (L0+L1)
│   ├── profile.md                       ← 叶子 (L2)
│   ├── preferences/
│   │   ├── .abstract.md                 ← L0
│   │   ├── editor.md                    ← L2
│   │   └── language.md                  ← L2
│   ├── entities/
│   │   ├── .abstract.md                 ← L0
│   │   └── redis.md                     ← L2
│   └── events/
│       ├── .abstract.md                 ← L0
│       └── decided-redis.md             ← L2
└── agents/{agent}/memory/               ← .abstract.md + .overview.md (L0+L1)
    ├── cases/
    │   ├── .abstract.md                 ← L0
    │   ├── fix-timeout.md               ← L2
    │   └── auth-errors.md               ← L2
    └── patterns/
        ├── .abstract.md                 ← L0
        └── error-handling.md            ← L2
```

| 类型 | abstract (L0) | overview (L1) |
|------|:---:|:---:|
| 叶子 `.md` 文件 | — | — |
| 包含文件的子目录 | ✅ | — |
| 顶级 memory 目录 | ✅ | ✅ |

#### 三层定义

| 层 | 文件 | 大小 | 用途 |
|----|------|------|------|
| L0 | `.abstract.md` | ~100 tokens | 一句话汇总目录内容，用于向量初筛 |
| L1 | `.overview.md` | ~1-2k tokens | 文件清单 + 各一行描述，LLM 导航决策 |
| L2 | 原始 `.md` 文件 | 不限 | 完整内容，按需加载 |

#### 生成策略：自底向上

任何写操作（`memory_store` / `session_commit` 提取）之后：

1. 检测脏目录（有文件变更的目录）
2. 从最深层叶子目录开始，LLM 汇总目录内所有 L2 文件 → 写入 `.abstract.md`（L0）
3. 向上一级，LLM 聚合所有子目录的 L0 → 写入父目录 L0
4. 顶级目录额外生成 `.overview.md`（L1）：文件清单 + 各一行描述

#### 目录递归检索

```
query → embed → 在所有 L0 上向量搜索 → top-K 候选
  → LLM 遍历候选的 L0 文本，判断哪些相关
  → 对命中目录，读 L1 (.overview) 确认细节
  → 需要详情的，加载 L2 文件
  → 返回结果（路径 + 摘要 + 分数）
```

每一层决策由 LLM 在文本上做判断，不做纯向量匹配。每步消耗控制在 100-2000 tokens。

### 4.4 Memory 产生路径

| Case | 触发方式 | 说明 |
|------|---------|------|
| 直接写入 | `rs.memory_store(kind, value)` | 应用显式设置偏好/实体/事件 |
| 会话提取 | `agent.session_commit()` | LLM 从对话自动提取 MemoryChange |
| 资源导入 | `rs.resource_add(uri)` | URL/文件/GitHub → 解析 → memory（Phase 2） |
| Agent 自主记忆 | `agent.memory_store(case, content)` | Agent 主动决策记录（Phase 4） |

### 4.5 Retriever + Index Manager

- L0 向量索引：每个 `.abstract.md` 生成 embedding，存储在内存/本地文件中
- 增量索引：commit 后只重索引变更的 L0 文件
- 全量重建：`index rebuild` CLI 命令

### 4.4 Git 存储封装

- 每个 owner 独立仓库
- 封装 git2 (libgit2)：init, add, commit
- 支持配置远程 URL（Phase 4）

## 五、接口设计

### CLI 命令

```bash
# 会话管理
runestone session create --owner <o> --agent <a> --session <s>
runestone session add --owner <o> --agent <a> --session <s> --role <r> --content <c>
runestone session commit --owner <o> --agent <a> --session <s>
runestone session history --owner <o> --agent <a> --session <s>

# 记忆检索（Phase 3）
runestone memory search --owner <o> --query <q> --scope global,agent=<a>
runestone memory list --owner <o> --agent <a> --type cases

# Git 同步（Phase 4）
runestone git sync --owner <o>

# 索引管理（Phase 3）
runestone index rebuild --owner <o>
```

## 六、技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 编程语言 | Rust 2024 (stable) | 性能、内存安全 |
| Git 操作 | `git2` (libgit2) | 稳定、功能完整、API 简洁 |
| 异步运行时 | `tokio` | 主流、生态成熟 |
| 命令行 | `clap` | 功能强大、派生宏友好 |
| 序列化 | `serde` + `serde_json` | 标准方案 |
| 错误处理 | `exn` + `thiserror` | 结构化错误 + 自动回溯链 |
| 向量数据库 | SQLite + `sqlite-vec` | 轻量、无额外进程、可嵌入 |
| Embedding | `candle` 或 `ort` | 纯 Rust 或 ONNX Runtime |
| LLM 客户端 | `reqwest` + `async-trait` | 兼容 OpenAI API 格式 |
| 文件锁 | `fd-lock` | 简单可靠 |
| HTTP 服务 | `axum` | 可选，现代 Rust 风格 |
