# AGENTS.md

本文件给 Codex 和其他代码 Agent 使用。进入仓库后请先阅读本文件，再开始修改。

## 项目主线

- MemoryNexus 是家庭 AI 记忆中心，后端主线是 **Rust-first**。
- Rust + Axum 服务位于 `src/` crate 中，是唯一继续演进的主后端。
- Python/FastAPI 代码若仍存在，只作为历史兼容层或实验参考，不承载新功能主路径。

## 架构决策

- 重要决策必须写入 `decisions/`，使用 ADR 形式：`ADR-00X-short-title.md`。
- 新增 ADR 后必须更新 `decisions/README.md`。
- 不要把长期架构决策只写在 `docs/` 或对话总结里。
- 当前 Rust-first 主线见 `decisions/ADR-009-rust-first-backend.md`。

## 开发规则

- 新增 API、数据库访问、对象存储、向量检索、AI 编排默认落在 Rust 服务。
- 修改 Rust 行为时优先补单元测试或端到端验收；至少运行 `cargo test`。
- 本地没有 Rust 工具链时，可以用 Docker 验证：

```bash
docker run --rm \
  -v "$PWD/src:/workspace" \
  -w /workspace \
  rust:1.75 cargo test
```

- 不要把生成的 `*.profraw`、`target/`、临时测试产物提交进仓库。
- 不要回退用户已有改动；遇到无关脏文件时保持原样。

## P0 优先级

1. Embedding -> Qdrant -> Rust search API 的语义检索闭环。
2. 注册登录、创建记忆、搜索召回、摘要生成的端到端验收。
3. 文档口径统一：README、architecture、roadmap、TODO 都应以 Rust 主线为准。

## 文档位置

- `README.md`：面向使用者的项目入口和快速开始。
- `docs/`：API、开发、部署、路线图等说明。
- `decisions/`：架构决策记录，所有长期决策放这里。
- `src/`：Rust 后端 crate。
