---
name: focus-cli
description: Focus Desktop 本地控制面调用技能。当用户要求查询或控制 Focus（计时器、统计、桌面布局、正在运行的应用、Agent 日程）时，使用 focus-cli 命令。
---

# focus-cli：编排 Focus Desktop

Focus Desktop 运行期间提供 `focus-cli` 本地控制面（localhost TCP + token，要求 Focus Desktop 已在运行）。始终携带 `--agent-thread <thread_id>`，以启用宿主的白名单与审计。

## 获取当前线程 id

每次调用前读取 `~/.codex/focus-thread.json`；使用其中 `threadId` 的值。文件不存在时使用 `focus`。命令形式为：

```text
focus-cli --agent-thread <thread_id> <command>
```

## 查询与专注控制

- `ping`
- `timer status|start|pause|skip`
- `stats today|week|sessions`
- `desktop layout`
- `apps now|visible`

## Agent 与日程工作流

先执行 `agent list`，从返回的 Agent 中选择节点目标；需要查看该 Agent 会话时使用 `agent session <agent-id>`。

日程始终先 `workflow list`，按需 `workflow read <id>`。可用命令如下：

- `workflow create --payload <workflow-json>`
- `workflow update <id> --payload <workflow-json>`
- `workflow delete <id>`
- `workflow run <id>`
- `workflow runs <id>`
- `workflow cancel <id>`

创建或更新时，`--payload` 后必须是一个 JSON 文档。下面是最小的无副作用手动日程：工作流本身不绑定 Agent（`characterId` 为空），唯一的 `agent` 节点显式目标为从 `agent list` 取得的 id；将 `<agent-id>`、名称和提示词替换为实际值。

```json
{
  "id": "",
  "characterId": "",
  "name": "临时 Agent 验收",
  "trigger": "manual",
  "guard": "none",
  "enabled": true,
  "nodes": [
    {
      "id": "agent-1",
      "kind": "agent",
      "params": {
        "characterId": "<agent-id>",
        "prompt": "请只用一句中文确认日程验收成功。",
        "wait": true,
        "showResult": true
      },
      "x": 0,
      "y": 0
    }
  ],
  "edges": []
}
```

验证时只创建这种不含桌面、计时、应用或文件副作用的 Agent 手动日程。完成 create、read、update 后执行 `workflow run <id>`，每秒查询一次 `workflow runs <id>`，在有界等待内直到状态为 `success`、`failed` 或 `cancelled`。`success` 后报告结果；`failed` 或 `cancelled` 时如实说明。只在超时或明确清理时执行 `workflow cancel <id>`，随后必须 `workflow delete <id>` 清理临时日程。不要猜测 JSON 或 Agent id，始终读取命令返回的 JSON。

## 注意

- `debug` 等未列入白名单的命令会被宿主拒绝并返回 `agent CLI denied`。
- Focus Desktop 未运行时，focus-cli 会提示找不到 `cli.json`；请告知用户先启动 Focus Desktop。
- 返回均为 JSON；基于返回字段回复用户，不要编造数据。
