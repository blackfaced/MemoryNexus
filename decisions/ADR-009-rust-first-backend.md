# ADR-009: Rust-first 后端主线

## 状态
✅ 已接受

## 背景

MemoryNexus 曾经同时保留 Rust + Axum 与 Python/FastAPI 两套后端实现痕迹。随着 Rust 侧已经形成认证、记忆 CRUD、标签、搜索、AI 接口、存储抽象等核心骨架，继续双轨推进会带来以下问题：

- README、architecture、roadmap、TODO 等文档容易出现口径漂移。
- 同一能力在 Rust 与 Python 两侧重复设计，增加测试和部署成本。
- P0 阶段真正需要的是打通语义检索和 AI 生成闭环，而不是维护两条主路径。

## 决策

MemoryNexus 后端主线确定为 **Rust-first**。

Rust 服务作为唯一主后端继续演进，承载认证、记忆管理、标签、搜索、AI 编排、上传与媒体管理等用户可见能力。Python/FastAPI 相关实现若仍保留，只作为历史兼容层、实验代码或迁移参考，不再新增业务能力。

## 执行规则

- 新增 API、数据库访问、对象存储、向量检索和 AI 编排默认落在 Rust 服务。
- Python 代码不得再作为新功能入口；确需保留时，必须在文档中标注为 compatibility 或 experiment。
- README、architecture、roadmap、TODO 等文档必须以 Rust 主线为准。
- P0 验收用例优先覆盖 Rust API，而不是 Python 路由。
- 影响架构、技术选型、长期接口契约的决策必须通过 `decisions/ADR-00X-*.md` 记录。

## 后果

**正面：**
- 后端演进路线更清晰，减少双轨维护成本。
- P0 可以聚焦 Embedding、Qdrant、语义搜索、端到端验收。
- Rust 类型系统和编译检查可以更早暴露接口、状态和模块边界问题。

**负面：**
- Python 侧已有代码需要冻结、迁移或删除，短期会产生清理成本。
- AI/ML 快速实验仍可能需要 Python，但不能再混入主 API 路径。
- Rust 主线要求本地开发环境必须能稳定运行 `cargo test`。

## P0 接续任务

1. 打通语义检索最小闭环：Embedding 生成 -> Qdrant upsert -> 查询召回 -> Rust API 返回。
2. 补齐端到端验收：注册登录、创建记忆、搜索召回、摘要生成。
3. 清理或冻结 Python/FastAPI 文档表述，避免“FastAPI 为主”的误导。

## 相关决策

- ADR-001: 使用 Rust 作为后端语言
- ADR-003: Qdrant 向量数据库
- ADR-007: 开发方法论
- ADR-008: 数据库设计
