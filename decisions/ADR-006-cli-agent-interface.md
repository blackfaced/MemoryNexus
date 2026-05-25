# ADR-006: CLI + Agent 接口双模式设计

## 状态
✅ 已接受

## 背景
MemoryNexus 不仅服务于人类用户，还需要服务于 AI Agent。AI Agent 需要：
- 机器可读的输出格式（JSON）
- 可编程的交互方式（CLI/API）
- 支持环境变量配置
- 支持管道输入输出

## 决策

### 选择：统一 API + CLI 双接口

> 2026-05-25 implementation note: 当前 Rust-first 主线已经扩展为
> **REST API + CLI + MCP** 三个客户端入口。CLI 面向人类和脚本；
> `memorynexus-mcp` 面向 Claw/Hermes 类本地 Agent，并提供
> `create_space`、`create_lens`、`add_memory`、`get_profile`、
> `search_memories`、`run_lens` 和 `route_agent_context` 等工具。当前可执行
> 命令以 `docs/cli.md` 和 `docs/mcp.md` 为准。

```
┌─────────────────────────────────────────────────────────────┐
│                     MemoryNexus                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   🌐 Web    │     │   💻 CLI    │     │   🤖 API    │   │
│  │    UI      │     │   (Agent)   │     │  (Agent)    │   │
│  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘   │
│         │                   │                   │          │
│         └───────────────────┼───────────────────┘          │
│                             ▼                               │
│                    ┌─────────────────┐                      │
│                    │   Unified API   │                      │
│                    │    (REST)       │                      │
│                    └────────┬────────┘                      │
│                             ▼                               │
│                    ┌─────────────────┐                      │
│                    │  Core Services  │                      │
│                    └─────────────────┘                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### CLI 命令设计

```bash
# 认证与空间
cargo run --bin memorynexus-cli -- auth register --email you@example.com --name You --password secret123
cargo run --bin memorynexus-cli -- space create --name "Project Space"

# 记忆管理
cargo run --bin memorynexus-cli -- memory add --space <space-id> --content "今天学习了 Rust" --tags "学习,rust"
cargo run --bin memorynexus-cli -- memory list --space <space-id> --limit 10
cargo run --bin memorynexus-cli -- memory get <memory-id>

# 搜索
cargo run --bin memorynexus-cli -- search "Rust async" --space <space-id> --semantic --limit 5

# Lens
cargo run --bin memorynexus-cli -- lens create --space <space-id> --template personal_context
cargo run --bin memorynexus-cli -- lens run <lens-id> --query "总结当前上下文"

# 提醒
cargo run --bin memorynexus-cli -- reminder add --space <space-id> --content "团队会议" --at 2026-05-26T09:00:00Z

# 配置
cargo run --bin memorynexus-cli -- config
```

### API 端点设计

```yaml
# REST API v1
GET    /api/v1/memories              # 列出记忆
POST   /api/v1/memories               # 创建记忆
GET    /api/v1/memories/{id}         # 获取单个
PATCH  /api/v1/memories/{id}         # 更新
DELETE /api/v1/memories/{id}         # 删除

GET    /api/v1/search                 # 关键词 / 语义搜索
POST   /api/v1/lenses                 # 创建 Lens
POST   /api/v1/lens-runs             # 运行 Lens
POST   /api/v1/profiles              # 生成 Cognitive Profile
POST   /api/v1/agent/route           # Agent 路由建议
POST   /api/v1/ai/summarize          # AI 摘要

GET    /api/v1/health                 # 健康检查
```

### Agent 特性

| 特性 | 说明 | 示例 |
|------|------|------|
| JSON 输出 | 机器可解析 | `memorynexus-cli memory list \| jq '.data.items[0]'` |
| 管道支持 | 命令组合 | `memorynexus-cli search "x" \| jq '.data.items[0]'` |
| 环境变量 | 配置管理 | `MEMORYNEXUS_API_URL=http://localhost:8080 MEMORYNEXUS_TOKEN=...` |
| MCP 工具 | Agent 直接调用 | `create_space`, `create_lens`, `add_memory`, `get_profile` |
| 静默模式 | 脚本调用 | `cargo run --quiet --bin memorynexus-cli -- ...` |

## 后果

**正面：**
- 同时服务人类和 Agent
- 便于 CI/CD 集成
- 易于调试和自动化
- 支持脚本和管道操作

**负面：**
- API 设计需要考虑两种场景
- 输出格式需要保持一致性

## 相关决策
- ADR-001: Rust 后端选择
