# 下一个对话初始化提示词（Focus Desktop）

> 由 2026-08-07 会话收尾生成。把下面「提示词」直接粘贴到新对话即可；本文件同时归档在仓库，供压缩后自取。

## 提示词

我是 Focus Desktop 项目的维护者。请先阅读仓库根目录的 `docs/STATUS.md`（当前状态与交接页，单一事实源），再按需查阅 `docs/next-phase.md`（路线）、`docs/requirements-verbatim.md`（需求原话 #1–#25）、`docs/decisions/`（ADR-0001~0011）与 `README.md`。

当前真实状态：v1.9.1（本地音乐播放器 + 音乐窗口尺寸化已完成）；M2 已实现（v1.8 统计真实化、v1.9 本地音乐）；M3 已实现垂直切片（Codex CLI 嵌入 + focus-cli 白名单审计 + skills 透传）；M4（内置工作流引擎 / 精简 n8n）与 M5（新的 Agent：外部 Agent 驱动的角色循环）已于 2026-08-07 讨论并锁定方向，但未实施（见需求 #26/#27 与 next-phase）。

路线开放：本轮做什么由你根据 STATUS / next-phase 提出建议并先和我讨论，不默认推进任何大版本。

铁律（AGENTS.md 已写明，务必遵守）：
1. 新需求先以原话追加到 `docs/requirements-verbatim.md`，再动手；不改历史条目（仅状态列可后补）。
2. 重要架构决策写 ADR（`docs/decisions/ADR-00XX.md`）。
3. 代码改动后必跑：`cd apps/desktop && npm run build`、`cd apps/desktop/src-tauri && cargo test --lib`、`cd packages/event-schema && npm test`；涉及前端/Rust 交付前必须 `launch-focus.cmd rebuild`，并给编号手测清单让我逐项验收。
4. 提交风格 `feat(…)/fix(…)/docs(…)/chore(…): …`，分阶段提交，保持工作区干净并 push 到 Amazinnn/Halcyon。
5. 不得修改/移动/重编 `local-focus-desktop-agent-design-v0.2.md`。

环境注意：PowerShell 管道传中文会变 ?，写中文用 .NET `WriteAllText`（UTF-8 无 BOM）；git push 直连易 reset（Clash 开走代理，否则 `git -c http.proxy= -c https.proxy= push`）；cargo 拉依赖需清代理并设 `NO_PROXY=crates.io,index.crates.io,static.crates.io,github.com,*.crates.io`；本机单显示器（多显示器 N/A）。

可选候选（供参考，不强制）：M4 工作流引擎实施；M1 系统托盘；M3 剩余项（plan mode / Diff / 终端面板 / Claude Code 接入）；Agent 状态模拟器完善。