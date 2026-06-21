# ADR-021: External Media Evidence References

## 状态

✅ 已接受

## 背景

Dictation Coach 和其他上游产品可能从图片、音频或视频开始交互。OCR、ASR、媒体采集和
用户确认依赖具体设备、Agent 能力、产品体验与 provider，不应进入 MemoryNexus 的长期
反馈 Engine。Engine 需要的是稳定、可确认的文字输入，同时保留回看原始来源的
provenance。

ADR-002 已定义 MemoryNexus 托管对象的存储抽象，但外部媒体可能位于本地文件、移动
硬盘、WebDAV、S3、OSS 或其他系统。要求全部复制到 S3/MinIO 会把可选的 provenance
变成反馈主链的硬依赖，也会混淆托管字节与引用外部证据两种责任。

## 决策

- Agent / App 负责 OCR、ASR、媒体获取和用户确认。
- MemoryNexus 的反馈主链只依赖用户确认后的 normalized text。
- 原始媒体通过 provider-neutral `EvidenceRef` 关联到 Trace / Surface provenance；
  `EvidenceRef` 不表示 MemoryNexus 拥有或已经读取媒体字节。
- `EvidenceResolver` 是可选的 integration abstraction。第一版只登记引用，不要求
  MemoryNexus 解析或读取媒体。
- S3、OSS、WebDAV、移动硬盘、本地文件和未来的 MemoryNexus managed storage 都是
  provider，不是 Engine 核心。
- `CognitiveSpace` 仍是 ownership / permission boundary；Namespace 仍只是 Space 内的
  领域分区，不是媒体权限边界。
- 未来独立的 Dictation Coach 仓库只通过 Surface Gateway / MCP 使用 MemoryNexus，
  不直接访问 Engine 内部表或对象。

字段、校验、resolver 能力和错误码的规范以
[Media Evidence Contract](../docs/media-evidence-contract.md) 为准。

## 处理流程

```text
image / audio / video
  -> Agent or App OCR / ASR
  -> user-confirmed normalized text
  -> Surface Gateway(text + optional EvidenceRefInput)
  -> Trace / FeedbackLoop / GrowthModel / PracticePlan
```

确认后的 Surface text 是反馈、归因和计划的 canonical input。`EvidenceRef` 中可选的
transcript 只用于 provenance，不能覆盖已确认文字。

## 失败行为

- 媒体不可访问、provider 暂时不可用或 resolver 失败时，只影响原始证据检查。
- 不可访问的媒体不得使已确认文字、既有 Trace 或已完成反馈失效，也不得阻断后续的
  text-based feedback loop。
- 内容 hash 不匹配时，不得把解析出的文件呈现为原始证据；已完成的文字反馈仍然保留。
- 权限不足时不得泄露 locator 或 provider credentials。
- 无效引用应在 Gateway 边界被拒绝，不得让 Engine 猜测或补造媒体内容。

## 安全后果

- Surface Gateway 从已授权上下文绑定 `CognitiveSpace`；调用方不能通过引用声明所有权。
- locator 和 metadata 不得保存 credentials、API keys、mount secrets 或短期 signed URL
  query parameters。
- resolver 必须在授权后工作，且不能绕过 Space membership 或把 provider 权限提升为
  Engine 权限。resolver 还必须将访问限制在已配置的 provider roots / buckets / hosts，
  不能成为任意本地文件或 URL 读取入口。
- OCR / ASR 的不确定性由 Agent / App 与用户在提交前解决；MemoryNexus 不从不可检查的
  引用中推断文字。

## 后果

正面：

- 反馈主链保持 text-first、local-first，并与媒体 provider 的可用性解耦。
- 原始媒体仍可保留跨 provider 的可追溯性。
- Dictation Coach 与其他 Adapter 可以选择适合自身环境的 OCR、ASR 和媒体管理方案。
- MemoryNexus managed storage 未来仍可作为一种 provider 演进。

负面：

- 外部引用可能移动、过期或暂时不可用，provenance inspection 不能保证始终成功。
- provider-specific resolution 需要后续独立 integration 实现和授权策略。
- `evidence_mismatch` 只表示解析出的内容无法通过 hash 或稳定身份验证为原始证据。
  transcript 与确认后的 Surface text 不同属于 Adapter / 用户确认 provenance，不是
  `evidence_mismatch`，也不要求 Engine 分析媒体或修正既有反馈。

## 非目标

- 不在第一版实现 `EvidenceRef` schema、repository 或 `EvidenceResolver` 执行。
- 不上传、下载、移动、复制或删除媒体。
- 不为 Engine 增加 OCR、ASR、图片理解或视频理解。
- 不选择唯一媒体 provider，也不要求外部媒体复制到 S3/MinIO。
- 不创建独立 Dictation Coach 仓库、产品 UI 或产品角色模型。
- 不把 Namespace 改造成所有权或权限边界。

本 ADR 和 Media Evidence Contract 只建立文档与未来验证契约，不引入当前可调用的
Surface、持久化、resolver 或媒体处理能力。

## 相关决策

- [ADR-002: 存储层抽象设计](ADR-002-storage-abstraction.md)
- [ADR-019: Surfaces vs Adapters vs Engine](ADR-019-surfaces-adapters-engine.md)
- [ADR-020: Dictation Coach as First Upstream Product](ADR-020-dictation-coach-first-upstream-product.md)
