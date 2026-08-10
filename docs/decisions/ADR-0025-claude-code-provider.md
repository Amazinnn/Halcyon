# ADR-0025：Claude Code Provider（按宠物选择与会话隔离）

- 状态：已接受（2026-08-10）
- 关联：需求 #79、ADR-0022（宠物 = Agent）、ADR-0024（对话与工作流闭合）

## 背景

Focus 已通过 Codex CLI 提供按宠物隔离的真实 Agent 运行时。用户要求把 Claude Code CLI 一并接入；其目标是 Focus 内集成，而不是由 Focus 重新实现 Claude 的认证、模型、代理或权限系统。

## 决策

1. **每个宠物固定选择一个 Provider。**`characters.tool` 是该宠物当前的 Provider 选择；同一时刻一个宠物只使用 Codex 或 Claude。Focus Demo Pet 初始选择 Claude，已有非 Demo 宠物保持 Codex，直到用户在设置中显式更改。
2. **会话按宠物和 Provider 分离。**每日会话存入 `character_provider_sessions`，主键为 `(character_id, provider)`。切换 Provider 不覆盖另一 Provider 的当日会话；切回时可恢复各自的会话 id。旧 `characters.current_session_hash` / `session_date` 保留本发布期兼容，并迁移为 `provider='codex'` 的记录。
3. **Claude CLI 原生拥有凭据和权限。**Focus 只通过本机 PATH 启动 Claude CLI，使用用户本地 Claude 配置。Focus 不读取、写入或显示 Claude 凭据、模型、代理或权限配置；不传递跳过权限的危险参数，也不新增 Focus 审批 UI 或工具白名单。
4. **每个 Claude turn 启动一个新 CLI 进程。**后续同日 turn 仅在该宠物的 Claude 会话 id 存在时恢复对应会话。Provider 运行时、会话和事件均继续按宠物隔离。
5. **对话与工作流的可见语义不变。**直接对话继续发出普通 Agent 事件。工作流调用不把原始过程事件流入聊天；仅在 `showResult=true` 时向目标宠物回流一次带「日程 · 名称」来源的最终结果及一次宠物气泡，`showResult=false` 时两者均不产生。
6. **不扩展冻结的工作流引擎。**不增加节点、模板、看板、历史会话 UI 或通用自动化能力；Mock 仍只允许测试注入，正式路径不增加自动回退。

## 影响

- 存储层提供按 `character_id + provider` 读取和 upsert 会话的接口，运行时以 `AgentProviderKind::as_str()` 传入 Provider 标识。
- 设置页承载按宠物的 Provider 选择；聊天窗口不新增 Provider、模型或权限控制。
- 真实验收必须使用 Focus Demo Pet 和真实 Claude；Claude 不可用时报告原始 Provider 错误，不能用 Mock 替代。
