# Runestone 路线图

## 总览

| 阶段 | 状态 | 核心交付 |
| --- | --- | --- |
| Phase 1 — 基础骨架 | ✅ 完成 | 会话管理 + Git 版本化 + CLI |
| Phase 2 — 记忆提取 | 🚧 当前 | LLM 记忆提取、资源导入 |
| Phase 3 — 检索与索引 | 🔲 计划 | 语义检索、向量索引、Agent 集成 |
| Phase 4 — 服务化 | 🔲 计划 | HTTP API、Web-UI Dashboard |
| Phase 5 — 同步与增强 | 🔲 计划 | Git 同步、去重、技能管理 |

## 架构调整

项目拆分为 **lib** + **cli** 两层：

```text
src/
├── lib.rs               # 库入口：所有公开 API
├── error.rs             # 错误类型
├── git_repo.rs          # Git 存储
├── session.rs           # 会话管理
├── memory.rs            # 记忆类型 + MemoryExtractor
├── resource.rs          # 资源管理（导入、分层解析）
├── skill.rs             # 技能管理
├── retriever.rs         # 检索器
├── index.rs             # 向量索引
└── bin/
    └── runestone.rs     # CLI 入口（二进制）
```

- **lib**：提供完整的功能接口，其他 Rust 应用可直接集成
- **cli**：将 lib 发布为 CLI 工具 + 可选 HTTP 服务
- **Agent 集成**：通过 CLI + Hooks 模式嵌入 Claude Code / OpenClaw 等 Agent（详见 [INTEGRATION.md](INTEGRATION.md)）

---

## Phase 1 — 基础骨架 ✅

- [x] 项目初始化、依赖配置
- [x] 错误处理框架（exn + thiserror）
- [x] GitRepo 封装（init, add, commit）
- [x] SessionManager（创建、追加消息、偏移量提交）
- [x] MemoryChange 类型定义（6 类记忆 + 摘要）
- [x] CLI 命令（session create/add/commit/history）
- [x] 单元测试 + CI
- [ ] lib/cli 目录拆分

## Phase 2 — 记忆提取 🚧

**目标**：LLM 从会话中提取结构化记忆，支持资源导入和解析。

- [ ] LLM 客户端抽象（OpenAI 兼容接口）
- [ ] MemoryExtractor：消息 → 结构化记忆
- [ ] 记忆变更写入文件系统（全局 + Agent）
- [ ] 增量摘要合并（.abstract.md / .overview.md）
- [ ] **资源导入**（URL、本地文件、GitHub 仓库）
- [ ] **L0/L1/L2 分层解析**（摘要 → 概览 → 详情）

## Phase 3 — 检索与索引 🔲

**目标**：语义检索，URI 寻址，Agent 可调用。

- [ ] Embedding 模型集成
- [ ] EmbeddingDatabase（SQLite + sqlite-vec）
- [ ] 增量索引更新
- [ ] `memory recall` CLI（语义召回，供 Agent Hooks 调用）
- [ ] `memory capture` CLI（增量提取，供 Stop Hook 调用）
- [ ] `context inject` CLI（SessionStart 上下文注入）
- [ ] viking:// URI 解析
- [ ] **去重**（候选级 + 条目级）
- [ ] Hook 脚本模板（4 个生命周期 Hooks）

## Phase 4 — 服务化 🔲

**目标**：可选的 Web 界面和服务接口。

- [ ] **HTTP API**（axum，端口 1933，兼容 OpenViking API）
- [ ] **Web-UI Dashboard**（会话列表、记忆浏览、检索可视化）
- [ ] `memory finalize` CLI（SessionEnd 提交 + CLAUDE.md 更新）
- [ ] lib 接口文档
- [ ] MCP Server（优先级降低，延后评估）

## Phase 5 — 同步与增强 🔲

- [ ] Git 远程同步（pull/push）
- [ ] **技能管理**（skill 注册、版本化、依赖管理）
- [ ] 多模态资源支持（PDF、图片、代码仓库）
- [ ] 性能优化、发布 v1.0.0

---

## 版本规划

| 版本 | 阶段 | 核心交付 |
| --- | --- | --- |
| v0.1.0 | Phase 1 | 会话管理 + Git 版本化 + 库接口 |
| v0.2.0 | Phase 2 | LLM 记忆提取 + 资源导入 |
| v0.3.0 | Phase 3 | 语义检索 + Agent Hook 集成 |
| v0.4.0 | Phase 4 | HTTP API + Dashboard |
| v1.0.0 | Phase 5 | 远程同步 + 技能管理 + 稳定版 |
