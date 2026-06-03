# Runestone vs OpenViking 对比分析

## OpenViking 是什么

OpenViking 是字节跳动火山引擎开源的 **AI Agent 上下文数据库**（Context Database），采用文件系统范式统一管理 Agent 的**记忆（Memory）、资源（Resource）和技能（Skill）**。

- 仓库：<https://github.com/volcengine/OpenViking>
- 协议：Apache 2.0
- 语言：Python（服务端 + SDK）+ Rust CLI
- 典型集成：OpenClaw、Claude Code

## 功能对比

### 记忆管理

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| 会话消息存储 (JSONL) | ✓ | ✓ | Phase 1 |
| 偏移量增量提交 | ✓ | ✓ | Phase 1 |
| LLM 记忆提取 | ✓ | — | Phase 2 |
| 全局记忆 (Profile/Preferences/Entities) | ✓ | 类型已定义 | Phase 2 |
| **决策/事件记录** (GlobalEvent) | ✓ | 类型已定义 | Phase 2 |
| Agent 专属记忆 (Cases/Patterns/Tools/Skills) | ✓ | 类型已定义 | Phase 2 |
| 会话摘要 (.abstract.md / .overview.md) | ✓ | — | Phase 2 |
| L0/L1/L2 分层上下文加载 | ✓ | — | Phase 2 |

### 资源管理

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| URL 资源导入 | ✓ | — | Phase 2 |
| 本地文件上传 | ✓ | — | Phase 2 |
| GitHub 仓库导入 | ✓ | — | Phase 2 |
| 多模态资源 (PDF/图片/代码) | ✓ | — | Phase 5 |
| 资源 L0/L1/L2 分层解析 | ✓ | — | Phase 2 |
| 资源目录浏览 (viking://resources/) | ✓ | — | Phase 3 |

### 技能管理

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| 技能注册/管理 | ✓ | — | Phase 5 |
| 容器化技能 (Docker) | ✓ | — | 待评估 |
| 技能独立上下文依赖 | ✓ | — | 待评估 |

### 检索与索引

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| 语义检索 (向量) | ✓ | — | Phase 3 |
| 目录递归检索 | ✓ | — | Phase 3 |
| URI 寻址 (viking://...) | ✓ | — | Phase 3 |
| 去重（候选级 + 条目级） | ✓ | — | Phase 3 |
| 可视化检索轨迹 (DAG) | ✓ | — | 待评估 |

### 接口与服务

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| CLI 工具 | ✓ | ✓ | Phase 1 |
| REST API (端口 1933) | ✓ | — | Phase 4 |
| Python SDK | ✓ | — | 待评估（优先 Rust + MCP） |
| **MCP Server** | ✓ | — | Phase 4 |
| Web-UI Dashboard | — | — | Phase 4 |
| Claude Code / OpenClaw 集成 | ✓ | — | Phase 4 |

### 存储与同步

| 特性 | OpenViking | Runestone | 状态 |
|------|-----------|-----------|------|
| Git 版本化存储 | ✓ | ✓ | Phase 1 |
| Git 远程同步 | ✓ | — | Phase 5 |
| LSM-tree 元数据存储 | ✓ | — | 不采用（Git 替代） |

## 核心差异

### 1. 架构定位

| | OpenViking | Runestone |
|--|-----------|-----------|
| 定位 | Context Database（上下文数据库） | **Memory System（记忆系统）** |
| 范式 | 虚拟文件系统 (viking://) | **物理文件系统 + Git** |
| 部署 | 独立服务 (openviking-server) | **嵌入式库 + 可选服务** |

### 2. 语言和运行时

| | OpenViking | Runestone |
|--|-----------|-----------|
| 语言 | Python（服务端）+ Rust CLI | **纯 Rust 2024** |
| 运行时 | 需要 Python 环境 | **单一静态二进制** |
| 性能 | Python GIL 限制 | **零成本抽象** |

### 3. 存储策略

| | OpenViking | Runestone |
|--|-----------|-----------|
| 元数据 | 改进 LSM-tree | **Git + 文件系统** |
| 向量索引 | HNSW | **SQLite + sqlite-vec** |
| 文件锁 | — | **tokio::Mutex** |

### 4. 与原始需求对齐

Runestone 在以下方面**新增**：

- **嵌入式 lib 设计** — 其他 Rust 应用可直接集成，无需独立服务
- **纯 Rust 实现** — 编译为单一静态二进制，零运行时依赖
- **Git 原生版本化** — 所有记忆变更可追溯、可回滚

以下 OpenViking 特性被**有意延后**或省略：

- **Python SDK** — 优先提供 MCP + Rust lib 接口
- **Docker 容器化技能** — 轻量级替代方案（Phase 5 评估）
- **LSM-tree 存储** — Git 文件系统已满足版本化需求
- **可视化检索轨迹** — Phase 4 Dashboard 中考虑
