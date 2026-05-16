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
# 记忆管理
memorynexus add --content "今天学习了 Rust" --tags "学习,rust"
memorynexus list --limit 10 --format json
memorynexus get <memory-id>

# 搜索
memorynexus search "Rust async" --top-k 5 --format json

# AI 功能
memorynexus summarize <memory-id>
memorynexus suggest-todos --context "工作"

# 提醒
memorynexus remind "明天 9:00" "团队会议"

# 配置
memorynexus config set API_URL http://localhost:8080
memorynexus config set API_KEY xxx
```

### API 端点设计

```yaml
# REST API v1
GET    /api/v1/memories              # 列出记忆
POST   /api/v1/memories               # 创建记忆
GET    /api/v1/memories/{id}         # 获取单个
PATCH  /api/v1/memories/{id}         # 更新
DELETE /api/v1/memories/{id}         # 删除

POST   /api/v1/search                 # 语义搜索
POST   /api/v1/summarize             # AI 摘要
POST   /api/v1/suggest-todos         # TODO 建议

GET    /api/v1/stats                  # 统计信息
GET    /api/v1/health                 # 健康检查
```

### Agent 特性

| 特性 | 说明 | 示例 |
|------|------|------|
| JSON 输出 | 机器可解析 | `memorynexus list --format json` |
| 管道支持 | 命令组合 | `memorynexus search "x" \| jq '.results[0]'` |
| 环境变量 | 配置管理 | `MEMORYNEXUS_API_KEY=xxx memorynexus ...` |
| 流式响应 | 长任务 | `--stream` 标志 |
| 静默模式 | CI/CD | `--quiet` 标志 |

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
