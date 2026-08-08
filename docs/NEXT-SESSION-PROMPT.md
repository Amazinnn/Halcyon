# 下一个对话初始化提示词（Focus Desktop）

> 由 2026-08-08 会话收尾生成（v1.10.5 已实现待验收 + Agent 概念 / 工作流冻结讨论定稿）。把下面「提示词」直接粘贴到新对话即可；本文件同时归档在仓库，供压缩后自取。

## 提示词

我是 Focus Desktop 项目的维护者。请先阅读仓库根目录的 `docs/STATUS.md`（当前状态与交接页，单一事实源），再按需查阅 `docs/next-phase.md`（路线）、`docs/requirements-verbatim.md`（需求原话 #1–#66）、`docs/decisions/`（ADR-0001~0019）与 `README.md`。

当前真实状态：**v1.10.5.1 已实现、待用户验收**（三样修复 + 文档固化；需求 #64–#66 + ADR-0019；前序 v1.10.5 需求 #59–#63 + ADR-0018）。M4 工作流引擎 v1 已实现（v1.10.4 工作流 v2 画布 + v1.10.5 收敛为 **7 类节点**：Agent / 显示窗口 / 等待 / 分支 / 专注 / 空闲 / 响铃——气泡与 IF 已彻底移除）。M5（外部 Agent 驱动的角色循环）尚未实施。

### 最新讨论定稿（2026-08-08 晚，必须遵守，勿再推翻）

1. **Agent 概念（M5 方向，仅功能需求，细节 TBD）**
   - 每个宠物（Character）↔ 一个 Agent，可切换。
   - 所有 Agent **共享一个对话框**；切换 Agent 时对话框**上下文被替换**。
   - 每个 Agent **过去一天的上下文被存储**，但对话框 UI 中对话被清空（存储≠显示）。
2. **工作流降级为日程工具（冻结，不再更新）**
   - 工作流**保留 v1.10.5 设计，不再加功能/不再重设计**。
   - 工作流可绑定/不绑定 Agent：不绑定 = 完全机械化的系统工作流（自动运行的日程安排表）；绑定 = 该 Agent 的日程安排表，在特定时机告诉 Agent 做什么——**它不是 Agent 本身**。
   - 已否决（本轮讨论明确不采纳）：Agent 出口+动作双通道、槽值自动锁定可解锁、显示窗口开/关节点、重命名「发送给 Agent」→「Agent」等。
3. **冻结边界 = 修完三样后冻结**：① 存档角色绑定（内容丢失根因）② 连线箭头不显示 ③ 数字框 spinner（Web 默认上下箭头）。

### 三样修复的根因与方向（已勘察定稿，实施时直接照做）

1. **存档角色绑定**：现象=创建内容但未点运行，关闭重开 Focus 后内容“丢失”。根因=DB 中工作流 `character_id=''`（内容其实落库，因角色为空被列表过滤不可见）+ `characters_list` 返回空数组被 `unwrap_or_default()` 吞掉 + 前端 `initialized` 一次性缓存 null 不重试。修复方向：Rust `ensure_characters` 永不静默返回空（store 锁失败用 `into_inner()`；无宠物包确保 char-default）；`workflow_save` 空角色自动挂默认角色；启动 `repair_orphan_workflows()` 把空角色旧数据**挂回默认角色**（用户明确选“挂回找回”，这是一次性数据找回，不算兼容层）；前端 refreshCharacters 空角色重试 3 次（500ms）；toDraft 空角色拦截；顶部「保存中/已保存✓」+ beforeunload flush。
2. **连线箭头**：`markerEnd: "url(#wf-arrow)"` 手写 SVG marker 未渲染进 Vue Flow DOM → 改用 Vue Flow 原生 `MarkerType.ArrowClosed`（`@vue-flow/core` 已安装）。
3. **spinner**：`styles.css` 全局隐藏 `input[type=number]` 的 `::-webkit-outer/inner-spin-button`。

### 下一轮待办（按用户指定顺序）

A. **固化本提示词**：若 `docs/NEXT-SESSION-PROMPT.md` 仍是旧版，先用 .NET WriteAllText（UTF-8 无 BOM）覆盖为当前版本。
B. **落实开发文档**：`requirements-verbatim.md` 追加 #64（“内置 agent 看板 + 工作流不再更新”原话）、#65（Agent 概念整理稿）、#66（修三样后冻结）；新增 `ADR-0019-agent-concept-and-workflow-freeze.md`；#59–#63 状态列后补「已实现（待验收）」；同步 STATUS / next-phase / README。
C. **实施三样修复**（上述方向）→ 三测试 → `launch-focus.cmd rebuild` → 编号手测清单 → push。

### 铁律（AGENTS.md 已写明，务必遵守）

1. 新需求先以原话追加到 `docs/requirements-verbatim.md`，再动手；不改历史条目（仅状态列可后补）。
2. 重要架构决策写 ADR（`docs/decisions/ADR-00XX.md`）。
3. 代码改动后必跑：`cd apps/desktop && npm run build`、`cd apps/desktop/src-tauri && cargo test --lib`、`cd packages/event-schema && npm test`；涉及前端/Rust 交付前必须 `launch-focus.cmd rebuild`，并给编号手测清单让我逐项验收。
4. 提交风格 `feat(…)/fix(…)/docs(…)/chore(…): …`，分阶段提交，保持工作区干净并 push 到 Amazinnn/Halcyon。
5. 不得修改/移动/重编 `local-focus-desktop-agent-design-v0.2.md`。

### 环境注意

PowerShell 管道传中文会变 ?，写中文用 .NET `WriteAllText`（UTF-8 无 BOM）；git push 直连易 reset（Clash 开走代理，否则 `git -c http.proxy= -c https.proxy= push`）；cargo 拉依赖需清代理并设 `NO_PROXY=crates.io,index.crates.io,static.crates.io,github.com,*.crates.io`；本机单显示器（多显示器 N/A）。

### 行为偏好与全局准则

- **绝对不要向后兼容**（#62，全局准则）：旧工作流含 IF/气泡启动即删；唯一例外=用户明确选择的一次性孤儿数据挂回。
- 功能优先，**不要花里胡哨但无用的 UI**；用户心疼电脑，不做长时间压测（受控拖动/重叠验收各几轮即可）。
- 工作流 v1.10.5 语义冻结；M5 Agent 看板细节尚未定稿，先与用户讨论，不默认实施。
- 用户已确认方向：内置 Agent 看板，保留当前工作流设计但不再更新。