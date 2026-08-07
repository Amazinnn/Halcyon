# ADR-0012：M4 工作流引擎（精简 n8n，画布版）

- 状态：已接受（2026-08-07，M4 v1 实施中）
- 关联：需求 #26/#28/#29；设计稿 §25「下一阶段拆分建议」；ADR-0002（事件总线）；ADR-0006/0007（focus-cli 控制面与 Agent 接入）；ADR-0011（M2 本地音乐）

## 背景

用户希望内置"精简 n8n"工作流引擎（#26），以自己喜欢的方式组合工作台。经 2026-08-07 多轮讨论锁定：#28（定时=分钟间隔+每日固定时刻；5 种内部节点；完整数据流；失败即停；自动化线程可见可清理；独立模块+CLI 验收）、#29（可视化画布+多分支；不引入 n8n 代码——n8n 为 Sustainable Use License（fair-code，非 MIT），与项目 MIT 优先冲突；n8n 画布本身基于 Vue Flow（MIT），直接复用同源库）。

## 决策

1. **领域模型**：角色（Character）= 桌面宠物 + agents.md（persona，绑定）；工作流 = 独立资产（按宠物分栏、可复制/迁移，复制自动重绑目标角色、迁移=复制+删源）。Agent 节点执行采用**一次性上下文**：每次执行新开线程，注入角色 persona + 节点提示词，不做跨调用会话/上下文维护。
2. **执行器**：独立 Rust 模块 workflow_engine（不依赖 Tauri），节点+边执行，run 内数据流（{{nodeId.field}} 引用，run 结束即弃），手动/定时（间隔或每日 HH:MM）/专注结束/监督告警四类触发，前置守卫（无/专注中/休息中/空闲中），防重入，**失败即停**；依赖注入（AgentCall/EventSink/WindowOps trait）。
3. **节点集 v1（5 种）**：气泡、发送给 Agent（默认同步等待、超时+可取消、可勾选不等待→无输出）、显示窗口（chat/stats/music/workflow）、等待（≤3600s）、IF（真/假两输出口→多分支）。
4. **编辑器**：Vue Flow（@vue-flow/core，MIT）画布 + 节点/连线拖拽 + 参数面板（含"插入引用"选择器）；新窗口 workflow（固定 4×4 格、无缩放，准则 #23）。模板 3 个（专注结束收尾/定时自检/监督安抚）。
5. **线程呈现**：工作流产生的 Agent 线程记录于 utomation_threads，对话列表可见、带「自动化」徽标、可删除/一键清理（app-server 不支持服务端删除时本地隐藏）。
6. **防循环**：事件触发源只包含专注结束与监督告警，**不包含 Agent 回复**；focus-cli 新增 workflow 命令组仅本地交互、不进 agent 白名单（避免 agent→workflow→agent 循环）。
7. **独立验收**：引擎纯逻辑 cargo 单测；focus-cli workflow list|run|runs|cancel 提供无 UI 触发与查验通道。

## 风险

- Codex app-server 为线程模型：Agent 节点同步等待依赖 	urn/completed 信号（新增 turn_done 广播），并发多工作流同时调用同一 provider 时以 thread_id 关联，无法关联时退化为"等待任一完成"。
- 自动化线程数量随运行增长：v1 提供清理入口，长期需线程回收策略（后续）。
- 完整数据流/画布使 v1 工作量集中在编辑器 UI；引擎逻辑保持独立可测。

## 后果

- 新增 SQLite 迁移 0005：characters / workflows / workflow_runs / utomation_threads。
- CoreEvent 新增 FocusStateChanged / SupervisionAlert / WorkflowRunChanged；supervision 告警与 focus 状态经事件总线。
- 新增 Tauri 命令：characters_list、workflow_list/save/delete/run/cancel/copy/move、workflow_cleanup_threads；focus-cli workflow list|run|runs|cancel。
- 不引入 n8n 代码/依赖；新增前端依赖 @vue-flow/core（MIT），THIRD_PARTY_NOTICES 同步更新。