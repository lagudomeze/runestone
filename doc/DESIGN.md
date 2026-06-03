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

### 4.3 Retriever + Index Manager（Phase 3）

- 语义检索 + 目录递归检索
- 向量库：SQLite + sqlite-vec 扩展
- 索引存储：`~/.local/share/runestone/embeddings/{owner}.db`
- 支持增量索引和全量重建

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
