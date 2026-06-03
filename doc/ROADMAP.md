# Runestone 路线图

## 总览

```text
Phase 1 (已完成)     Phase 2 (当前)      Phase 3             Phase 4             Phase 5
基础骨架             记忆提取             检索与索引           Git 同步            增强与优化
▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
```

## Phase 1 — 基础骨架 ✓

**目标**：可用的 CLI 工具，支持基本的会话管理和 Git 版本化。

- [x] 项目初始化、依赖配置
- [x] 错误处理框架（exn + thiserror）
- [x] GitRepo 封装（init, add, commit）
- [x] SessionManager（创建、追加消息、偏移量提交）
- [x] MemoryChange 类型定义
- [x] CLI 命令（session create/add/commit/history）
- [x] 单元测试 + 集成测试
- [ ] 发布 v0.1.0

## Phase 2 — 记忆提取 🚧

**目标**：集成 LLM，从会话消息中自动提取结构化记忆。

- [ ] LLM 客户端抽象（OpenAI 兼容接口）
- [ ] MemoryExtractor 实现
- [ ] LLM 提示词模板（提取 profile/preferences/entities/events/cases）
- [ ] 记忆变更写入文件系统（全局 + Agent）
- [ ] 增量摘要合并（.abstract.md / .overview.md）
- [ ] 测试增量 commit 流程
- [ ] 发布 v0.2.0

### 依赖库
- `reqwest` + `async-trait`（HTTP 客户端）
- `tokio`（已有）

## Phase 3 — 检索与向量索引

**目标**：实现语义检索，支持 URI 寻址。

- [ ] Embedding 模型集成（candle / ort / 外部 API）
- [ ] EmbeddingDatabase（SQLite + sqlite-vec）
- [ ] 文件索引（分段、向量化、存储）
- [ ] 增量索引更新（commit 后自动触发）
- [ ] Retriever::semantic_search
- [ ] Retriever::list_recursive（URI 目录遍历）
- [ ] URI 解析器（viking://...）
- [ ] CLI: memory search / memory list
- [ ] CLI: index rebuild
- [ ] 发布 v0.3.0

### 依赖库
- `rusqlite`（SQLite）
- `candle` 或 `ort`（Embedding）
- `tokenizers`（文本分段）

## Phase 4 — Git 同步

**目标**：支持跨设备同步。

- [ ] GitRepo: pull_rebase / push
- [ ] 远程 URL 配置
- [ ] 冲突检测与提示
- [ ] CLI: git sync
- [ ] 发布 v0.4.0

## Phase 5 — 增强与优化（可选）

**目标**：完善体验，扩展使用场景。

- [ ] HTTP API 服务（axum）
- [ ] 本地 LLM 支持（llama.cpp / ollama）
- [ ] 配置文件（~/.config/runestone/config.toml）
- [ ] 日志完善 + 性能优化
- [ ] 发布 v1.0.0

### 备选增强
- [ ] TUI 界面（ratatui）
- [ ] 记忆去重 / 冲突合并
- [ ] 多语言 Embedding 支持
- [ ] 记忆导出（JSON / Markdown）

## 版本规划

| 版本 | 阶段 | 核心交付 |
|------|------|----------|
| v0.1.0 | Phase 1 | 会话管理 + Git 版本化 CLI |
| v0.2.0 | Phase 2 | LLM 记忆提取 |
| v0.3.0 | Phase 3 | 语义检索 |
| v0.4.0 | Phase 4 | Git 远程同步 |
| v1.0.0 | Phase 5 | 稳定版 + 可选增强 |
