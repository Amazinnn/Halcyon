# ADR-0001：AgentEvent 协议 v1 —— 信封 + 事件联合

- 状态：已接受（2026-08-05，任务 D）
- 关联：设计稿 v0.2 §8.2；`packages/event-schema/`

## 决策

- 以 `packages/event-schema/agent-event.schema.json`（JSON Schema draft 2020-12）为唯一事实源。
- 每条消息使用信封：`{ schemaVersion: 1, agentId, sessionId, timestamp(ISO-8601 UTC), event }`。
- 内部 `event` 为 11 种事件的判别联合（session.started / message.delta / message.completed / tool.started / tool.completed / file.read / file.changed / permission.requested / status.changed / session.completed / session.error）。

## 相对设计稿的细化

- 设计稿 §8.2 联合类型中每个事件都携带 `sessionId`；本 ADR 将身份信息统一收进信封，事件对象只承载领域载荷，避免冗余与不一致。
- `risk` 枚举固定为 low/medium/high/critical；`state` 使用 §5.2 的 14 态枚举。
- 配套产出 `agent-event.ts`（TS 类型 + `AgentState` 14 态 + `PetReaction`）、`fixtures/valid`（11 个合法样例）、`fixtures/invalid`（4 个反例）；`npm test` 用 Ajv 校验。

## 后果

- 任何 Agent 的原始协议先经 Adapter 转成此信封，Panel/Pet/Scheduler 只能消费统一事件。
- Schema 未稳定前不接任何真实 Agent；版本升级走 `schemaVersion`。