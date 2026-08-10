# 2026-08-10 Claude Provider 与稳定性检查点

## 范围

- 关联需求：#75–#82。
- 关联决策：ADR-0023（桌面锁）、ADR-0024（聊天与工作流闭合）、ADR-0025（Claude Provider）。
- 关联实现：`828c9da..6bd8cd2`，其中 `6bd8cd2` 修复 Provider 切换状态隔离。
- 本快照同时记录文档/质量检查点的建立；它不把未完成的 Windows 视觉手测标记为通过。

## 自动化与交付证据

| 项目 | 状态 | 实际证据 |
| --- | --- | --- |
| 前端构建 | Pass | 2026-08-10 运行 `cd apps/desktop && npm run build`；Vue 类型检查和 Vite 构建通过，95 个模块完成。 |
| Rust 单测 | Pass | 2026-08-10 运行 `cd apps/desktop/src-tauri && cargo test --lib`；158 passed / 0 failed。现有 6 条编译警告未改变本次文档结果。 |
| 事件协议 | Pass | 2026-08-10 运行 `cd packages/event-schema && npm test`；11 valid / 4 invalid fixtures checked。 |
| Release 重建 | Pass | Provider 实现交付时已运行 `launch-focus.cmd rebuild`；本快照仅增加文档，未重复重建。 |
| 文档检查 | Pass | 已验证相对链接、需求 #1–#82 连续性、12 条事故记录与统计一致性，并运行 `git diff --check`。 |

## 真实 Provider 与工作流

| 项目 | 状态 | 实际证据 |
| --- | --- | --- |
| Claude 真实运行时 | Pass | `Focus Demo Pet` 使用本机已登录 Claude Code，未使用 Mock 回退。 |
| Agent 工作流控制面闭环 | Pass | 真实 Claude 经 `focus-cli` 创建、读取、更新、运行、删除唯一命名的临时单 Agent 手动工作流；运行记录为 success，删除后列表为空。 |
| Provider/session 隔离 | Pass | Rust 单测覆盖 Provider 分离 session、Demo Pet 一次性迁移、切换期间 turn 互斥；前端测试覆盖切换后清空当前会话和按角色过滤状态。 |
| 聊天窗口直接发送 | Pending | 打开 chat，选择 Focus Demo Pet，发送一条真实请求，确认显示合理 Claude 回复。 |
| 工作流来源消息与宠物气泡 | Pending | 在 `showResult=true` 的真实工作流中，确认目标聊天恰好一条「日程 · 名称」消息、一次宠物气泡和列表刷新；`showResult=false` 不应回流。 |

## Windows 稳定性人工门槛

| 项目 | 状态 | 验收步骤 |
| --- | --- | --- |
| 浮窗非客户区与淡蓝标题条 | Pending | 运行 `scripts/window-style-probe.ps1`；连续打开、关闭、移动、缩放 chat/stats/music/pet/workflow，探针无 caption/thick frame，窗口不激活且无淡蓝标题条。 |
| 三档专注锁 | Pending | 轻度不锁；标准屏蔽 Win、Alt+Tab、Alt+F4、Ctrl+Esc；学霸额外隐藏任务栏与桌面。 |
| 暂停与结束恢复 | Pending | 每档验证开始、暂停、恢复、跳过、自然结束和连续点击；暂停立即完整解锁。 |
| 强杀 Shell 恢复 | Pending | 在学霸模式结束 Focus 主进程，最多等待 watchdog 一个轮询周期；任务栏、桌面、Win 键和任意窗口均恢复。 |

## 结论与后续

自动化、重建和真实 Claude 控制面闭环通过。直接聊天视觉回流、浮窗样式和桌面锁仍为 `Pending`，因此不能宣称完整人工验收通过。相关生产问题见 [生产事故台账](../production-incidents.md)，长期执行规则见 [Eval 检查点](./README.md)。
