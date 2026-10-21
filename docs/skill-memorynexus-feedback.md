# Skill: memorynexus-feedback（已落地）

> Status: **已落地并验通**。正式 skill 在
> `../.minimax/skills/memorynexus-feedback/SKILL.md`（随仓库走）。
> 本文件是**设计说明存档**，不是待办草稿；两者冲突时以 SKILL.md 为准。
>
> ⚠️ **副本同步**：agent `blackfaced`（`agent-27f35bedcea3`）的运行时副本在
> `~/.minimax/agents/agent-27f35bedcea3/skills/memorynexus-feedback/`。
> 那份是**部署副本**，内容应与仓库这份一致 —— **改仓库这份后要同步那份**，
> 否则两份会分叉（分叉教训见 jhwg 项目 `docs/踩坑记录.md` 第 83 条）。

## 目标

让 owner 在 MiniMax 会话里用自然语言跑
`Observation → Recommendation → Experiment → Outcome → Review` 4 步，
**不接触任何 SQL / 表名 / 字段**。

所有权威写入走 `memorynexus-feedback-cli --ledger <path> <command> <json-via-stdin>`。

## 真实路径（以实际为准）

| 东西 | 路径 |
|---|---|
| CLI | `/Users/mac/.local/bin/memorynexus-feedback-cli` |
| 正式 ledger | `/Users/mac/.local/share/memorynexus/dogfood-2026-08-30.sqlite` |
| 历史 ledger（**只读留存，禁止写入**）| `/Users/mac/.local/share/memorynexus/ledger.db` |
| skill 正文 | `<repo>/.minimax/skills/memorynexus-feedback/SKILL.md` |
| 周解读参考 | `<repo>/.minimax/skills/memorynexus-feedback/references/weekly-perspectives.md` |
| 周解读脚本 | `<repo>/.minimax/skills/memorynexus-feedback/scripts/weekly_review.py` |

- ❌ **没有** `ledger.env` 配置文件 —— 路径直接写死在 SKILL.md 里。
- ❌ **没有** `bin/memorynexus-feedback-cli` —— 用的是 `~/.local/bin/` 下那个编译好的二进制。
- ledger 路径不是运行时算出来的，是**约定**：换路径要同时改 SKILL.md 和 agent 副本。

## CLI 契约（observe 样例）

```json
{
  "confirmation": "confirmed",
  "kind": "initial",
  "statement": "今天下午 3 点 30 分钟午睡",
  "occurred_at": "2026-08-30T15:00:00+08:00",
  "source": "owner_report",
  "idempotency_key": "obs-2026-08-30-1530-nap-001"
}
```

→ `memorynexus-feedback-cli --ledger <ledger.sqlite> observe < input.json`

## CLI 支持的 12 个 command

来源：`src/bin/memorynexus-feedback-cli.rs` 的 command 分派。
（注意 `--help` 打出来的 usage 字符串只列了 3 个，**那行是过时的**，别照它写。）

| CLI command | 自然语言 trigger | 必填 JSON 字段 |
|---|---|---|
| `observe` | "记录", "观察到", "obs: <text>" | statement, occurred_at, kind=initial\|correction, idempotency_key |
| `observation-history` | "最近的观察", "obs history" | (无输入) |
| `retract` | "撤回 <id>", "retract <reason>" | observation_id, reason, idempotency_key |
| `add-recommendation` | "建议", "蚂蚁阿福说 ...", "recommendation" | summary, source=owner\|ant_afu\|agent_candidate, observation_ids[], idempotency_key |
| `start-experiment` | "开始实验", "start experiment" | recommendation_id, action, starts_at, ends_at, expected_signal, idempotency_key |
| `end-experiment` | "结束实验" | experiment_id, state=completed\|cancelled, idempotency_key |
| `record-outcome` | "结果", "记录 outcome" | experiment_id, occurred_at, execution_state, evaluation, note, idempotency_key |
| `review` | "review", "总结", "看本周" | (无输入) |
| `due` | "今天该记什么" | (可选 JSON) |
| `export` | 导出账本 | (无输入) |
| `backup` | 备份账本 | (JSON) |
| `restore` | 从备份恢复 | (JSON) |

前 9 个是日常业务面；后 3 个是运维面（见 `docs/feedback-ledger-recovery.md`）。

## Skill 行为守则（来自 #279 acceptance）

1. **必须 confirmation**：每个权威写入前，skill 必须等 owner 显式说"确认" / "ok" / "写" 才调 CLI
2. **必须 bounded summary**：写入前给 owner 看一条"准备写：xxx"摘要
3. **必须稳定 JSON 入参**：所有 CLI 调用都走结构化 JSON，不用 `printf` 拼字符串
4. **必须 idempotency_key**：用 `<command>-<date>-<hash>` 防止重复写入
5. **fresh session 一致性**：用同一 ledger 路径，跨 session 能读回同一条 record
6. **不暴露术语**：CLI / 内部 SQL / 表名都不暴露给 owner
7. **Pasted 蚂蚁阿福内容**：默认降级为"Recommendation 候选"（source=ant_afu），原始对话默认不持久化
