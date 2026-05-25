# ADR-012: Family and Shared Cognitive Space

## 状态
✅ 已接受

## 背景

Phase 3 需要从个人项目记忆扩展到家庭和共享认知场景。项目主线仍然要求：

- Memory 属于 `CognitiveSpace`，不属于 Agent。
- Agent、用户和未来 UI 都只是进入某个 Space 的参与者。
- 共享能力不能把 Memory ownership 重新放回单个用户或 Agent。

## 决策

引入共享 `CognitiveSpace` 成员模型：

- `CognitiveSpace` 是 Memory、Lens、Lens Run 和后续认知对象的归属边界。
- `cognitive_space_members` 表示用户进入 Space 的成员关系。
- 成员角色采用最小集合：
  - `owner`: Space 所有者，可以管理成员、邀请码和写入内容。
  - `editor`: 可以在 Space 内写入和修改自己创建的 Memory。
  - `viewer`: 可以读取 Space 内可见内容，但不能写入。
- 邀请码是加入共享 Space 的第一版机制。
  - 邀请码属于 Space。
  - 邀请码授予 `editor` 或 `viewer`，不能授予 `owner`。
  - 接受邀请码后创建或更新成员关系。
- Memory 仍然保存 `user_id` 作为创建者 provenance，但 ownership 归 `space_id`。
- Memory 更新和删除默认只允许创建者或 Space owner；editor 不能修改他人 Memory。

## 后果

正面：

- 个人、家庭、项目和组织都可以复用同一套 Cognitive Space 边界。
- 后续 Reminder、Profile、CognitiveState、Lens Review 都可以绑定 Space 成员关系。
- 邀请码机制足够简单，适合 CLI 和早期 MVP。

负面：

- 第一版角色模型刻意保守，暂不支持细粒度 ACL。
- `is_shared` 作为早期字段会继续存在，但共享边界以 Space membership 为准。
- 更复杂的家庭身份、儿童账号、监护关系需要后续 ADR 或 issue 承接。

## 相关决策

- ADR-002: Cognitive Lens Memory 产品方向
- ADR-009: Rust-first 后端主线
- ADR-010: Memory salience 与 automatic forgetting
