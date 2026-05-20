# 📋 MemoryNexus 开发计划

> 最后更新: 2026-05-20
> 管理方式: GitHub Issues + Markdown 文档

---

## 🎯 当前版本: v0.1.0

---

## ✅ 已完成

| ID | 任务 | 优先级 | 完成日期 | PR/Commit |
|----|------|--------|----------|-----------|
| P0.1 | Rust 项目初始化 | P0 | 2026-05-16 | 5837ad5 |
| P0.2 | 数据库连接层 | P0 | 2026-05-16 | d7416b7 |
| P0.3 | 用户认证 API | P0 | 2026-05-16 | 87f6138 |
| P0.4 | 记忆 CRUD API | P0 | 2026-05-16 | 8714d9c |
| P1.1 | S3 存储层抽象 | P1 | 2026-05-17 | 0f3ce27 |
| P1.2 | 标签系统 | P1 | 2026-05-17 | 6239d62 |
| P1.3 | 搜索与过滤 | P1 | 2026-05-17 | b118427 |
| P1.4 | AI 摘要与智能标签 | P1 | 2026-05-17 | 99619bf |
| P1A.1 | Cognitive Space 基础模型与 CLI 入口 | P0 | 2026-05-20 | - |
| P1B.1 | 本地语义检索基础链路 | P0 | 2026-05-20 | - |

---

## 🔴 P2 — AI 功能（核心）

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P2.1 | Embedding 集成 | ✅ Done | P0 | Codex |
| P2.2 | Qdrant 向量存储 | ✅ Done | P0 | Codex |
| P2.3 | 语义搜索实现 | ✅ Done | P0 | Codex |
| P2.4 | AI 摘要完善 | 🟡 Todo | P1 | - |
| P2.5 | 智能标签生成 | 🟡 Todo | P1 | - |

---

## 🔴 P1A — Cognitive Space 基础闭环

目标：把 memory 的归属边界从“用户/Agent”推进到 Cognitive Space。

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P1A.1 | 新增 `cognitive_spaces` 和 `cognitive_space_members` 迁移 | ✅ Done | P0 | Codex |
| P1A.2 | 注册时创建默认 personal Cognitive Space | ✅ Done | P0 | Codex |
| P1A.3 | `memories.space_id` 落库并进入 memory 创建/列表/search 路径 | ✅ Done | P0 | Codex |
| P1A.4 | Qdrant payload/search filter 增加 `space_id` | ✅ Done | P0 | Codex |
| P1A.5 | 新增 Space REST API：create/list/get | ✅ Done | P0 | Codex |
| P1A.6 | CLI 支持 `space create/list` 与 `--space` 参数 | ✅ Done | P0 | Codex |
| P1A.7 | 本地数据库迁移 smoke test | ✅ Done | P0 | Codex |

---

## 🔴 P1B — Semantic Index 基础闭环

目标：让 memory create 后可以进入向量索引，并通过同一 Cognitive Space 内的 semantic search 被召回。

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P1B.1 | 新增 deterministic local embedding provider，支持无外部 API 的本地烟测 | ✅ Done | P0 | Codex |
| P1B.2 | Rust 服务启动时确保 Qdrant collection 存在 | ✅ Done | P0 | Codex |
| P1B.3 | memory create 复用全局 embedder 并 upsert 到 Qdrant | ✅ Done | P0 | Codex |
| P1B.4 | 向量 payload 补齐 `space_id`、`memory_id`、`source_type`、`created_at`、`visibility` | ✅ Done | P0 | Codex |
| P1B.5 | `search --semantic --space <SPACE_ID>` 保持空间隔离过滤 | ✅ Done | P0 | Codex |
| P1B.6 | 本地 Qdrant + CLI semantic smoke test | ✅ Done | P0 | Codex |
| P1B.7 | 注册、space、memory、semantic search 的端到端自动化验收 | 🟡 Todo | P0 | Codex |

---

## 🟠 P2.5 — CLI MVP（最小试用入口）

目标：先用一个很薄的 Rust CLI 验证后端 API 手感，不等待前端完成。

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| CLI.1 | 新增 `memorynexus-cli` Rust binary | ✅ Done | P0 | Codex |
| CLI.2 | 支持 `health` 命令 | ✅ Done | P0 | Codex |
| CLI.3 | 支持 `auth register/login` 命令，登录结果输出 token | ✅ Done | P0 | Codex |
| CLI.4 | 支持 `memory add/list/get/delete` 命令 | ✅ Done | P0 | Codex |
| CLI.5 | 支持 `search <query> --semantic` 命令 | ✅ Done | P0 | Codex |
| CLI.6 | 默认 JSON 输出，错误也保持机器可读 | ✅ Done | P0 | Codex |
| CLI.7 | 使用 `MEMORYNEXUS_API_URL` 和 `MEMORYNEXUS_TOKEN` 配置 | ✅ Done | P0 | Codex |
| CLI.8 | 补充 CLI smoke test 或命令解析单元测试 | ✅ Done | P1 | Codex |
| CLI.9 | 更新 `docs/cli.md` 快速开始为当前可运行命令 | ✅ Done | P1 | Codex |
| CLI.10 | 支持 `space create/list` 与 memory/search `--space` | ✅ Done | P0 | Codex |

暂不做：

- 本地 token 持久化。
- 交互式配置向导。
- 表格/CSV 输出。
- Lens 命令。等 Lens API 落地后再追加。

---

## 🟣 P3 — 高级功能

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P3.1 | 家庭成员系统 | ⚪ Backlog | P1 | - |
| P3.2 | 定时提醒功能 | ⚪ Backlog | P1 | - |
| P3.3 | Whisper 语音转文字 | ⚪ Backlog | P2 | - |
| P3.4 | 定期回顾报告 | ⚪ Backlog | P2 | - |

---

## 🔵 P4 — 前端

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P4.1 | React 项目初始化 | 🟡 Todo | P0 | - |
| P4.2 | 登录/注册页面 | 🟡 Todo | P0 | - |
| P4.3 | 记忆列表页面 | 🟡 Todo | P1 | - |
| P4.4 | 记忆详情页 | 🟡 Todo | P1 | - |
| P4.5 | 上传组件 | ⚪ Backlog | P1 | - |
| P4.6 | 搜索界面 | ⚪ Backlog | P2 | - |

---

## 📅 开发时间线

```
2026-05 (Week 1-2):  P2 AI 功能 → 智能核心
2026-05 (Week 3-4):  P3 高级功能 → 家庭/提醒
2026-06 (Week 1-2):  P4 前端 → 用户界面
2026-06 (Week 3-4):  集成测试 → v1.0 发布
```

---

## 📊 状态说明

| 状态 | 颜色 | 说明 |
|------|------|------|
| ✅ Done | 🟢 绿 | 已完成 |
| 🟡 Todo | 🟡 黄 | 计划中，待开始 |
| 🔄 In Progress | 🔵 蓝 | 正在进行 |
| ⚪ Backlog | ⚪ 灰 | 备选功能 |

---

## 🔗 相关资源

- 📂 [架构文档](./architecture.md)
- 📖 [API 文档](./api.md)
- 🚀 [部署指南](./deployment.md)
- 💻 [开发指南](./development.md)
- 📝 [决策记录](../decisions/)
