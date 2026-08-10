# 2026-08-10 对话连续性、常驻 Provider 与触发器检查点

## 范围

- 关联需求：#83。
- 关联决策：ADR-0026（可见聊天历史与常驻 Provider）、ADR-0027（触发节点与每周计划），并受 ADR-0024/0025、事故 INC-001、INC-009、INC-010、INC-011、INC-012 约束。
- 改动范围：Claude stdin `stream-json` 常驻、同日可见消息回放、单次 Skill、无生命周期聊天状态、虚拟触发节点、每周计划、浮窗原生无激活路径与样式探针。

## 自动检查

| 项目 | 状态 | 实际证据 |
| --- | --- | --- |
| 前端单测 | Pass | 2026-08-10：`cd apps/desktop && npm test -- --run`，9 files / 41 passed / 0 failed。 |
| 前端构建 | Pass | 2026-08-10：`cd apps/desktop && npm run build`，`vue-tsc` 与 Vite 通过，98 modules transformed。存在既有 chunk size warning，不影响退出码。 |
| Rust 单测 | Pass | 2026-08-10：`cd apps/desktop/src-tauri && cargo test --lib`，166 passed / 0 failed。保留 6 条既有编译 warning，未新增失败。 |
| 事件协议 | Pass | 2026-08-10：`cd packages/event-schema && npm test`，11 valid / 4 invalid fixtures checked。 |
| Release 重建 | Pass | 2026-08-10：`launch-focus.cmd rebuild` 退出码 0，重建 release 已启动并通过 `focus-cli ping`。 |
| 文档一致性 | Pass | 2026-08-10：需求 #1–#83 连续、14 条事故记录与顶部统计一致、ADR/Eval 链接存在，`git diff --check` 通过。 |

## 本轮自动回归

| 场景 | 状态 | 覆盖 |
| --- | --- | --- |
| Claude 常驻初始化、stdin 多轮输入、原生 delta/tool/完成/错误映射 | Pass | Rust `agents::claude` resident protocol tests。 |
| 取消后销毁并以保存 session 恢复、Focus 重启首轮 `--resume` | Pass | Rust Claude interruption/resume tests。 |
| 可见消息按宠物、Provider、日期隔离与回放 | Pass | 前端 `stores/agent.test.ts` 与 Rust session/storage tests。 |
| 生命周期系统消息不进入聊天 | Pass | `ChatView.test.ts` 与 agent store 状态测试。 |
| Skill 只附加下一条提示并按 Provider 枚举 | Pass | Rust selected-skill test 与前端聊天/store 覆盖。 |
| 虚拟触发节点不进入持久化 nodes/edges | Pass | `WorkflowView.test.ts` 与 workflow serialization tests。 |
| 间隔、每日、每周 next run 及 scheduler 重排 | Pass | Rust workflow/model tests；周一 `0` 至周日 `6`。 |
| 已运行的日程不会在 scheduler tick 重入 | Pass | Rust `workflow_run_claim_prevents_scheduler_reentry_until_released`；领取发生在运行记录写入前。 |
| 工作流 Agent 最终结果回流恰好一次 | Pass | Rust engine tests，`showResult` true/false 分支均覆盖。 |

## 真实 Provider / 控制面

| 项目 | 状态 | 证据或复现步骤 |
| --- | --- | --- |
| Focus Demo Pet 真实 Claude 三轮上下文追问 | Pending | 当前 `claude.exe` 普通 print 与 stream-json 探针均在 55 秒内没有终态；不以 Mock 替代。恢复后连续提出三轮有关联的问题，确认后轮使用前轮事实而非重新解释。 |
| Focus 重启后恢复当天上下文 | Pending | 依赖当前真实 Claude admission；完成至少一轮后退出并重启 Focus，首轮应恢复同一宠物当天消息与 Provider session。 |
| 选择 `focus-cli` Skill 完成只读查询 | Pending | 依赖当前真实 Claude admission；聊天选择一次 Skill，发送只读 `focus-cli` 状态请求；发送后 Skill 选择应清除。 |
| 真实工作流每周实际触发 | Pass | 2026-08-10：临时 `eval-weekly-claim-20260810-211654` 设为本地下一分钟、仅含 35 秒 wait 节点；触发后全程仅 1 条 run，`triggeredBy=schedule`、最终 `success`，随后已删除。 |
| 既有 Claude 控制面 CRUD 闭环 | Pass | 前一检查点已用真实 Claude 经 `focus-cli` 完成临时工作流 create/read/update/run/delete；不以 Mock 替代。 |

## Windows 人工门槛

| 项目 | 状态 | 验收步骤 |
| --- | --- | --- |
| 浮窗非客户区与激活态 | Pass | 2026-08-10：`scripts/window-style-probe.ps1 -ProcessId 28628` 成功；内部 host 窗口没有 `WS_CAPTION`/`WS_THICKFRAME`，含 `WS_EX_NOACTIVATE`，且未成为前台窗口。 |
| 浮窗淡蓝标题条视觉回归 | Pending | 连续打开、关闭、移动、缩放 chat/stats/music/pet/workflow，确认视觉上无淡蓝条。探针结构通过不等于视觉验收。 |
| 轻度/标准/学霸模式 | Pending | 轻度不锁；标准屏蔽 Win、Alt+Tab、Alt+F4、Ctrl+Esc；学霸额外隐藏任务栏和桌面。 |
| 暂停、恢复、跳过、自然结束 | Pending | 各模式逐项执行并确认暂停立即恢复 Shell，恢复按本轮模式重新锁定，最终任务栏/桌面/Win 键正常。 |
| 应用退出与强杀恢复 | Pending | 应用内退出和任务管理器结束主 Focus 进程；最多等待 watchdog 一个轮询周期后确认 Shell 恢复。 |

## 结论

自动化、release 重建、样式结构检查和一次实际每周触发通过。当前真实 Claude CLI 未在 55 秒内给出终态，连续对话、重启恢复与单次 Skill 不能用模拟结果代替；浮窗视觉、桌面锁和聊天视觉回流同样仍待人工验收。本快照不能作为这些项目已验收的证据。完成人工步骤后更新本快照状态，并同步 `docs/STATUS.md` 与 `docs/production-incidents.md`。
