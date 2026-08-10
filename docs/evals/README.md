# Focus Desktop Eval 检查点

## 目的

本目录是 Focus 的发布证据库。它不替代测试代码、ADR 或事故台账：测试证明可重复行为，ADR 记录决策，事故台账记录真实问题；Eval 将一次改动的范围、已运行证据、人工结果和未完成项目放在同一个可追溯检查点中。

## 每轮任务的硬规则

**每一轮任务结束前必须更新 `docs/evals/`。**

1. 新增 `YYYY-MM-DD-<topic>-checkpoint.md`，或更新该轮已创建的同名快照。
2. 记录提交前的改动范围、实际执行的命令及结果、人工步骤的 `Pass`、`Fail`、`Pending` 或 `N/A` 状态。
3. `Pending` 不是通过；不能用 Mock、单测或推断替代要求真实 Provider 或真实 Windows 行为的项目。
4. `STATUS.md` 链接最新快照；若发现生产问题，同时新增或重开 `docs/production-incidents.md` 的记录。

文档专用轮也必须有 Eval 更新，但只要求文档链接、需求编号、状态一致性和 `git diff --check`。不因纯文档变更重复运行构建。

## 按影响范围的门槛

| 改动范围 | 必做自动检查 | 必做人工或真实检查 |
| --- | --- | --- |
| 任意前端或 Rust 行为 | `apps/desktop: npm run build`；`apps/desktop/src-tauri: cargo test --lib`；`packages/event-schema: npm test`；`launch-focus.cmd rebuild` | 受影响功能的编号手测 |
| Agent Provider、聊天或工作流 | 上述全部 | 当前宠物通过真实 Provider 获得合理回复；由它经 `focus-cli` 创建、读取、更新、运行、删除唯一命名的临时工作流；确认成功记录和最终清理。不得回退 Mock。 |
| 工作流结果回流 | 上述全部 | `showResult=true` 时目标 Agent 聊天恰好一条「日程 · 名称」消息、一次宠物气泡、列表即时刷新；`showResult=false` 时不出现两者。 |
| 浮窗、拖拽或窗口样式 | 上述全部 | 运行 `scripts/window-style-probe.ps1`；连续打开、关闭、移动、缩放 chat/stats/music/pet/workflow，确认无 caption、thick frame、异常激活或淡蓝标题条。 |
| 专注锁或桌面锁 | 上述全部 | 验证轻度、标准、学霸三档的开始/暂停/恢复/跳过/自然结束；标准模式屏蔽 Win、Alt+Tab、Alt+F4、Ctrl+Esc；强杀 Focus 主进程后 watchdog 最多一个轮询周期恢复 Shell。 |
| 仅文档或交付记录 | 文档链接、编号和状态检查；`git diff --check` | 无 |

现有项目规则仍适用：前端/Rust 改动未完成四个自动命令和 `launch-focus.cmd rebuild` 前不得交付；历史待验项不会阻塞无关修改，但受本轮影响的项目必须完成。

## 快照模板

每个快照至少包含：

- 改动范围、关联需求/ADR/事故编号和 Git 修订；
- 自动检查命令、日期、结果与测试数量；
- 真实 Provider 或原生 Windows 项的逐项状态与简短证据；
- 未通过或待验项目、下一位验收者可复现的步骤；
- 对应 `STATUS.md` 和事故台账的链接。

不要记录 token、会话原文、私有路径以外的凭证或 Provider 配置机密。
