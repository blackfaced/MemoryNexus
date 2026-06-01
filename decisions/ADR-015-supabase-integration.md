# ADR-015: Supabase Integration Boundary

## 状态
✅ 已接受

## 背景

MemoryNexus 当前主线是 Rust-first Axum backend，核心数据模型存储在 PostgreSQL，
语义检索使用 Qdrant，对象存储通过 S3 / MinIO-compatible abstraction 接入。

Supabase 提供托管 PostgreSQL、Auth、Storage、Realtime、Edge Functions 等能力。
它可以降低部署和运维成本，但如果把 Supabase 当成新的 backend 主线，会和现有决策冲突：

- ADR-009 已确认 Rust + Axum 是唯一继续演进的主后端。
- `CognitiveSpace` 是 ownership / permission boundary。
- MemoryNexus 的认知模型、Lens Run、Namespace、FeedbackLoop 和未来 memory lifecycle
  都应该继续由 Rust 服务控制。

因此需要明确 Supabase 的接入边界：它可以是托管基础设施和可选 adapter，但不应成为
MemoryNexus 的产品/后端架构替代品。

## 决策

MemoryNexus 对 Supabase 的接入顺序和边界如下。

### 第一阶段：Supabase Postgres Compatibility

Supabase 首先作为托管 PostgreSQL 兼容目标：

```text
MemoryNexus Rust API
→ SQLx
→ Supabase Postgres
```

这一阶段只要求：

- `DATABASE_URL` 可以指向 Supabase Postgres。
- 现有 SQLx migrations 可以在 Supabase Postgres 上执行。
- Rust API、CLI、MCP 和 Thought Review UI 继续通过 MemoryNexus 后端访问数据。
- `CognitiveSpace` membership 仍然由 MemoryNexus 表和 Rust 权限检查控制。
- Qdrant 继续作为向量检索后端，除非未来有单独 ADR 改变。

注意事项：

- Supabase transaction pooler 不适合作为默认 SQLx 连接方式，除非确认关闭 prepared
  statement cache 或使用兼容配置。
- 长驻 Rust 服务优先使用 direct connection 或 session pooler；serverless / edge
  场景才优先考虑 transaction pooler。
- 必须使用 SSL 连接。

### 第二阶段：Supabase Auth Adapter

Supabase Auth 可以作为后续可选身份源，但必须单独设计。

候选模式：

```text
Frontend signs in with Supabase Auth
→ sends Supabase JWT to MemoryNexus Rust API
→ Rust verifies JWT
→ maps Supabase user id to local users row
→ MemoryNexus checks CognitiveSpace membership
```

边界：

- Supabase Auth 可以负责登录、OAuth、session issuing。
- MemoryNexus 仍负责 `User` 映射、`CognitiveSpace` membership、roles、invites 和业务权限。
- 不允许让 Supabase Row Level Security 取代 MemoryNexus 的 Rust 权限边界。
- 如果使用 Supabase JWT，必须通过 JWKS / 高质量 JWT verification library 或明确的
  Auth server 验证路径处理，不在前端暴露 service role / shared secret。

### 第三阶段：Supabase Storage / Realtime Adapter

Supabase Storage 和 Realtime 只作为后续可选 adapter：

- Storage 可以作为新的 object storage provider，但不能移除现有 S3-compatible
  abstraction。
- Realtime 可以用于 UI 刷新、review status、background job progress 等产品体验，
  但不进入认知核心模型。
- Edge Functions 不作为 MemoryNexus 的主后端；需要后台任务时优先在 Rust 服务或
  明确的 worker 方案中实现。

## 非目标

- 不把 Supabase 变成第二套 backend 主线。
- 不用 Supabase REST / PostgREST 绕过 Rust API 直接操作核心表。
- 不把 RLS 作为 MemoryNexus 的主要权限模型。
- 不在第一阶段迁移现有 auth。
- 不在第一阶段迁移对象存储。
- 不把 Qdrant 替换为 Supabase / Postgres vector 能力，除非未来有独立 ADR。

## 后果

正面：

- Supabase 可以作为低运维成本的托管 PostgreSQL 部署选项。
- 现有 Rust-first 架构、SQLx repositories、migrations 和权限模型可以保持稳定。
- Auth、Storage、Realtime 可以按 adapter 逐步评估，不会一次性重构后端。
- 对个人开发者更友好：先解决部署数据库问题，再决定是否引入托管身份能力。

负面：

- 需要测试 SQLx 与 Supabase pooler / prepared statement 的兼容性。
- 如果未来接入 Supabase Auth，需要维护 external identity 到 local user 的映射。
- 如果同时使用 Supabase Auth 和 MemoryNexus 自有 JWT，需要明确 token 信任边界，避免
  双认证模型混乱。
- Supabase RLS 与 Rust 权限检查并存时，必须避免权限规则分散和难以审计。

## 后续任务

第一批后续任务应聚焦 Supabase Postgres compatibility：

- 增加 Supabase Postgres deployment note。
- 验证 migrations 可以在 Supabase Postgres 上执行。
- 明确 SQLx pool 配置：direct / session pooler / transaction pooler 的推荐用法。
- 增加一个最小 smoke checklist：注册、登录、创建 Space、创建 Memory、搜索、Lens Run。

Supabase Auth / Storage / Realtime 必须通过独立 issue 和 ADR 补充推进。

## 相关决策

- ADR-008: 数据库设计
- ADR-009: Rust-first 后端主线
- ADR-012: 家庭与共享 Cognitive Space
- ADR-013: Thought Review UI MVP
- ADR-014: Namespace and Feedback Loop Model
