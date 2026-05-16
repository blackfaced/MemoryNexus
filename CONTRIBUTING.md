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

### Python (后端)

- 遵循 [PEP 8](https://pep8.org/)
- 使用 `black` 格式化代码
- 使用 `ruff` 进行 linting
- 所有函数添加类型提示
- 写单元测试

```bash
# 格式化
black src/

# Linting
ruff check src/

# 测试
pytest tests/
```

### React/TypeScript (前端)

- 遵循项目已有的代码风格
- 使用有意义的变量命名
- 组件添加 PropTypes 或 TypeScript 类型

## 🧪 测试

所有新功能必须包含测试：

```bash
# 运行所有测试
pytest tests/

# 带覆盖率
pytest tests/ --cov=src --cov-report=html
```

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

- 至少有一个 reviewer 批准才能合并
- 响应 review 反馈
- 保持分支最新

## 📜 许可

通过贡献代码，你同意你的代码将遵循 [MIT License](LICENSE)。

---

有问题？随时开 [Discussion](https://github.com/blackfaced/MemoryNexus/discussions) 讨论！
