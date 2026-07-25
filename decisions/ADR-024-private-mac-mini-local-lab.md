# ADR-024: Private Mac mini Local Lab

## 状态

✅ 已接受

## 背景

MemoryNexus 需要一条适合个人长期使用的 Apple Silicon Mac mini 部署路径。它应当使用
官方 release 二进制，在本机运行 Rust API、PostgreSQL 和 Qdrant，不要求安装 Rust
工具链，也不能为了方便而绕过 Rust API、`CognitiveSpace` 权限边界或
Surface / Adapter / Engine 分层。

Local One-click bundle 已经提供 API、CLI、MCP 二进制和 Docker Compose runtime，
但私有 Local Lab 还需要明确网络默认值、数据所有权、媒体边界，以及 PostgreSQL 与
Qdrant 的成对备份、恢复和回退规则。

## 决策

### 运行与网络边界

- Mac mini Local Lab 使用匹配机器架构的官方 release bundle；Apple Silicon 对应
  `aarch64-apple-darwin`。正常安装和运行不需要 Cargo 或 Rust 工具链。
- `memorynexus` API 在宿主机运行；Docker Compose 只运行 PostgreSQL 和 Qdrant。
- Local One-click 默认把 API、PostgreSQL HTTP/TCP 和 Qdrant HTTP/gRPC 端口绑定到
  `127.0.0.1`。暴露到局域网、VPN 或公网必须作为单独的、用户明确批准的部署变更，
  并补充认证、TLS 和网络访问控制。
- MCP、CLI、Chat Agent 或微信机器人都是 Adapter。它们必须通过 MCP / Rust API
  访问 MemoryNexus，不能直接读写 PostgreSQL、Qdrant 或 Engine 内部对象。
- Memory 仍然归属于用户控制的 `CognitiveSpace`；Mac mini、MCP client 或机器人不成为
  新的 ownership boundary。

### 数据与配置边界

- 业务记录、权限、`CognitiveSpace`、Namespace、Memory、Trace 和 practice 数据存放在
  PostgreSQL named volume。
- 向量索引存放在 Qdrant named volume。PostgreSQL 与 Qdrant 是同一份逻辑状态的两个
  部分，不能把其中一个的备份声称为完整备份。
- Docker Desktop 和 Colima 在 macOS 上通过 Linux VM 运行容器；named volume 的物理
  数据通常位于 VM / Docker volume 内，不承诺是 Finder 可直接管理的普通目录。
- release 二进制默认安装到 `~/.local/bin`。MCP client 配置位于具体 Adapter 的配置目录；
  bundle 的 `install.sh --mcp-config` 输出必须设置为 `0600`，因为其中可能包含 token。
- token、JWT secret、MCP 配置和备份不得提交到仓库。

### 外部媒体边界

- 微信照片、OCR/ASR 原文和原始媒体留在 Agent / App Adapter。
- MemoryNexus 只接收用户明确接受或修正后的 normalized text。第一版不把原始照片、完整
  OCR 结果或 provider credential 写入 Memory、Trace 或其他 Engine 持久化对象。
- 该边界遵循 ADR-021；媒体接收成功不能被描述成 MemoryNexus 已经保存了照片。

### 备份、升级与回退

- 备份前停止 API 写入和所有写入型 Adapter，然后在同一维护窗口生成 PostgreSQL dump
  与 Qdrant collection snapshot；两份文件、release 版本和 collection 名称共同组成一个
  备份集。
- 每个备份集都要校验 checksum，并定期恢复到隔离数据库和隔离 Qdrant collection，使用
  同一 release API 做 application-level smoke。
- 升级前必须先生成并验证成对备份。新二进制启动时会运行 SQLx migrations。
- 仅回退 `memorynexus` 二进制不是可靠 rollback：新 schema 可能与旧二进制不兼容。
  回退必须停止写入、恢复升级前的 PostgreSQL 与 Qdrant 备份对，再启动对应旧 release。
- Compose project name 会参与 named volume 的实际名称。给现有安装直接加入固定 project
  name 或重命名 bundle 目录可能让 Docker 选择一组新 volume，并让旧数据看起来“消失”。
  本 ADR 不修改现有 project / volume identity；现有安装要先发现并保持当前 identity，任何
  标准化都必须通过独立迁移和成对 restore 验证。

具体命令和恢复演练见
[Mac mini Local Lab Runbook](../docs/mac-mini-local-lab.md)。

## 后果

正面：

- 默认网络面只在本机可见，适合 private self-use。
- release 二进制路径可以在没有 Rust 工具链的 Mac mini 上重复部署。
- 数据位置、媒体责任和完整 rollback 的定义清晰，减少“只备份数据库”或“只换回二进制”
  带来的假安全感。

负面：

- API 仍需由用户或后续 service manager 启动；本决策不提供 reboot-persistent launchd
  服务。
- macOS 上的 named volume 不便于直接用 Finder 检查，必须使用 Docker 和应用级 smoke
  验证。
- 现有 Compose project identity 尚未统一；升级者必须先确认自己正在复用原 volume。

## 非目标

- 不实现 launchd、自动更新、远程访问或公网托管。
- 不修复任何第三方 Adapter 已移除的外部 CLI。
- 不把临时微信 shell helper 定义为正式 MemoryNexus 或 MiniMax 产品能力。
- 不新增第二后端、直接数据库 Adapter 或媒体持久化实现。

## 相关决策

- [ADR-009: Rust-first 后端主线](ADR-009-rust-first-backend.md)
- [ADR-019: Surfaces vs Adapters vs Engine](ADR-019-surfaces-adapters-engine.md)
- [ADR-021: External Media Evidence References](ADR-021-external-media-evidence-references.md)
