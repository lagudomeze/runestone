# Rune — 个人记忆管理

跨项目的个人记忆入口。基于 Runestone 读写 `./data/{owner}/memory/`。

## 子命令

### remember — 捕获记忆

```
/rune remember <种子描述>
```

1. 理解用户意图：是什么（现象/偏好/概念/经验）
2. 分类为 `profile` / `preference` / `entity` / `case`
3. 生成标题（简洁、文件名风格，支持中英文）
4. 展开为 3-5 句：是什么、为什么重要、怎么用。200 字内
5. 执行：

```bash
source .envrc 2>/dev/null
runestone --owner "${RUNESTONE_OWNER:-$(whoami)}" memory store \
  --kind <kind> --key "<title>" --content "<content>"
```

| kind | 适用 |
|------|------|
| `profile` | 身份、角色、背景 |
| `preference` | 习惯、风格、工作方式 |
| `entity` | 概念、模式、知识点 |
| `case` | 经验、bug 模式、设计决策 |

### recall — 搜索 + 应用

```
/rune recall <主题>
```

1. 用 `runestone memory list` 列出记忆文件（快速）或用关键词搜索
2. 展示匹配结果（类型 + 标题 + 摘要）
3. **主动问用户**：是否需要将这些记忆应用到当前项目？（写入 CLAUDE.md、生成 skill、设定规则等）
4. 如果需要，根据记忆内容和当前项目上下文，生成对应的项目文件并写入

### list — 列出所有

```
/rune list
```

列出所有记忆文件，按类型分组显示路径和内容摘要。

### sync — 同步远程

```
/rune sync
```

```bash
source .envrc && runestone --owner "$RUNESTONE_OWNER" git sync --remote "$RUNESTONE_REMOTE"
```

### clean — 整理

```
/rune clean
```

1. 先 `runestone --owner "$OWNER" memory clean --dry-run` 预览重复
2. 展示给用户确认
3. 用户确认后执行 `runestone memory clean` 合并
4. 对于 CLI 未检测到的冗余，手动提出建议：哪些过时、哪些可合并
