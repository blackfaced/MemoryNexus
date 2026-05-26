# ADR-013: Thought Review UI MVP

## 状态
✅ 已接受

## 背景

Phase 4 需要为 MemoryNexus 选择第一版 UI 方向。项目已经确定
Rust-first 后端主线，旧的空前端骨架也已移除，因此第一版 UI 不能重新引入
第二套后端主线。

同时，产品定位已经从直接展示 “personal cognitive substrate” 调整为更清晰的
普通用户入口：

```text
写下一个混乱想法，用不同视角帮用户看清它。
```

如果第一版 UI 直接围绕 `CognitiveSpace`、`Memory`、`Lens Run` 等后端对象建
CRUD 页面，用户会先看到系统模型，而不是产品价值。Phase 4 的第一版 UI 应该先验证
Thought Review 这个 magic moment，再决定是否引入更完整的前端应用栈。

## 决策

Phase 4 的第一版 UI 采用 **Rust-served static Thought Review UI**：

- Rust + Axum 服务继续作为唯一后端和 HTTP 入口。
- 静态 HTML/CSS/JavaScript 由 Rust API 直接在 `/` 和 `/app` 提供。
- 不引入 Node dev server、Next.js、Vite、BFF 或第二套后端。
- UI 通过现有 REST API 调用认证、Space、Memory、Lens、Lens Run 和 Review
  Report 能力。
- 第一版可用工作流固定为 Thought Review：
  1. 用户注册或登录。
  2. 用户写下一条当前最占脑子的想法。
  3. UI 自动创建或复用默认视角：工程视角、侦探视角、叙事视角。
  4. 系统保存原始想法为 `Memory`。
  5. 系统运行多个 `Lens Run`，展示多视角解读。
  6. 用户可以查看最近复盘。
  7. 用户可以生成 Weekly Cognitive Review，看到反复主题、内在张力、正在形成的主线和下一步。

这不是永久否定 React、Vite 或其他前端栈。它是 Phase 4 的最小产品入口选择：
先验证用户 ritual 和信息架构，再决定是否升级到独立前端工程。

## 后果

正面：

- 保持 Rust-first 主线，不增加部署面和双后端维护成本。
- 本地试用路径简单：启动 Rust API 后打开 `http://localhost:8080/`。
- 第一屏直接呈现 Thought Review 场景，而不是后端对象模型。
- 现有 Memory、Lens Run、Review Report provenance 全部复用，不需要新数据库模型。
- 静态页面足够薄，后续可以被 React/Vite 前端替换，而不改变 API 契约。

负面：

- 原生 HTML/JS 不适合长期承载复杂交互、路由、状态管理和组件复用。
- 当前 UI 主要验证产品入口，不是最终设计系统。
- 更完整的 Space 切换、Memory 详情、搜索结果、Lens Run 详情和可访问性增强仍需后续 issue。
- 如果 Phase 4 后续进入多页面复杂应用，需要再记录前端栈升级决策。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-006: CLI + Agent 双模式接口
- ADR-009: Rust-first 后端主线
- ADR-012: Family and Shared Cognitive Space
