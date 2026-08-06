# ADR-0007：M3 Agent 接入 = 嵌入真实 Codex CLI（Claudian 式 + focus-cli 技能 + 白名单审计）

- 状态：已接受（2026-08-06，v1.6）
- 关联：设计稿 §26/§28（M3「OpenCode Agent」仅作架构参考，不照搬）；ADR-0001（AgentEvent v1）；ADR-0002（事件总线）；ADR-0006（focus-cli 本地控制面）

## 背景

用户明确两点：① 不直接内置 Agent，设计稿 M3「OpenCode Agent」只提供架构参考；倾向以 Obsidian「Claudian」插件的形态，把 Codex / Claude Code 的 CLI 嵌入 Focus 的对话窗口；② 接入的 agent 应能调用本软件提供的一系列 CLI（focus-cli）、能调用 agent 自身具有的 skill（UI 上给出用户选项），具备一切 agent TUI 所具有的功能，参照 Paseo 项目（getpaseo/paseo：本地 daemon 编排 Claude Code/Codex 等，只包装不拦截 agent 会话，agent 用完整本地环境 + 技能）。

## 决策

1. **形态：不内置 Agent，嵌入真实 Codex CLI（app-server）**。Focus 以 stdio 行协议与 `codex app-server --stdio` 通信（JSON-RPC 风格，线上无 `jsonrpc` 头字段）：`initialize`（clientInfo.name=`focus_desktop`）+ `initialized` 通知 → `thread/start|resume|list`、`turn/start|interrupt`；订阅 `item/started|completed`、`item/agentMessage/delta`、`turn/completed`（含 error）等通知，映射为 AgentEvent v1 信封经事件总线转发（`agent:event` / `pet:state_changed` / `bubble:requested`）。
2. **首接 Codex；禁止 spawn `.cmd/.ps1` 包装**。探测 `%LOCALAPPDATA%\OpenAI\Codex\bin\*\codex.exe`（版本目录会变，取最新可执行文件；缺失/启动失败 → Mock fallback + UI 徽标）。Claude Code（`D:\nodejs\node_global\claude.exe`）为后续 provider。
3. **Provider 抽象**：`AgentProvider` trait（start_thread / resume_thread / list_threads / send / interrupt / status）；`CodexProvider`（真实）与 `MockProvider`（脚本化回话，保留 schema 校验）实现；`settings.agentProvider`（默认 `codex`）选择。
4. **focus-cli 白名单 + 审计（落实 ADR-0006 预留项）**：focus-cli 新增可选 `--agent-thread <thread_id>`；宿主 `cli.rs` 对带该标志的请求强制白名单（`ping`、`timer start|pause|skip|status`、`stats today|week|sessions`、`desktop layout`、`apps now|visible`；`debug` 及未登记/未来命令默认拒绝），每次调用（放行与拒绝）审计写入 `supervision_events`（rule=`agent_cli_call`，payload=thread_id / command / allowed / result，storage 迁移 0004 增加 payload 列）。不带标志的交互式调用行为不变（全量、不审计）。
5. **skills 透传（Paseo 思路）**：依赖 Codex 原生加载 `~/.codex/skills`（本机已有大量技能）；自动安装内置 `focus-cli` skill（SKILL.md 内容随应用同步）教 agent 用 `focus-cli --agent-thread <thread_id> …` 编排 Focus；不拦截 agent 会话。本轮不做自定义 skill/MCP 服务器，也不在 thread/start 额外注入引导消息（依赖技能描述被模型感知，与 Paseo 机制一致）。
6. **工作区可配**：`settings.agentWorkspaceDir`（默认用户主目录）作为 agent cwd 传入 `thread/start` / `turn/start`。
7. **UI 基础选项**：provider 切换、工作区目录显示/编辑、新建/恢复/停止会话、技能列表（点击发送引导）、focus-cli 快捷 chip。

## 风险

- agent 继承用户 Codex 配置（`approval_policy="never"`、`sandbox_mode="danger-full-access"`、自定义 provider）→ 白名单只是 Focus 侧第一道闸，agent 本身拥有 shell 全权限；对外发布前需重审（后续：审批 UI M5、权限 profile、token 轮换）。
- app-server 进程惰性启动（首次打开对话窗口时），应用退出时 kill；异常退出后下次调用时重建。
- app-server 协议处于 alpha（本机 0.146.0-alpha.3.1）：实现时以 `codex app-server generate-json-schema` 生成的 schema 交叉核对字段；serde 结构体最小化、忽略未知字段。

## 后果

- 新增 `agents/codex.rs`（CodexProvider + 协议/事件映射/探测）、`agents/mod.rs`（trait + AgentThreadInfo + 共享 validate_envelope）；改造 `agents/mock.rs`（MockProvider 实现 trait）；AppState 增加 agent runtime / fallback / events_tx；settings 增加 `agentProvider` / `agentWorkspaceDir`；storage 迁移 0004。
- focus-cli 增加 `--agent-thread`；cli.rs 增加白名单 + 审计。
- 前端：agent store 状态机与 delta 合并、ChatView 真对话 UI、SettingsPopover agent 设置。

## 后续（本 ADR 不实施）

- Claude Code provider；plan mode / Diff 查看 / 终端面板（完整 TUI 对标）；审批 UI（M5）；多 provider 统一管理；token 轮换与加密。