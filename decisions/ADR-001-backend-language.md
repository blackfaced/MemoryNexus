# ADR-001: 使用 Rust 作为后端语言

## 状态
✅ 已接受

## 背景
需要为 MemoryNexus 选择后端技术栈。最初考虑 Python (FastAPI)，但考虑到性能、稳定性和现代开发体验，决定重新评估。

## 决策

### 选择：Rust + Axum

**原因：**

| 因素 | Rust | Python |
|------|------|--------|
| 性能 | ✅ 极高 | ⚠️ 一般 |
| 类型安全 | ✅ 编译期保证 | ⚠️ 运行时检查 |
| 并发 | ✅ 原生 async | ⚠️ 需 asyncio |
| 部署体积 | ✅ 小 (~10MB) | ⚠️ 大 (依赖多) |
| 学习曲线 | ⚠️ 较陡 | ✅ 平缓 |

**Rust 优势：**
- 编译时消除 null 指针和数据竞争
- 内存安全，无需 GC
- 异步性能优秀（Tokio）
- 生态成熟（Axum, SQLx, Serde）

**核心依赖：**
```toml
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres"] }
serde = { version = "1", features = ["derive"] }
```

## 后果

**正面：**
- 高性能 API 服务
- 更好的类型安全
- 更小的容器镜像
- 团队技术成长

**负面：**
- 学习曲线较陡
- 编译时间较长
- AI/ML 库生态不如 Python

## 相关决策
- ADR-002: 存储层抽象设计
- ADR-003: 向量数据库选择
- ADR-004: Whisper 部署方案
