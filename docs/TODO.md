# 📋 MemoryNexus 开发计划

> 最后更新: 2026-05-21
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
| P1C.1 | Space-scoped CLI 端到端验收 | P0 | 2026-05-20 | - |
| P2A.1 | Lens 最小模型与 CLI 入口 | P0 | 2026-05-20 | - |
| P2B.1 | Lens Run 同步执行闭环 | P0 | 2026-05-21 | - |

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
| P1B.7 | 注册、space、memory、semantic search 的端到端自动化验收 | ✅ Done | P0 | Codex |

验收入口：

```bash
docker compose up -d postgres qdrant
MEMORYNEXUS_ACCEPTANCE=1 \
QDRANT_URL=http://localhost:6333 \
MEMORYNEXUS_EMBEDDING_PROVIDER=local \
cargo test --test phase1c_acceptance -- --ignored --nocapture
```

下一步 P1C：

- 摘要生成路径的端到端验收。
- 将验收环境隔离到独立 test database。
- 评估是否在 CI 中增加可选 service-based acceptance job。

---

## 🔴 P2A — Lens 最小模型

目标：先把 Lens 从理论概念变成可创建、可列出、可审计的空间级解释策略配置。

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P2A.1 | 新增 `lenses` 和 `lens_runs` 迁移 | ✅ Done | P0 | Codex |
| P2A.2 | 新增 Lens repository，所有访问经 Cognitive Space membership 校验 | ✅ Done | P0 | Codex |
| P2A.3 | 新增 Lens REST API：create/list/get | ✅ Done | P0 | Codex |
| P2A.4 | CLI 支持 `lens create/list/get` | ✅ Done | P0 | Codex |
| P2A.5 | acceptance 覆盖 Lens create/list/get | ✅ Done | P0 | Codex |
| P2A.6 | Lens Run execution API | ✅ Done | P0 | Codex |
| P2A.7 | search/summarize 支持 `lens_id` | 🟡 Todo | P0 | - |
| P2A.8 | 内置 Lens 模板：项目上下文、学习复盘、家庭成长、风险审查 | ✅ Done | P1 | Codex |
| P2A.9 | Lens Run provenance 输出：query、命中 memory、策略、生成时间、输出 | ✅ Done | P1 | Codex |
| P2A.10 | CLI 支持 `lens run` 和 `lens run get` | ✅ Done | P0 | Codex |
| P2A.11 | Lens Run 接入 AI summary provider，并保留 deterministic fallback | ✅ Done | P0 | Codex |
| P2A.12 | Lens Run summary 支持 OpenAI-compatible base URL、provider/model/长度配置 | ✅ Done | P0 | Codex |
| P2A.13 | 仅设置 `OPENROUTER_API_KEY` 时自动推断 OpenRouter summary provider | ✅ Done | P0 | Codex |
| P2A.14 | 补充 Lens Run provider smoke troubleshooting 与 `SPACE_ID` 说明 | ✅ Done | P0 | Codex |
| P2A.15 | Lens Run 输出补充 key points、open questions、next actions、citations | ✅ Done | P0 | Codex |
| P2A.16 | 增加 ignored OpenRouter Lens Run acceptance test | ✅ Done | P1 | Codex |
| P2A.17 | 支持 Lens Run history/list by Lens 或 Space | ✅ Done | P1 | Codex |
| P2A.18 | CLI/API 暴露运行时 AI config，便于排查 provider/model/env | ✅ Done | P1 | Codex |

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
| CLI.11 | 支持 `lens create/list/get` | ✅ Done | P0 | Codex |
| CLI.12 | 支持 `lens run` 和 `lens run get` | ✅ Done | P0 | Codex |

暂不做：

- 本地 token 持久化。
- 交互式配置向导。
- 表格/CSV 输出。
- 交互式 Lens Run 向导。

---

## 🟣 P3 — 高级功能

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P3.1 | 家庭成员系统 | ⚪ Backlog | P1 | - |
| P3.2 | 定时提醒功能 | ⚪ Backlog | P1 | - |
| P3.3 | Whisper 语音转文字 | ⚪ Backlog | P2 | - |
| P3.4 | 定期回顾报告 | ⚪ Backlog | P2 | - |

---

## 🔵 P4 — 用户界面

| ID | 任务 | 状态 | 优先级 | 负责人 |
|----|------|------|--------|--------|
| P4.1 | 选择 UI 技术栈和交互范围 | 🟡 Todo | P1 | - |
| P4.2 | 登录/注册页面 | 🟡 Todo | P1 | - |
| P4.3 | Cognitive Space 列表和切换 | 🟡 Todo | P1 | - |
| P4.4 | Memory 创建、列表、详情 | 🟡 Todo | P1 | - |
| P4.5 | 语义搜索界面 | ⚪ Backlog | P2 | - |
| P4.6 | Lens 运行结果界面 | ⚪ Backlog | P2 | - |

---

## 📅 开发时间线

```
2026-05 (Week 1-2):  P2 AI 功能 → 智能核心
2026-05 (Week 3-4):  P3 高级功能 → 家庭/提醒
2026-06 (Week 1-2):  P4 用户界面
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
