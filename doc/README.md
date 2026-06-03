# Runestone

面向个人 AI 智能体的、基于 Rust + Git 的长期记忆系统。

## 核心价值

- **会话式记忆存储**：追加式 JSONL，消息永不丢失
- **增量式记忆提取**：基于偏移量的增量处理，LLM 自动提取结构化记忆
- **版本化持久化**：所有数据存储在 Git 仓库中，支持时间旅行和跨设备同步
- **语义检索**：本地向量索引（SQLite + sqlite-vec），随时可重建

## 设计原则

- **本地优先**：不依赖外部服务，数据完全自控
- **Git 原生**：利用 Git 的版本控制能力管理记忆演化
- **Rust 实现**：高性能、内存安全、易于分发为单一二进制文件
- **兼容 OpenViking**：核心语义对齐，URI 互操作

## 快速开始

```bash
# 创建会话
runestone session create --owner alice --agent mybot --session s1

# 添加消息
runestone session add --owner alice --agent mybot --session s1 \
    --role user --content "Hello, I am Bob"

# 提交（触发记忆提取）
runestone session commit --owner alice --agent mybot --session s1

# 查看历史
runestone session history --owner alice --agent mybot --session s1
```

## 数据结构

```
./data/{owner}/                   # 每个 owner 独立的 Git 仓库
├── agents/{agent_id}/
│   ├── sessions/{session_id}/
│   │   ├── messages.jsonl        # 追加式写入
│   │   ├── .commit_offset        # 已处理偏移量
│   │   ├── .abstract.md          # 会话摘要（增量更新）
│   │   └── .overview.md          # 会话概览
│   └── memory/                   # Agent 专属记忆
│       ├── cases/                # 案例库
│       ├── patterns/             # 可复用模式
│       ├── tools/                # 工具知识
│       └── skills/               # 技能工作流
└── memory/                       # 全局共享记忆
    ├── profile.md
    ├── preferences/
    ├── entities/
    └── events/
```

## 技术栈

| 组件 | 选型 | 说明 |
|------|------|------|
| 语言 | Rust 2024 (stable) | |
| Git | git2 (libgit2) | 成熟稳定，API 简洁 |
| 异步 | tokio | 主流、生态成熟 |
| CLI | clap | 功能强大、派生宏友好 |
| 序列化 | serde + serde_json | 标准方案 |
| 错误处理 | exn + thiserror | 结构化错误 + 自动回溯 |
| 日志 | tracing | 结构化日志 |

## 路线图

参见 [ROADMAP.md](ROADMAP.md)。

## 与 OpenViking 的差异

参见 [COMPARISON.md](COMPARISON.md)。
