# 🦀 MemoryNexus Backend

> Rust + Axum 后端服务

## 开发

```bash
# 安装依赖
cargo build

# 运行
cargo run

# 测试
cargo test

# 代码检查
cargo fmt
cargo clippy
```

## API

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/health` | GET | 健康检查 |
| `/api/v1/memories` | GET | 列出记忆 |
| `/api/v1/memories` | POST | 创建记忆 |

## 测试

```bash
cargo test --all-features -- --nocapture
```
