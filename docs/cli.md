# CLI 命令行工具设计

> MemoryNexus CLI - 为 Agent 和开发者设计的命令行工具

## 设计原则

| 原则 | 说明 |
|------|------|
| **机器优先** | 默认 JSON 输出，便于 Agent 解析 |
| **人类友好** | 添加 `--pretty` 或 `--format table` 便于阅读 |
| **可组合** | 支持管道和重定向 |
| **无状态** | 状态存储在服务端，CLI 只做接口 |

## 安装

```bash
# 通过 cargo 安装
cargo install memorynexus-cli

# 或下载预编译二进制
curl -fsSL https://memorynexus.sh/cli | sh
```

## 快速开始

```bash
# 配置 API
export MEMORYNEXUS_API_URL=http://localhost:8080
export MEMORYNEXUS_API_KEY=your-api-key

# 添加记忆
memorynexus add --content "今天学习了 Rust async/await" --tags "rust,学习"

# 搜索
memorynexus search "Rust 异步编程"

# 列出最近记忆
memorynexus list --limit 10
```

## 命令参考

### 🏠 记忆管理

```bash
# 添加记忆
memorynexus add [OPTIONS]
  --content <TEXT>          # 记忆内容
  --title <TEXT>            # 可选标题
  --tags <TAGS>             # 逗号分隔标签
  --type <TYPE>             # text|image|audio|video
  --shared                   # 家庭共享

# 列出记忆
memorynexus list [OPTIONS]
  --limit <N>               # 数量限制 (默认 20)
  --offset <N>              # 偏移量
  --tag <TAG>               # 按标签筛选
  --format <FORMAT>          # json|table|csv (默认 json)
  --pretty                   # 格式化输出

# 查看记忆
memorynexus get <MEMORY_ID>

# 更新记忆
memorynexus update <MEMORY_ID> [OPTIONS]
  --content <TEXT>          # 新内容
  --title <TEXT>            # 新标题
  --tags <TAGS>             # 新标签

# 删除记忆
memorynexus delete <MEMORY_ID>
```

### 🔍 搜索

```bash
memorynexus search <QUERY> [OPTIONS]
  --top-k <N>               # 返回数量 (默认 5)
  --format <FORMAT>          # json|table
  --stream                   # 流式输出
```

### 🤖 AI 功能

```bash
# 摘要
memorynexus summarize <MEMORY_ID>

# AI 建议 TODO
memorynexus suggest-todos [OPTIONS]
  --context <TEXT>           # 上下文
  --limit <N>                # 建议数量

# AI 洞察
memorynexus insights [OPTIONS]
  --period <PERIOD>          # 时间范围 (7d, 30d)
```

### 🔔 提醒

```bash
# 创建提醒
memorynexus remind <TIME> <CONTENT>
  # TIME 格式: "tomorrow 9:00", "2024-01-01 12:00", "+2h"

# 列出提醒
memorynexus reminders

# 删除提醒
memorynexus reminder delete <REMINDER_ID>
```

### 👨‍👩‍👧 家庭

```bash
memorynexus family [OPTIONS]
  invite <EMAIL>          # 邀请成员
  members                    # 列出成员
  leave                      # 离开家庭
```

### ⚙️ 配置

```bash
memorynexus config [OPTIONS]
  set <KEY> <VALUE>         # 设置配置
  get <KEY>                  # 获取配置
  list                       # 列出所有配置
  init                       # 交互式初始化
```

## 输出格式

### JSON (默认，Agent 友好)

```json
{
  "ok": true,
  "data": {
    "id": "mem_xxx",
    "content": "今天学习了 Rust",
    "tags": ["学习", "rust"],
    "created_at": "2024-01-01T12:00:00Z"
  }
}
```

### Table (人类可读)

```
┌──────────────────────────────────────┬──────────┬────────────────────┐
│ ID                                   │ Tags     │ Created            │
├──────────────────────────────────────┼──────────┼────────────────────┤
│ mem_xxx                              │ 学习,rust│ 2024-01-01 12:00   │
│ mem_yyy                              │ 工作     │ 2024-01-02 09:30   │
└──────────────────────────────────────┴──────────┴────────────────────┘
```

### CSV (导出)

```bash
memorynexus list --format csv > memories.csv
```

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `MEMORYNEXUS_API_URL` | API 地址 | http://localhost:8080 |
| `MEMORYNEXUS_API_KEY` | API 密钥 | - |
| `MEMORYNEXUS_FORMAT` | 默认输出格式 | json |
| `MEMORYNEXUS_TIMEOUT` | 请求超时(秒) | 30 |

## Agent 使用示例

### 作为 AI Agent 的记忆工具

```python
# Python Agent 集成示例
import subprocess
import json

def add_memory(content: str, tags: list):
    result = subprocess.run([
        "memorynexus", "add",
        "--content", content,
        "--tags", ",".join(tags),
        "--format", "json"
    ], capture_output=True, text=True)
    return json.loads(result.stdout)

def search_memories(query: str):
    result = subprocess.run([
        "memorynexus", "search", query,
        "--format", "json"
    ], capture_output=True, text=True)
    data = json.loads(result.stdout)
    return data["data"]["results"]
```

### Shell 脚本集成

```bash
#!/bin/bash
# 自动记录 git commit 到记忆
git commit -m "$1" && \
memorynexus add --content "完成提交: $1" --tags "git,commit" --format json
```

### CI/CD 集成

```yaml
# .github/workflows/daily.yml
- name: AI Code Review Summary
  run: |
    memorynexus suggest-todos --context "今天的代码审查" --format json
```

## 错误处理

```json
{
  "ok": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Memory not found",
    "details": {
      "id": "mem_xxx"
    }
  }
}
```

错误码：

| 代码 | 说明 |
|------|------|
| `UNAUTHORIZED` | 未认证或 API Key 无效 |
| `NOT_FOUND` | 资源不存在 |
| `VALIDATION_ERROR` | 输入参数错误 |
| `RATE_LIMITED` | 请求过于频繁 |
| `SERVER_ERROR` | 服务器内部错误 |

## Shell 自动补全

```bash
# Bash
memorynexus completion bash > /etc/bash_completion.d/memorynexus

# Zsh
memorynexus completion zsh > "${fpath[1]}/_memorynexus"

# Fish
memorynexus completion fish > ~/.config/fish/completions/memorynexus.fish
```

## 项目结构

```
memorynexus-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口
│   ├── commands/            # 命令模块
│   │   ├── mod.rs
│   │   ├── add.rs
│   │   ├── list.rs
│   │   ├── search.rs
│   │   ├── summarize.rs
│   │   └── ...
│   ├── api/                 # API 客户端
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── types.rs
│   ├── config/              # 配置管理
│   └── output/              # 输出格式化
│       ├── mod.rs
│       ├── json.rs
│       ├── table.rs
│       └── csv.rs
└── tests/
```
