# ADR-0020：工作流退化为 Agent 日程工具（空角色合法化 + 节点级目标 + JSON 文档/CLI 通道）

- 状态：已接受（2026-08-08）
- 关联：需求 #67；推翻 ADR-0019 第 4 条（孤儿挂回）；ADR-0006/0007（CLI 控制面与白名单）、ADR-0017/0018（工作流 v2 与 v1.10.5 收敛）

## 背景

v1.10.5.1 交接待办 C（三样修复）已实现并 push，但 2026-08-08 晚 grill 定稿推翻了 #66 的「空角色挂回默认」决定。用户明确：工作流是**任务单位**而非对话预约；不绑定 Agent 的工作流是合法的机械化日程表；Agent 是 Boss，可自主创建/编写/修改/删除工作流（通过高度结构化文档）；「绑定/不绑定」只是事后归类，不是字段、不进 UI。

## 决策

1. **空角色合法化（推翻 ADR-0019 §4）**
   - `character_id=''` 是合法的「不绑定 Agent」工作流：保存不挂回、列表可见、引擎照常执行（调度器本就不按角色过滤）。
   - 移除启动 `repair_orphan_workflows()` 与 `rebind_orphan_workflows()`；既有孤儿测试数据直接删库清理，不留任何迁移/修复代码（#62 不向后兼容保持）。
   - `workflow_list(character_id)`：空串 = 全部（含未绑定），非空仍按角色过滤。

2. **Agent 节点级目标**
   - Agent 节点新增参数 `characterId`（目标 Agent）：节点显式指定调谁，缺省/空 = 工作流 `character_id`。
   - 含 Agent 节点 = 必然绑定（由节点决定，可「某个或某些」）；无 Agent 节点 = 纯机械化日程。
   - 引擎 `AgentCall` trait 新增 `resolve_character(id) -> Option<CharacterInfo>`；engine 保持 Tauri-free。

3. **JSON 文档 + focus-cli 通道（M5 地基）**
   - 工作流的结构化文档 = `WorkflowDef` 完整 JSON（camelCase，与画布同一 wire 格式），JSON 是唯一真相来源，画布只是渲染/编辑器。
   - focus-cli 新增：`workflow read <id>`、`workflow create --payload <json>`、`workflow update <id> --payload <json>`、`workflow delete <id>`（复用现有 `save_workflow`/`delete_workflow`/`validate_workflow`；走 TCP+token，无 PowerShell 管道编码问题）。
   - 白名单（ADR-0007）新增 `workflow list/read/create/update/delete/run/runs/cancel`——Agent 是 Boss，可管理自己的日程表（推翻 ADR-0012 的 anti-loop 注释）。

4. **变更事件**
   - `CoreEvent` 新增 `WorkflowChanged { action, workflow_id }` → `workflow:changed`，save/delete 成功后广播。前端监听 M5 做。

5. **信息分配**
   - 对话框 = 对话 + 标注来源的工作流最终结果（一条消息）；中间采集/整合/流式过程不进对话框，留在自动化线程可追溯。
   - 本次只落后端语义；前端展示形态 M5 再做。

6. **前端从简**：本次前端只加 Agent 节点「目标Agent」下拉（列出角色 + 空=工作流默认）；不加筛选器/文字。

## 影响

- ADR-0019 §4（孤儿挂回）被本 ADR 推翻；ADR-0019 §2 冻结边界不变。
- 前端工作流视图目前按角色下拉筛选，未绑定工作流暂不可见（后端已就绪，M5 统一改列表形态）。
- M5 Agent 看板可直接复用：CLI CRUD + `workflow:changed` 事件 + 节点级角色。
