# 🤝 贡献指南

感谢你考虑为 MemoryNexus 做贡献！

## 📋 如何贡献

### 1. 报告问题

- 使用 [GitHub Issues](https://github.com/blackfaced/MemoryNexus/issues)
- 选择合适的 Issue 模板
- 提供详细的问题描述和复现步骤

### 2. 提交代码

1. **Fork** 本仓库
2. **Clone** 到本地：
   ```bash
   git clone https://github.com/YOUR_USERNAME/MemoryNexus.git
   cd MemoryNexus
   ```
3. **创建分支**：
   ```bash
   git checkout -b feature/your-feature-name
   # 或
   git checkout -b fix/your-bug-fix
   ```
4. **开发 & 测试**
5. **提交**：
   ```bash
   git commit -m "Add: 简短描述"
   ```
6. **推送**：
   ```bash
   git push origin feature/your-feature-name
   ```
7. **创建 Pull Request**

## 📐 代码规范

### Rust 后端

- MemoryNexus 是 Rust-first 项目，Rust + Axum crate 位于仓库根目录。
- 新增 API、数据库访问、对象存储、向量检索、AI 编排默认落在 Rust 服务。
- Memory 归属于 `CognitiveSpace`，不要重新引入 Agent-owned memory 或第二套后端主线。
- 修改 Rust 行为时，优先补单元测试或端到端验收。

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

### 静态 UI

- 当前 UI 是 Rust 服务直接提供的静态 Thought Review MVP，入口在 `web/thought_review.html`。
- 修改静态 UI 时保持轻量实现，除非 issue 或 ADR 明确要求升级前端栈。
- 不要在没有 ADR 的情况下引入 React/Vite/Next、Node dev server、BFF 或第二套 frontend/backend 主线。

## 🧪 测试

所有新功能必须包含测试。开 PR 前至少运行：

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D clippy::all
```

如果只改 Markdown 文档，可以说明未跑 Rust 测试的原因，并至少运行 `git diff --check`。

## 📝 提交信息规范

使用语义化的提交信息：

```
feat: 新功能
fix: 修复 bug
docs: 文档更新
style: 代码格式（不影响功能）
refactor: 重构
test: 测试相关
chore: 构建/工具更新
```

示例：
- `feat: 添加用户注册功能`
- `fix: 修复记忆检索返回空结果的问题`
- `docs: 更新部署文档`

## 🔍 Code Review

- PR 必须通过 CI 中的 Format、Clippy、Build、Test。
- 响应 review 反馈。
- 保持分支最新。

## 📜 许可

通过贡献代码，你同意你的代码将遵循 [MIT License](LICENSE)。

---

有问题？随时开 [Discussion](https://github.com/blackfaced/MemoryNexus/discussions) 讨论！
