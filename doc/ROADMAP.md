# Runestone 路线图

## 总览

| 阶段 | 状态 | 核心交付 |
| --- | --- | --- |
| Phase 1 — 基础骨架 | ✅ 完成 | 会话管理、Git、lib/bin 拆分、Agent 抽象、MemoryKind trait |
| Phase 2 — 记忆提取 | ✅ 完成 | rig Extractor、apply_changes、L0/L1 生成、NoopExtractor |
| Phase 3 — 检索与索引 | 🚧 当前 | 向量索引、递归检索、Agent Hook CLI |
| Phase 4 — 服务化 | 🔲 计划 | HTTP API、Web-UI Dashboard |
| Phase 5 — 同步与增强 | 🔲 计划 | Git 同步、去重、技能管理 |

---

## Phase 1 — 基础骨架 ✅

- [x] 项目初始化、依赖配置
- [x] 错误处理框架（exn + thiserror）
- [x] GitRepo 封装（init, add, commit）
- [x] SessionManager 泛型（CRUD、偏移量提交）
- [x] MemoryChange 类型定义（10 种）
- [x] CLI 命令（session create/add/commit/history）
- [x] 单元测试 + CI
- [x] Workspace 拆分（crates/runestone + crates/runestone-cli）
- [x] Runestone + Agent 门面，MemoryKind trait
- [x] 45 个测试（unit + integration + doctest）

## Phase 2 — 记忆提取 ✅

- [x] rig Extractor trait（extract / summarize_directory / generate_overview）
- [x] RigExtractor<M>（任意 CompletionModel）
- [x] NoopExtractor（显式空实现，取代隐式 `()` 默认）
- [x] session_commit 自动提取 + 写入磁盘（apply_changes）
- [x] L0 .abstract.md 自底向上生成（commit 时 + memory_store 时）
- [x] L1 .overview.md 顶级目录生成
- [x] memory_store / memory_load / memory_list（Runestone + Agent 两层）
- [ ] resource_add（资源导入 — stub）

## Phase 3 — 检索与索引 🚧

**已完成：**

- [x] 关键词搜索（retriever.rs），优先 L0 abstract（3x 权重）
- [x] MemoryHit 结构（path + snippet + score）
- [x] CLI: `memory search` / `memory list`

**待实现：**

- [ ] L0 向量索引（rig-fastembed，替换关键词匹配）
- [ ] 目录递归检索（query → embed L0 → LLM 路由 → L1 过滤 → L2 加载）
- [ ] `memory recall` CLI（语义召回，供 Agent Hooks 调用）
- [ ] `memory capture` CLI（增量提取，供 Stop Hook 调用）
- [ ] `context inject` CLI（SessionStart 上下文注入）
- [ ] Hook 脚本模板（4 个生命周期 Hooks）
- [ ] index rebuild CLI（替换 stub）

## Phase 4 — 服务化 🔲

- [ ] HTTP API（axum，兼容 OpenViking API）
- [ ] Web-UI Dashboard
- [ ] `memory finalize` CLI（SessionEnd + CLAUDE.md 更新）
- [ ] lib 接口文档

## Phase 5 — 同步与增强 🔲

- [ ] Git 远程同步（pull/push）
- [ ] 去重（候选级 + 条目级）
- [ ] 技能管理（skill 注册、版本化）
- [ ] 多模态资源支持（PDF、图片）
- [ ] URI 解析（viking://...）
- [ ] 性能优化、v1.0

---

## 版本规划

| 版本 | 阶段 | 核心交付 |
| --- | --- | --- |
| v0.1.0 | Phase 1+2 | 会话管理 + LLM 提取 + L0/L1 生成 |
| v0.2.0 | Phase 3 | 向量索引 + 递归检索 + Agent Hook CLI |
| v0.3.0 | Phase 4 | HTTP API + Dashboard |
| v1.0.0 | Phase 5 | 远程同步 + 去重 + 技能 + 稳定版 |
