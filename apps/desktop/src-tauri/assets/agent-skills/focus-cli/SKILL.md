---
name: focus-cli
description: Focus Desktop 本地控制面调用技能。当用户要求查询或控制 Focus（计时器、统计、桌面布局、正在运行的应用）时，使用 focus-cli 命令。
---

# focus-cli：编排 Focus Desktop

Focus Desktop 运行期间提供 `focus-cli` 本地控制面（localhost TCP + token，要求 Focus Desktop 已在运行）。
**调用方式**：始终携带 `--agent-thread <thread_id>` 以启用宿主的白名单与审计。

## 获取当前线程 id

Focus 会把当前 Agent 线程 id 写入 `~/.codex/focus-thread.json`（格式 `{"threadId": "...", ...}`）。
每次调用 focus-cli 前先读取该文件，把其中的 `threadId` 作为 `--agent-thread` 的值；文件不存在时使用 `focus`。

## 白名单命令

- `focus-cli --agent-thread <thread_id> ping` — 连通性检查
- `focus-cli --agent-thread <thread_id> timer status` — 当前计时器状态
- `focus-cli --agent-thread <thread_id> timer start` — 开始专注
- `focus-cli --agent-thread <thread_id> timer pause` — 暂停
- `focus-cli --agent-thread <thread_id> timer skip` — 跳过当前阶段
- `focus-cli --agent-thread <thread_id> stats today` — 今日专注时长/轮数
- `focus-cli --agent-thread <thread_id> stats week` — 本周每日专注
- `focus-cli --agent-thread <thread_id> stats sessions` — 最近专注会话
- `focus-cli --agent-thread <thread_id> desktop layout` — 桌面布局与快捷方式
- `focus-cli --agent-thread <thread_id> apps now` — 当前前台应用
- `focus-cli --agent-thread <thread_id> apps visible` — 可见应用列表

## 注意

- `debug` 等未列入白名单的命令会被宿主拒绝（返回 `agent CLI denied`）。
- Focus Desktop 未运行时 focus-cli 会提示找不到 `cli.json`；此时请告知用户先启动 Focus Desktop。
- 返回均为 JSON；基于返回字段回答用户，不要编造数据。