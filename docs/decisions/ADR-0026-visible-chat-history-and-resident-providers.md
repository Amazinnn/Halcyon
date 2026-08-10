# ADR-0026：可见聊天历史与常驻 Provider

- 状态：已接受（2026-08-10）
- 关联：需求 #83、ADR-0022（宠物 = Agent）、ADR-0024（聊天与工作流闭合）、ADR-0025（Claude Code Provider）

## 背景

按宠物与 Provider 保存当天 session 只能让真实 CLI 尝试恢复上下文，不能让聊天窗口回放用户实际看过的消息。Claude 又以逐轮进程运行，连续追问会重复建立进程，削弱了本次 Focus 生命周期内的对话连续性。

## 决策

1. **可见消息按“宠物 x Provider x 本地日期”保存和回放。**聊天窗口打开或切换宠物时只装载当前组合当天的用户、Agent 与来源消息；新的一天从空白开始，不提供跨天浏览。
2. **Claude 在 Focus 生命周期内常驻。**每个 Claude Provider Runtime 启动一个 `claude -p --input-format stream-json --output-format stream-json --include-partial-messages` 子进程，并经 stdin 顺序发送每一轮 JSON 消息。Codex 保持既有 app-server 常驻语义。
3. **重启后的首轮才使用 `--resume`。**Focus 重启、Provider Runtime 首次创建时，若本地当天已保存 Provider session，启动命令带 `--resume`；同一常驻进程内的后续轮次不重复 resume。停止、异常、切换 Provider 或应用退出销毁该进程；下次运行可从已保存 session 恢复。
4. **聊天只展示对话内容。**不持久化或渲染“会话开始/结束、成功、失败、切换 Agent”等生命周期提示；连接或生成中仅作为短暂状态。真实 Provider 错误保留可操作说明，不以 Mock 或伪造成功替代。
5. **Skill 是下一条消息的一次性附加上下文。**`agent_list_skills` 只列出当前 Provider 的 `.codex/skills` 或 `.claude/skills`；用户选择后，后端读取该 `SKILL.md` 并附加到下一次真实提示，发送完成后立即清除。聊天窗口不承载 Provider、模型或长期 Skill 配置。

## 影响

- `characters` 的 Provider 选择、Provider session 与可见消息都以当前宠物为边界；切换 Provider 不串用另一个 Provider 的 session 或历史。
- 本 ADR 取代 ADR-0025 第 4 条“每个 Claude turn 启动一个新 CLI 进程”，其余认证、权限、隔离和正式路径禁用 Mock 的约束不变。
- 真实验收至少验证同一 Claude 连续三轮上下文、重启 Focus 后的当天恢复，以及一次 `focus-cli` Skill 只读调用；窗口视觉回流仍按 Eval 单独人工确认。
