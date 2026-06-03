# Runestone vs OpenViking 对比分析

## OpenViking 是什么

OpenViking 是一个面向 AI 智能体的记忆系统，提供了会话存储、LLM 驱动的记忆提取、以及语义检索能力。Runestone 在设计上与 OpenViking 保持核心语义兼容，同时用 Rust 重写以提供更高的性能和更好的部署体验。

## 功能对比

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| 会话消息存储 (JSONL) | ✓ | ✓ | 已实现 |
| 偏移量增量提交 | ✓ | ✓ | 已实现 |
| 消息历史读取 | ✓ | ✓ | 已实现 |
| LLM 记忆提取 | ✓ | — | Phase 2 |
| 全局记忆 (profile/preferences/entities/events) | ✓ | 类型已定义 | Phase 2 |
| Agent 专属记忆 (cases/patterns/tools/skills) | ✓ | 类型已定义 | Phase 2 |
| 会话摘要 (.abstract.md / .overview.md) | ✓ | — | Phase 2 |
| 语义检索 (向量) | ✓ | — | Phase 3 |
| 目录递归检索 | ✓ | — | Phase 3 |
| Git 版本化存储 | ✓ | ✓ | 已实现 |
| Git 远程同步 (pull/push) | ✓ | — | Phase 4 |
| 多 Agent 隔离 | ✓ | ✓ | 已实现 |
| URI 寻址 (viking://...) | ✓ | — | Phase 3 |
| CLI 工具 | ✓ | ✓ | 已实现 |
| HTTP API | ✓ | — | Phase 5（可选） |

## 核心差异

### 1. 语言和运行时

| | OpenViking | Runestone |
|--|-----------|-----------|
| 语言 | Python | **Rust 2024** |
| 运行时 | CPython / 依赖管理复杂 | **单一静态二进制** |
| 性能 | GIL 限制 | **零成本抽象、无 GC** |
| 内存 | 较高 | **低** |
| 分发 | pip install + 依赖 | **单文件复制** |

### 2. 存储策略

| | OpenViking | Runestone |
|--|-----------|-----------|
| Git 后端 | 可能使用 dulwich 或 git CLI | **git2 (libgit2 绑定)** |
| 文件锁 | — | **tokio::Mutex + fd-lock** |
| 并发安全 | GIL 保护 | **异步锁 + 原子操作** |

### 3. 向量索引

| | OpenViking | Runestone |
|--|-----------|-----------|
| 向量数据库 | 可能使用 ChromaDB 或 FAISS | **SQLite + sqlite-vec**（嵌入式，无额外进程） |
| Embedding | 外部 API 或本地模型 | **candle / ort**（纯 Rust / ONNX，可选外部 API） |

### 4. 部署模式

| | OpenViking | Runestone |
|--|-----------|-----------|
| 本地优先 | 是 | **是** |
| 无外部依赖 | 否（Python 运行时） | **是（单一二进制）** |
| 跨平台 | ✓ | **✓（Linux/macOS/Windows）** |

## 与原始需求对齐

Runestone 在以下方面**新增**了原始设计中未强调的能力：

- **单一二进制分发** — Rust 编译为静态链接的单个可执行文件
- **类型安全** — 编译期保证数据结构正确性
- **更低资源占用** — 适合边缘设备或长期运行

以下特性被**有意简化**或延后：

- **HTTP API** — Phase 5 可选，优先完善 CLI
- **工具调用记录** (tools/{tool_id}.json) — 暂不实现，等待实际需求
- **TUI/Web UI** — 暂不实现，CLI 优先

## URI 互操作

Runestone 将实现与 OpenViking 兼容的 URI 寻址格式：

```
viking://{owner}/agents/{agent_id}/memory/
viking://{owner}/agents/{agent_id}/sessions/{session_id}/
viking://{owner}/memory/
```

URI 解析将在 Phase 3（Retriever）中实现。
