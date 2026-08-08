# ADR-0021：环状工作流执行语义（focus/idle/ring 阻塞 + 停止入口 + 响铃正确性）

- 状态：已接受（2026-08-08）
- 关联：需求 #68；ADR-0017/0018（工作流 v2 与 v1.10.5 收敛）、ADR-0020（工作流退化为 Agent 日程工具）

## 背景

用户测试「空闲3秒→响铃」环状工作流：一直响不停（非每 3 秒一次）、无停止按钮、整机卡死重启。systematic-debugging 全面排查执行链路发现多个根因：

1. **focus/idle/ring 节点引擎侧不阻塞**：发一个 `WorkflowSystemAction` 事件就返回 → 环飞快空转，「空闲3秒」形同虚设，且取消无检查点可停。
2. **ringFor 用 setTimeout 排秒**：环重启不清旧 setTimeout、新批叠加 → 破音/回声/音量时大时小（叠加声浪）。
3. **playChime 相对时间戳串音**：两个 oscillator 在相同 `ctx.currentTime` 基准同时触发。
4. **无停止入口**：工作流视图顶部只有「运行」，运行中禁用；`running` 状态由运行结束后的 `runs_changed` 事件复位 → 循环期间永不复位、无法停止。列表侧「停」按钮对未绑定角色工作流不可见。
5. **focus 节点归零误触发 focus_end**：工作流 focus 倒计时归零 → 前端发 `focus:core_state {state:"rest",completed:true}` → 触发 focus_end 触发器工作流 → 环中 focus 意外级联。

## 决策

1. **focus/idle/ring 引擎侧阻塞等待**
   - 节点执行时先发 `WorkflowSystemAction` 事件（前端启动计时/响铃），随后 `sleep_wait(secs, cancel)`：每 100ms 轮询 cancel，取消即返回 Err("已取消") → 运行标记 Cancelled。与 wait 节点同模式。
   - 环「空闲3秒→响铃」从此真实按 3 秒节奏执行；取消可随时打断。

2. **顶部「停止」按钮**
   - 运行中「运行」→「停止」（红色）；点击 `workflow_cancel` + 立即复位 `running` 与节点状态，不依赖 runs_changed 事件。
   - 列表侧「停」保留为第二入口。

3. **响铃正确性**
   - 前端 `ringFor` 改为单次响铃（引擎已阻塞等待时长），不再 setTimeout 排秒。
   - `playChime` 时间戳基准 `ctx.currentTime + 0.01` 微延时，避免双 oscillator 串音；gain 0.1→0.08 防破音。

4. **屏蔽 focus_end 联动**
   - 前端 `workflowDriven` 标记：工作流发起的 focus/idle 倒计时归零时 `startRest(false)`（completed=false），不触发 focus_end 触发器工作流；用户手动操作（startFocus/startRest）清除标记。

5. **触发标签「手动」→「保存」**
   - `TRIGGER_LABELS.manual` 与触发下拉显示「保存」（用户明确：手动读起来像保存按钮）。

6. **不设环迭代上限**
   - 用户决策：不设上限，依赖停止按钮 + 阻塞语义（环节奏真实，不会被误判超限打断）。

## 影响

- 引擎执行语义更真实：focus/idle/ring 现在与墙钟同步（用户可感知的计时）。
- 环状工作流（含「空闲N秒→响铃」循环日程）成为安全、可停止的机械化日程工具（ADR-0020 语义落地）。
- 现有测试 `focus_idle_ring_actions_called` 秒数改为 1 秒（避免阻塞长眠）。
