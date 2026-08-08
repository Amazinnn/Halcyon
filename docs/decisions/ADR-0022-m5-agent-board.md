# ADR-0022：M5 Agent 看板 MVP（宠物=Agent，多实例 + 每日会话 + 专属工作区）

- 状态：已接受（2026-08-08）
- 关联：需求 #65/#67/#69；ADR-0019（Agent 概念定稿）、ADR-0020（工作流退化为 Agent 日程工具）；M5 路线（#27）

## 背景

M5（外部 Agent 驱动的角色循环）概念 2026-08-08 定稿（#65，ADR-0019）：每宠物↔一个 Agent、共享对话框、切换替换上下文、过去一天存储但 UI 清空。v1.11.2 实现 MVP，v1.11.3 完善（grill 定稿：事件隔离 / 崩溃自动重启 / 记住选择 / 设置页管理 / 三开关生效）。

## 决策

1. **宠物 = Agent，一对一**（#69 grill 定稿）
   - characters 表即 Agent 定义（DB 迁移 0007 加列：`tool` / `workspace_dir` / `current_session_hash` / `session_date`）。
   - 编程工具（Codex/Claude Code）只是「Agent 的载体」归类：一个 Agent 只属一个工具，一个工具可下属多个 Agent。

2. **多实例 AgentRuntime**：每 character 一个 CodexProvider（registry 按 character_id 索引，懒构建）。envelope 的 `agentId` = character_id——多 Agent 事件**隔离**到各自对话框（#69）。

3. **懒生成工作区 + AGENTS.md**：首次使用建 `%USERPROFILE%/Focus-Agents/<agent-id>/` + `AGENTS.md`（身份/人格**唯一来源**，工具原生识别；`characters.persona` 退役，保留列不删）。设置页「打开工作区文件夹」按钮让用户直接编辑 AGENTS.md。

4. **每日会话旋转**：跨天首谈开新会话（新哈希落库）；当天内切换 Agent 恢复今日会话（resume）。哈希存 Rust/DB，Agent 经 focus-cli `agent session` 读哈希回看历史会话作上下文（#69：Focus UI 不回看历史，去掉 thread 下拉）。

5. **崩溃自动重启（无 fallback）**：Agent 进程失败 → 丢弃该 character 的 runtime，下次调用懒重建（#69：不要复杂 fallback 与防御性编程）。失败时调用方收到错误。

6. **三开关（provider 层生效）**：每 Agent 节点 `showInitial`（首条流式 delta = 开工短句，默认开）/ `showThinking`（后续流式，默认关）/ `showResult`（最终 message.completed，默认开）。对话调用全开（`agent_display_full()`）。引擎 `AgentDisplay` 传给 provider，事件按开关过滤。

7. **系统级输出纪律**：`agents::OUTPUT_DISCIPLINE` 注入**每次** turn（对话 + 工作流），短句换行、禁 Markdown、限 ~200 字。短句换行便于将来泡泡按句截断（截断机制本阶段不做）。

8. **设置页 Agent 管理**：列表 / 删除（**连带删工作区**含 AGENTS.md + 清会话哈希 + 丢 runtime）/ 打开工作区。新增 = 宠物包导入自动建。

9. **Skill 跟随工具**：同一工具的所有 Agent 共享 skill 列表（~/.codex/skills），切换 Agent 不重扫。

## 影响

- 工作流 Agent 节点 `characterId` 参数 → 调对应宠物（Agent）的实例；未绑定（空角色）工作流默认调当前选中 Agent。
- 对话可见性：#69 定稿——Focus UI 只显「当前会话 + 开关开启的工作流输出」，历史不回看；Agent 全知（CLI 读哈希 + workflow CRUD，Boss 语义 ADR-0020 延续）。
- 前端聊天：顶部 Agent 下拉（记住上次选择）、去掉 thread 下拉、选项面板简化（去 Provider 切换）。
- VPN 兼容：WebView2 加 `--proxy-bypass-list=<-loopback>`（v1.11.2，本地页面不被代理劫持）。
