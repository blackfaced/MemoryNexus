# ADR-007: 开发方法论

## 状态
✅ 已接受

## 背景
需要为 MemoryNexus 项目确定开发流程和方法论，确保开发效率、代码质量和团队协作。

## 决策

### 选择：Ralph Loop / Superpower + TDD + GitHub 验证

### 1. Ralph Loop / Superpower 框架

> Ralph Loop 是一种迭代式开发框架，强调快速反馈和持续改进。

**核心循环：**
```
Plan → Do → Check → Adjust
 │      │      │       │
 ▼      ▼      ▼       ▼
需求   实现   验证    优化
```

**Superpower 集成：**
- 每次迭代聚焦一个"超能力"（功能）
- 小步快跑，快速交付价值

### 2. TDD 测试驱动开发

**红绿重构循环：**
```
Red    → Green    → Refactor
编写失败测试 → 写最小代码通过 → 优化代码
```

**测试金字塔：**
```
        /\
       /  \      E2E 测试 (少量)
      /----\
     /      \    集成测试 (适量)
    /--------\
   /          \  单元测试 (大量)
  /____________\
```

### 3. GitHub 自动化验证

| 验证项 | 工具 | 触发时机 |
|--------|------|----------|
| 编译检查 | `cargo check` | PR/Commit |
| 单元测试 | `cargo test` | PR/Commit |
| 代码格式 | `cargo fmt` | PR/Commit |
| 安全扫描 | `cargo audit` | 每日/PR |
| 构建检查 | `cargo build` | PR |
| 文档生成 | `cargo doc` | PR |

### 4. 函数式编程风格

**核心原则：**
- 优先使用不可变数据
- 纯函数，无副作用
- 组合优于继承
- Option/Result 替代 null/异常

**Rust 实现：**
```rust
// 偏好组合
let result = input
    .transform()
    .filter()
    .map()
    .unwrap_or(default);

// 而不是
let mut result = Vec::new();
for item in input {
    if condition(&item) {
        result.push(transform(item));
    }
}
```

## 实施流程

### P0 开发流程 (TDD)

```
1. Write Test     → 创建功能测试 (RED)
2. cargo test     → 验证测试失败 (RED)
3. Write Code     → 实现最小代码 (GREEN)
4. cargo test     → 验证测试通过 (GREEN)
5. Refactor       → 优化代码 (REFACTOR)
6. cargo test     → 确保仍然通过
7. Git commit     → 提交代码
8. GitHub Action  → 自动化验证
```

### Git 工作流

```
feature/xxx
    ↓ (PR)
main ←────────── PR Review + CI Pass
    │
    └── GitHub Actions:
        ├── cargo check
        ├── cargo test
        ├── cargo fmt --check
        ├── cargo clippy
        └── cargo doc --no-deps
```

## 后果

**正面：**
- 快速反馈，及早发现问题
- 高测试覆盖率，信心保障
- 自动化验证，减少人工检查
- 代码质量稳定可维护

**负面：**
- 学习曲线较陡
- 初期开发速度较慢
- 需要持续维护测试

## 相关决策
- ADR-001: Rust 后端选择
