# 当前知识读取与订正合同 G1

状态：**#318 合同 v1 于 2026-10-27 完成 Coordinator 审阅并冻结；task read/check 已实现，
repository read/check 与 launch 分别由 #328/#329 交付。** 方向见
[ADR-030](../decisions/ADR-030-current-knowledge-maintenance.md)，父任务为
[#317](https://github.com/blackfaced/MemoryNexus/issues/317)。本文以
`46434b0`（含 #322、#323、#325）为审阅时的已实现基线。
审阅结论与实现切片见 [review](current-knowledge/contract-review.md)。
已有 CLI 与新增 adapter 接口分开定义；不得把本文的新字段传给现有 CLI。
验证矩阵见 [验收记录](acceptance/current-knowledge.md)，实施范围见
[handoff](current-knowledge/runtime-handoff.md)。

## 1. 当前用途的两条路径

| 路径 | 权威读取与入选条件 | 提供给客户端的身份 |
| --- | --- | --- |
| task | 现有 `context {"limit":20}`；仅 current 的 instruction、checkpoint、execution，以及 owner_confirmed lesson | 保留原 kind/trust。execution_reported 是执行报告，owner_confirmed 是 owner 确认，不保证客观真相 |
| repository | v2 workflow-status + 已发布受管区块 + 原始发布计划/回执；current、approved decision，对应 current、applied 的非 maintenance publication | consumer_output；列出全部输入/来源依赖及原 trust，不增加 owner_confirmed 标记 |

task 路径中的 review_candidate 只提供 id/kind/trust/state 的候选索引，不提供正文，
不能作为当前结论。draft 一律不进入该路径，即使 correct 已把 trust 改为
owner_confirmed。repository 路径允许 current draft 作为 consumer 输入，保留
source_reported 或其明确订正后的 owner_confirmed；批准发布只授权该份文档输出。
proposed/deferred/skipped decision、尚未 applied、失效或维护用中性文字均不是当前知识。
两条路径分次调用，不自动混成一个不明来源的文本块。

第一阶段只保证所选范围和已知依赖的一致性。两个不同 ID 的普通 record 即使内容矛盾，
仍可同时 current；不按时间或相似度偷偷挑一个。跨 ID 关系治理属于 #317 第二阶段。

## 2. 明确订正：复用已支持的接口

使用绝对安装路径、当前 task 的 config 和显式 ledger：

```text
env MEMORYNEXUS_TASK_LEDGER=LEDGER BINARY task --config CONFIG context
env MEMORYNEXUS_TASK_LEDGER=LEDGER BINARY task --config CONFIG correct
```

每次 stdin 是一个 JSON 对象。context 当前用途仅传 `limit`；`record_ids` 只用于显式
审阅/目标检查。correct 的完整合成请求见
[task-sequence.json](current-knowledge/examples/task-sequence.json)。调用前展示当前 task、
准确 target_id、原正文及精确替换正文，获取该次内容确认。保留请求原字节、key、run_id、
client 和时间用于 lost-response 重试。成功后重新读取，不能把 replay 当作新修订或新批准。

没有唯一目标时停在提案；已 superseded/stale/retracted 或并发更改时停止并报告 conflict。
不得换成 record、自动跟随到新 target_id、重写请求重用 key，或自动确认新正文。
需要新目标/正文就重新展示并取得确认。correct 保留 kind，以 owner_confirmed 新修订
替代旧 current 并失效旧依赖。lesson 的 source_ids 必须仍满足现有同任务/current 约束。
source drift 后 stale draft 走 source-capture/复核，不用 correct 复活。

`confirmation:"confirmed"`、client 标签及批准摘要是可信本地调用者声明；CLI 没有认证
屏幕前的人。OS 单 owner 与既有内容级确认边界不变。读取 adapter 不提交任何权威写入。

## 3. adapter 的精确入口（frozen v1，分切片交付）

新增接口定义为 `integrations/task-memory/scripts/current_knowledge.py`，仅标准库与既有 CLI：

```text
PYTHON current_knowledge.py read --binding ABS --request ABS --run-dir ABS
PYTHON current_knowledge.py check --binding ABS --evidence ABS --run-dir ABS
PYTHON current_knowledge.py launch --binding ABS --evidence ABS --run-dir ABS
```

不增加 task CLI 子命令、字段或 SQLite schema。`read` 生成材料；`check` 在每次当前用途
之前重新检查；`launch` 先执行同样检查，再将材料交给一个固定的外部 client bridge。
当前 G2 已支持 task 模式的 read/check；repository 模式、repository_binding 和 launch
在 G2 返回 unsupported_version，待 #328/#329 实现。已有 workflow.py 的
prepare/preview/apply/status 不被改名。

### Binding

新的读取 binding 是独立 0600 文件，放在 task 外的 0700 目录；不是修改 task config 或
旧 workflow binding。必填字段正好为：

```text
{schema_version:1, task_id:UUID, task_root:ABS, config:ABS, ledger:ABS,
 binary:ABS, binary_sha256:H, python:ABS, package:ABS, package_sha256:H,
 state_root:ABS, timeout_seconds:U, client:Name, copies:[Copy]}
```

可选字段仅为 `repository_binding:ABS` 和 `bridge:{executable:ABS,sha256:H}`。
repository 模式必须有前者；launch 必须有后者。bridge 是安装时明确选择的单个可执行文件，
无 shell、额外 argv、插件发现或模型生成命令；真实客户端参数由该桥接文件固定。
安装时需要 pin 它实际调用的客户端版本和配置并在 #312 核验，脚本哈希本身不证明这点。
schema/UUID/H/name/整数、严格 JSON 与 canonical SHA 规则复用
[kernel](repository-memory/kernel-v1.md)。timeout 为 1–300 秒。

复用 runner 的绝对路径/no-symlink/私有权限检查、binary/package inventory 哈希算法，
config 必须是 task_root/.memorynexus.json 且 task_id 相同，ledger 必须已存在且在 task 外。
state_root/package 在 task 外，run-dir 必须是 state_root 的直接子目录且独占新建；
state_root 必须为专用的私有读取目录，不与 workflow run、package 或 ledger 目录重叠。
binding 位于 task 外，禁止它位于 package 或 state_root 内。当前执行的脚本必须位于
binding.package/current_knowledge.py，不能校验另一个包后执行未绑定脚本。
现有 #325 installer 将源码 scripts/ 中的文件平铺到 package 根；读取入口复用该布局。仅支持已显式迁移的
v2 ledger，以便检查 incarnation；v1 返回 unsupported_version，绝不自动迁移或 init。
普通旧 v1 context 仍可独立使用，但不提供本合同的恢复一致性保证。
repository_binding 必须与 task/root/config/ledger/binary/python/package 完全相同；其自身
摘要、profile 和 kernel workflow-bind 也必须匹配，不因为读配置正确就绕过旧绑定。

Copy 正好为 `{id:Name,path:ABS,role:"managed"|"external",consumer_id?:Name}`，至多 20。
managed 必须指向已绑定 target，consumer_id 必填；external 禁止该字段。盘点范围包含
会被入口注入的 AGENTS、skill、native memory、缓存和受管目标；文件名本身不证明会被读取。
未能枚举/观察实际注入链时 coverage 必须为 unknown。external 只记录摘要，不将其正文
混入当前材料、不宣称其中的语义已校正。要把它排除出实际输入，需要 bridge 的运行证据，
不能因有一个预期文件哈希就判断旧说法消失。
v1 没有接收外部排除证明的字段：所有 external 项始终 unknown，read/check/launch 均
返回 injection_unknown 且不产生当前材料。不可读 external 仍是 unknown，不因缺少哈希
变成 verified。managed 项仅在 repository 模式可验证；task 模式中存在 managed 项也
返回 injection_unknown。按 id 排序并拒绝重复 id/path；自动加入的 target 与显式项发生
身份冲突时拒绝。实际入口还有未盘点输入时不能声称 coverage 完整。

### Request

`--request` 文件最多 32 KiB，严格对象，无额外字段：

- 当前 task：`{schema_version:1,mode:"task",purpose:"current",limit:20}`。
- task 历史：上式 purpose 改为 `"history"`，另有必填 `record_ids:[UUID...]`，1–20 个唯一 ID。
- repository：`{schema_version:1,mode:"repository",purpose:"current",consumer_ids:[Name...]}`，
  1–2 个已绑定 consumer，禁止 limit/record_ids。按所选 consumer 的全部 current 输出读取。

limit 在 task 两种用途均为 1–20，默认值不存在，显式传入。
[read-request.json](current-knowledge/examples/read-request.json) 是冻结接口示例。
历史 read 可以返回 superseded 正文，保留状态，材料标记 `current_use:false`；
stale/retracted 仍不返回正文；review_candidate 在历史用途也只保留候选元数据。
历史只含 tombstone 仍可成功，历史省略则 incomplete；当前用途无可用正文才返回 empty。
历史材料禁止 check/launch 作为当前用途使用。
不得把历史材料与后续新会话当前输入拼接。

### 环境、返回值与交付

调用 Python 时设置 `PYTHONDONTWRITEBYTECODE=1`，源码也在导入前禁用 bytecode。
CLI 子进程显式设置 `MEMORYNEXUS_TASK_LEDGER=binding.ledger`、固定 PATH/LANG，
不继承其他 ledger 变量，不搜索父目录。请求不经 shell，cwd 为 task_root。

每个命令 stdout 正好一个有界 JSON：

```text
{schema_version:1,status:"ok"|"needs_review"|"error",read_id:UUID,
 code:Code,artifacts:[{path:relative-path,sha256:H,bytes:U}]}
```

exit 0 仅 status=ok；needs_review 为 3，error 为 2。Code 是
ok/invalid_input/unsupported_version/io_error/scope_mismatch/stale_snapshot/
source_drift/source_unavailable/target_drift/injection_unknown/incomplete/empty/
history_only/bridge_unavailable/delivery_unknown/delivery_already_attempted。
invalid_input/unsupported_version/io_error/scope_mismatch 为 error；其他非 ok code 为
needs_review。CLI conflict/recovery_required 映射 stale_snapshot；CLI not_found/storage/
超时/无效响应映射 io_error，路径/权限/JSON/超限输入映射 invalid_input。不透传自由错误。
固定不含正文的诊断；无自由错误文本。
输出及 evidence 最大 128 KiB，request/binding 32 KiB，material 最大 64 KiB。
超限不得截掉一部分再称成功。单次 CLI stdout 超过 128 KiB、超时、非零、重复 key、
不匹配 task_id 或无效 JSON 均停止，不回退缓存、隐藏记忆或另建 ledger。

`read` 产出 `material.json`、`evidence.json`、`result.json`；失败只产出无正文 result/evidence。
若绑定/账本不可读导致必填身份未知，则仅产出 result，不捏造 evidence 字段。
不得留下可交付 material。`check` 只写新的 result/evidence，引用原 read_id 与原 evidence
摘要，不能修改旧快照。check evidence 不含 material 字段，不能再次作为 --evidence。
check/launch 的 --evidence 只接受同一 state_root 直接子目录的原 read evidence.json：
必须有 material、没有 previous_evidence_sha256；同目录 result.json 必须是成功 read 的
结果并包含匹配的 evidence 与 material 摘要/长度，read_id 一致。材料可已按生命周期删除；
check 依据重新读取的字节比对原摘要；仍存在的材料必须同时通过摘要校验，禁止相信被改的副本。
launch 必须使用重新读回且与原摘要相同的字节，原文件存在时其字节也必须相同，不能从缓存
跳过复查。失败时删除原 read material，保留无正文证据；不删除无效路径指向的任意文件。
`launch` 同样复查，再以 argv `[bridge.executable]`、cwd=task_root，
把 **material.json 的原字节**作为 stdin 发给 bridge；不做 Markdown 二次渲染。
bridge 可继承客户端所需的认证环境，但 adapter 不记录它；ledger 相关变量由绑定覆盖。
bridge stdout 限 4 KiB，必须为 `{schema_version:1,read_id:UUID,
material_sha256:H,session_ref:string}`，session_ref 1–128 字符。
bridge stderr 不进入证据正文；超时或丢失回执返回 delivery_unknown，禁止自动重启客户端。

launch 在 spawn 前以 O_EXCL/no-follow 在原 read 目录创建并 fsync
`delivery-started.json`，字段为 `{read_id,material_sha256,run_dir:ABS,started_at}`。
已存在即返回 delivery_already_attempted，不重复 spawn，即使改了 --run-dir。
这是交付去重标记，不是新账本；未知中断由人工/客户端证据协调，不能删除标记后自动重试。

launch 记录 `delivery.json`：`{schema_version:1,read_id,material_sha256,bytes,
bridge_sha256,started_at,finished_at,status:"acknowledged"|"unknown",session_ref?}`。
acknowledged 要求 exit 0、摘要/ID 完全匹配；它只证明可信 bridge 的交付声明。
真实客户端输入 trace 与后续动作另外验收，不能用自报摘要替代。
bridge 若不能证明 fresh session 收到这些字节，应返回非零，不能伪造 ack。
MiniMax/Codex 的实际启动、hook 能力与版本映射由 #312/#301 核实，本合同不假定它们
原生支持这个 stdin 协议；尚无可用 bridge 时 read 可供检查，launch 必须失败。

## 4. 同一读取快照与实际材料

每次 read 按以下顺序执行，任何写入都留给既有明确授权流程：

1. 核验安装/binding/config。调用 `workflow-status {}` 获得 task revision R 与 incarnation I。
   v2 即使没有 workflow 也能返回 header。保存实际观察时点。
2. task 调用 context 的现有 JSON，要求返回 revision=R。repository 的所有 status 页使用
   `{"workflow_id":W,"view":V,"limit":20,"at_revision":R}`，后续页加 `after:next_after`。
   所有页 task/I/R 相同，必须读到 complete；不得把每个 view 独立取到的“最新页”拼接。
   需要 summary、collections、sources、inputs、decisions、publications；指定 ID 可用已有
   ids 过滤。每 view 最多 128 页，总响应最多 16 MiB，超限 incomplete，不扫描整个库导出。
3. repository 对所选 decision 的完整 record/source 依赖核验 current、源版本/哈希、
   availability、profile、非 blocked。逐个读实际源文件/selector/JSONL 已消费 prefix，
   复用现有 runner 的原字节、边界、最大文件大小规则；未纳入账本的完整新行也报告 drift。
   选中 collection 的未完成尾行报告 incomplete。evidence_ref 路径不自动成为监控依赖。
4. 每个输出必须有当前 applied publication 与其原发布计划/回执。验证 proposal 的
   plan_sha256、decision、target、block、output_sha256，取计划中原 output_text；按现有
   render 规则正规化末尾 LF 后与 target 的唯一受管区块逐字节比较。缺原计划/回执就停止，
   不从文件反推批准，不让模型重造原文。whole-file 哈希作为本次文件快照记录并重查；
   不要求同一 target 的每个历史 applied after_sha 都等于现在整个文件（其他区块可后续发布）。
   未识别/失效区块必须计入 copy 缺口，不能以某个区块正确推断整份文件可注入。
   repository 模式自动把选中 consumer 的所有 target 纳入 copies，以 target_id 为 id，
   与显式 managed 项去重；它们的 whole-file 摘要必须写入证据，不能由调用者漏报。
5. 核验已声明 copies，记录无法观察的注入。把实际读取材料序列化并计算哈希。
   再读所选外部字节/绑定，与前次指纹比较；最后 `workflow-status {"at_revision":R}`
   仍须 I 相同。变更则丢弃材料，整个 read 最多重新开始一次，再变则 stale_snapshot。

以上是有限时间点的复查，不是跨 SQLite/文件/客户端的全局事务。它不能发现外部文件
在两次观测间修改后又还原，也不能阻止检查后写入。check/launch 必须重新执行完整读取
和指纹比对；任何 R/I、安装、来源、区块、材料或 copy 变化都让原 read 失效，返回
stale_snapshot（具体来源/目标可用对应 code）。不自动接受新快照、不复用旧批准。
重新 read 与 fresh session 是显式下一步。check/launch 重建材料时保留原 read_id，以便
字节比较；它们不得用新 ID 掩盖更换材料。比对来源/副本指纹时忽略本次 checked_at，
其余身份/版本/哈希不得改变。长会话每次依赖材料做新动作前再 check；
无法强制该检查的入口只能标为 best effort，不能宣称持续最新。

### Material 与 evidence 精确字段

material 是 canonical UTF-8 JSON 加一个末尾 LF；摘要包含该 LF：

```text
{schema_version:1,read_id,task_id,revision,incarnation,mode,purpose,current_use,
 entries:[Entry],candidates:[Candidate],outputs:[Output],gaps:[Code]}
```

Entry 直接保留 context 返回的字段（id/kind/trust/state/revision/source_ids/text 等），
不得补未返回的正文。Candidate 只有 id/kind/trust/state。Output 正好为
`{consumer_id,decision_id,approval_id,input_record_id,block_id,text,
classification:"consumer_output",dependencies,inputs:[{id,kind,trust,state}]}`；
dependencies 原样保留 kernel typed 形式，inputs 含所有 record dependencies。
task 模式 outputs=[]；repository entries/candidates=[]；各数组最多20。
entries/candidates 保持 context 顺序；outputs 按 consumer_id/decision_id 排序，inputs 按 id
排序；evidence selected 与材料记录次序相同，sources/outputs/copies 分别按 source_id、
consumer_id/decision_id、id 排序。created_at/checked_at 使用 UTC 秒精度 RFC3339；
check 保留原 created_at，只刷新 checked_at（包含 sources/copies 的 checked_at）。
repository 没有新 trust 标签。text 来自上述已验证的原输出，不来自源文件自由拼接。

evidence 正好为：

```text
{schema_version:1,read_id,created_at,checked_at,task_id,revision,incarnation,
 binding_sha256,config_sha256,binary_sha256,package_sha256,request,
 selected:[{id,kind,trust,state,revision}],
 sources:[{source_id,source_revision,sha256,availability,observed_at,checked_at}],
 outputs:[{consumer_id,decision_id,approval_id,block_id,output_sha256}],
 copies:[{id,status:"verified"|"stale"|"unknown",sha256?,checked_at}],
 coverage:"declared_files"|"unknown",omitted_count:U,candidate_count:U,
 truncated:boolean,gaps:[Code],material?:{path:"material.json",sha256:H,bytes:U}}
```

repository evidence 另有必填 `workflow:{workflow_id,profile_revision,profile_sha256,
binding_sha256}`；其他模式禁止。check evidence 另有 `previous_evidence_sha256:H`。
selected/sources/outputs 最大400，copies最大20，gaps去重且最大20。超限 incomplete。
source observed_at 来自 ledger，checked_at 是本次实际读回时间，不伪装一次新 source-sync。
示例 [read-evidence.json](current-knowledge/examples/read-evidence.json) 的 material 摘要
对应同目录的 [material.json](current-knowledge/examples/material.json) 原字节。
示例 ID、时间和安装摘要均为合成占位，不证明实际安装；仅 material 的字节数和摘要
可对示例文件直接复算。真实 runtime 只能填实际观察结果。
来源全文、原私聊、完整模型 prompt、认证环境均不入 evidence；它不是另一个知识数据库。

omitted_count/truncated 保留 context 的真实结果；有遗漏、空的可用内容或 source/target/copy
缺口时 needs_review，不生成 current material。候选计数单列，候选索引不是遗漏已确认事实。
task 记录的未监控 evidence_refs 不代表来源鲜活保证。没有来源依赖时 sources=[]，不能虚构
已扫描世界。无 copies 的普通 read 可为 declared_files，但真实 harness coverage 仍需独立证据。
external copy 未完成排除核验时为 unknown，launch 停止；不能通过删除盘点行掩盖实际注入。

## 5. 文件漂移、撤回与证据生命周期

read/check/launch 不 sync、不修文件、不 neutralize。发现 source drift/unavailable 后返回
对应缺口，使用原 workflow capture 的 metadata invalidation，再按既有授权复核/发布。
有 neutralize_stale 授权时由原 runner 执行固定中性替换；没有就 pending，不能自行删除。
人工区块保持原样；不能用“更正/补充/之前”关键词充当语义判定或删除规则。

运行材料 owner-only，目录0700/文件0600/no-follow/独占新建。每个 read 独立 UUID；并发
目录冲突失败，不互相覆盖。所有结果引用精确摘要，无共享可变 latest.json。一个 bridge
ack 丢失不能重发 launch；先由安装/客户端证据协调。只读命令不需要占用写入事务锁。

material 是短期交付副本：launch 结束或失败后删除原 read 目录内 material，保留摘要证据；
read 的复用有效期为 created_at 后24小时；未来时间或已过期 evidence 返回 stale_snapshot，
不能通过 check 刷新 created_at 延期。未 launch 时由操作者在检查后删除材料；每次命令
清理 state_root 直接子目录中已过期且通过本合同证据/摘要验证的 material，最多检查1000个
目录，超限 incomplete。损坏的证据或不匹配的材料交由操作者处理，不能按文件名任意删除。
没有后台进程时不能保证文件恰好在24小时物理消失；24小时是可复用上限，物理删除是命令
触发及人工清理。check 不依赖缓存正文，重新读取并比对原摘要；历史材料同样短期。
含可回放正文的 artifacts 不纳入普通 Git、公开验收附件或长期 read 日志。

撤回后的任何新 read/check/launch 均重新检查状态，不能从缓存返回正文；协调撤回时清理
state_root 内有关 material 和已声明外部副本，并按原 workflow 中性化/待确认规则处理。
不声称原 retract 命令已清理外部文件、旧会话、旧备份或 OS/客户端副本。
不可访问的副本必须 unknown。保留的无正文 read evidence 可用于审计，不作为知识重放。
现有 export 会保留独立 stale 派生记录自己的正文；撤回链清除不等于全库语义擦除。
有其他记录引用/复述该内容时，需明确纳入单独撤回范围。新的当前读取仍排除 stale 正文。
restore 后 I 改变，旧 evidence/plan/批准无效；只用包含撤回状态的备份验证不会复活正文。
更早备份的删除与私人证据保留需要单独 owner 决定，不属于自动清理。

## 6. 入口保证与交付条件

| 入口 | 可声明的覆盖 | 仍需证明 |
| --- | --- | --- |
| 强制 hook | 该客户端版本实际经过的启动/使用前事件；检查失败阻止受控动作 | 事件是否不可绕过、material 是否进入模型实际输入、外部副本是否也注入 |
| 显式 wrapper/bridge | 仅经它启动的 fresh sessions 和它控制的材料 | 直接启动客户端会绕过；bridge ack 不等于模型使用 |
| 提示词/AGENTS/skill | 给 Agent 的调用建议，best effort | 可能漏调或继续用旧上下文，不能标强制覆盖 |

第一阶段交付同时验证字节、修订/来源身份、实际客户端无害动作；成功不是只有 ledger
JSON 正确。模拟桥接、真实 MiniMax、真实 Codex、实际 scheduler 分栏。旧会话无法远程
“失忆”，订正后验收从 fresh session 开始，运行中失效要求停用旧材料并重新读取。
#312/#301/#313 与 #283 的现有安装、真实运行、owner 决策 gate 均保持原义。
没有固定实现 commit 和实测入口证据前，本合同不得用于宣称接入已完成。
