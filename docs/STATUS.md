# Focus Desktop 当前状态（压缩交接页）

> 当前检查点：需求 #109–#113 / OpenSpec `pet-state-pack-and-settings`。稳定主体校准、包级宽高校正、宠物专属主色宿主、Focus 持续状态和独立回复气泡已实现并进入自动化交付门槛；Windows 比例、tint、气泡避让和既有浮窗回归仍待用户人工验收。按 #113，本轮自动化与 rebuild 通过后先推送 `main`，但不打 tag、不把 Pending 写作通过。见 [本轮 Eval](./evals/2026-08-13-agent-first-pets-and-focus-controls.md)。

> 已关闭：需求 #108 / `INC-020`（Verified / S2）。真实诊断定位到吸附持有 `state.settings` 时气泡定位重入同一把非重入锁；修复把原生定位与气泡跟随移到锁外。用户完成拖动、松手、后续 Focus 点击和再次拖动后报告正常；独立 OpenSpec 变更已进入归档收尾。见 [诊断与修复 Eval](./evals/2026-08-13-pet-drag-post-release-freeze-diagnostics.md)。

> 维修候选：需求 #96–#98 与 ADR-0029。用户已真实确认拖动后的蓝白条、原生 caption 与标题文字已消失；当前只处理原生毛玻璃与网页圆角的轻微弧度差。DWM 圆角偏好保留，网页外层裁切以桌宠为基准由 12px 微调为 10px；本次按用户要求只重建 release，记录见 [维修 Eval](./evals/2026-08-11-float-window-repair-session.md)。

> 维修中：需求 #91 将浮窗拖动后的蓝白边框单独收敛为 [维修档案](./maintenance/float-window-blue-border-repair.md) 和 [本轮 Eval](./evals/2026-08-11-float-window-repair-session.md)。在用户完成真实鼠标拖动验收前，不以探针、脚本移动或截图宣称修复。

> 更新：2026-08-10（需求 #84/#85，窗口回归修复）。浮窗折叠改用异步原生隐藏，release UI Automation 已确认对话窗口真实隐藏；恢复窗口先原子决定空闲槽位，满格时保持折叠并在主界面提示，避免任何重叠。#83 同步补正跨午夜聊天轮换与每周参数验证。`window-style-probe.ps1` 显示所有内部宿主无 caption/thick frame、未异常激活；淡蓝条需要用户视觉复验，不能写作已验收。后续移动回归见最新 [窗口/Skill Eval](./evals/2026-08-11-float-host-ownership-and-stacked-skills.md)。

> 更新：2026-08-10（需求 #83，ADR-0026/0027）。Claude 现为按桌宠常驻的 stream-json Provider；重启后当天首轮 `--resume`，可见聊天消息按“桌宠 x Provider x 本地日期”回放。聊天去除生命周期噪音，Skill 选择和用户输入的可见拼接语义由 ADR-0028 修正。工作流画布增加不持久化的触发节点，计划支持间隔、每日和每周。浮窗统一走无激活原生路径；淡蓝条事故仍为 `Fixed pending verification`，不宣称视觉修复已通过。
>
> 上一轮自动证据保留在 [2026-08-10 快照](./evals/2026-08-10-conversation-continuity-checkpoint.md)；本轮浮窗透明擦除、叠加 Skill、探针和构建证据保留在 [2026-08-11 快照](./evals/2026-08-11-float-host-ownership-and-stacked-skills.md)。真实 Provider、浮窗移动后的视觉和桌面锁仍按 Eval 标为 Pending。

> 更新：2026-08-10（Claude Code 作为第二个真实 Provider 已接入；需求 #79/#80，ADR-0025）。每个桌宠在设置页固定选择 Codex 或 Claude；Focus Demo Pet 的历史迁移使用 SQLite 原子标记，按“桌宠 x Provider”隔离当日 session。聊天窗口不提供 Provider 切换。
>
> 本轮修复：切换当前桌宠的 Provider 后立即清空 UI 会话，防止将 Codex thread 交给 Claude（或反向）恢复；`agent:status` 带 `characterId`，非当前桌宠的状态不会覆盖当前聊天；设置页失败切换会回读持久 Provider。
>
> 已验证：`npm run build`、`cargo test --lib`（158）、`packages/event-schema` 测试。实机控制面已用 `Focus Demo Pet` 的真实 Claude 完成一次成功运行，并完成临时工作流的创建、读取、更新、运行、删除，删除后列表为空；不得以 Mock 代替。仍待人工确认聊天窗口的直接发送、来源消息与宠物气泡视觉回流，以及桌面锁和浮窗的 Windows 手工回归。详见 [Eval 检查点](./evals/README.md) 与 [本轮快照](./evals/2026-08-10-claude-provider-checkpoint.md)。

> 更新：2026-08-10（Claude 已成为第二个真实 Provider；#79/#80 已实现，#81/#82 建立文档与每轮 Eval 更新约束）。新对话请先读本页，再按需查阅 next-phase / requirements / ADR、Eval 与生产事故台账。
> 远程：github.com/Amazinnn/Halcyon（private，main）；本地 D:/Projects/Focus。

> 2026-08-14 / Requirement #114: the pet resize regression is repaired in the
> current OpenSpec change. The canvas now observes the stable pet stage and
> recomputes its CSS and DPR backing dimensions after Vue commits the loaded
> package DOM. Automated validation and release rebuild passed; all Windows
> visual checks remain Pending until user acceptance.

> 2026-08-14 / Requirement #115: successful direct replies can miss the
> independent pet bubble when its WebView restores Agent identity after the
> event. The active `pet-state-pack-and-settings` OpenSpec change now adds a
> 30-second, current-Agent-only memory handoff and one-time claim path; Windows
> visual acceptance remains Pending. See `INC-022` and the latest Eval.

> 2026-08-14 / Requirements #116/#117: Windows repro shows the bubble handoff
> still clears on an identical current-Agent initialization; the active pet
> change now has a red-first correction plus a default-off public-text streaming
> preference and 50%-tinted pet surface. `topbar-capsule-acrylic-clip` is a
> separate active OpenSpec for once-only native pill clipping and global acrylic
> synchronization. Automated gates and release rebuild now pass; Windows visual
> acceptance is the only remaining gate.

> 2026-08-14 retry: the prior visual report contradicted the first checkpoint.
> Current rebuild adds nonzero-client fallback for bubble placement, defers
> topbar region creation until HWND creation is complete, and surfaces the
> global stream switch in chat. These are automated changes only; require a
> fresh Windows report before any closeout.

> 2026-08-14 / Requirement #120: the previous topbar native-composition and
> bubble claim designs are superseded. Topbar is now composition-free with one
> WebView pill; direct bubbles use native ready/render-ack delivery; public
> Codex/Claude text streaming has a no-delta activity state. Automated work is
> in progress and Windows mouse acceptance remains the only authority for
> visible behavior.

> 2026-08-14 / Requirements #121/#122: live CDP instrumentation verified the
> true production root causes. The pet-bubble WebView was missing from the
> Tauri capability window list (every event listen ACL-rejected; the bubble
> host never reported ready), and resident Claude turns stream increments only
> through partial assistant messages (thinking/text/tool_use blocks), which the
> adapter ignored. The capability list now includes `pet-bubble`; the Claude
> adapter consumes assistant messages and streams Provider-visible thinking as
> the new additive `message.thinking` event (user approved showing thinking
> while the streaming switch is on); topbar's transparent host reserves shadow
> margins so the WebView pill shadow renders fully and follows the pill curve.
> ADR-0036 records the new policy. Automated gates and release rebuild pass;
> Windows mouse acceptance is the only remaining gate for both OpenSpec
> changes.


> 2026-08-14 / Requirements #123/#124: two new OpenSpec changes opened —
> `glass-opacity-slider` (global 5-100 glass opacity, 22 = current look, all
> windows at once, content cards stay opaque) and `pet-bubble-sentence-pages`
> (sentence-complete pages: whole short paragraphs, one sentence per page for
> long ones, max 4 lines/page, rotation only for multi-page; content-sized host
> 180-340px wide by lines via native no-activate resize). Automated gates and
> release rebuild pass; Windows mouse acceptance is the only remaining gate and
> both changes stay open.


> 2026-08-14 验收完成：用户逐项通过 Windows 鼠标验收（气泡出现与避让、流式思考、
> 胶囊阴影贴合、透明度滑条全局同步并重启保持、气泡句子级分页与动态尺寸、无回归）。
> 四个 OpenSpec 变更已同步主规格并归档：`2026-08-14-pet-state-pack-and-settings`、
> `2026-08-14-topbar-capsule-acrylic-clip`、`2026-08-14-glass-opacity-slider`、
> `2026-08-14-pet-bubble-sentence-pages`。INC-022 已 Verified。当前无进行中变更。

> 2026-08-14 重构轮（需求 #125，ADR-0037）：第一个变更「窗口注册表」已实现并通过
> 自动化门禁（前端 115 测试、Rust 221 测试、schema、OpenSpec 严格校验、release 重建）。
> Rust `WINDOW_SPECS` 声明表驱动窗口创建（`FLOAT_LABELS` 废除），前端 `ViewRegistry`
> 驱动视图与托盘，capabilities 与注册表一致性由测试守护；行为零变化。OpenSpec 变更
> `2026-08-14-window-registry-declarative` 保持开放，Windows 鼠标验收为唯一剩余门槛。
> 见 [本轮 Eval](./evals/2026-08-14-window-registry-refactor-checkpoint.md)。

> 2026-08-14 扩展性规划（需求 #126/#127）：brainstorming 定稿后产出
> [扩展性总规划 v1](./architecture/extensibility-plan-v1.md)——用户选定场景
> （自定义面板窗口 / Agent 经 focus-cli 扩展 / 多窗口事件协同，桌宠 P2 预留）
> 映射到四领域扩展点与路线图 C1 UI Kit → C2 CLI 命令注册表 → C3 事件流分组+
> 薄窗口订阅 → C4 面板框架，每项为独立 OpenSpec 变更。见
> [规划 Eval](./evals/2026-08-14-extensibility-planning-checkpoint.md)。

> 2026-08-14 扩展性实施（需求 #126/#127/#128，"一把做完"）：C1-C4 全部实现。
> C1 Focus UI Kit（8 控件 + tokens + ui-design/ui-maintenance 文档，4 视图
> 迁移去重）；C2 CLI 命令注册表（COMMAND_SPECS 声明驱动分发/白名单/帮助）；
> C3 事件 Domain 分组 + agent store 薄模式（topbar/pet-bubble/overlay 最小
> 订阅）+ 事件流订阅矩阵文档；C4 overview 概览面板（三处声明 + 只读查询 +
> 事件订阅 + Kit 拼装，托盘 4→5）。前端 129 测试与 build 全绿。**注意：本机
> 有挂起 Windows 更新导致 debug 二进制启动失败（0xc0000139），cargo test
> 与 rebuild 需重启系统后补跑**，随后交付手测清单。见
> [C1-C4 Eval](./evals/2026-08-14-extensibility-c1-c4-checkpoint.md)。

> 2026-08-14 验收反馈轮（需求 #129/#130）：概览面板因与统计窗口重复已移除
> （面板配方保留于 ui-maintenance.md §3）；UI 布局修复——布局最小宽度 tokens
> （输入 96px/下拉 88px/文本行 120px）、Agent 命名输入独占一行、Agent 管理行
> 换行不溢出，ui-design.md 新增布局与文本宽度准则。前端 129 测试、Rust 222
> 测试全绿；rebuild 与手测待完成。见
> [本轮 Eval](./evals/2026-08-14-overview-removal-and-layout-polish.md)。

> 2026-08-14 动态尺寸轮（需求 #131）：输入控件内容自适应（field-sizing:
> content，40px 下限/容器 100% 上限，绝不破框）；多行输入区上限 ~4 行内部
> 滚动；超长显示文本右缘虚化渐隐；对话窗口 Skills 与输入框同高对齐。前端
> 129 测试、Rust 222 测试全绿；rebuild 与手测待完成。见
> [本轮 Eval](./evals/2026-08-14-content-sized-inputs.md)。

> 2026-08-14 反馈修复轮（需求 #132）：修复 Kit 组件 `font: inherit` 覆盖字号
> 的回归（开关/分段回到 12px、下拉 11px）；对话窗口 Skills 下拉固定 88px 宽
> （列表弹层不受影响）；设置弹窗分区间距放宽，五个开关行加极浅分隔线。
> 前端 130 测试、Rust 222 测试全绿；rebuild 与手测待完成。见
> [本轮 Eval](./evals/2026-08-14-ui-feedback-fixes.md)。

> 2026-08-14 死代码清理轮（需求 #133，ponytail）：18 个编译警告全部清零——
> 移除 v1.7 遗留宠物 API 簇、topbar 组合标志、AGENT_ID、claude 冗余字段等；
> 前端 agent store 的 bubble 死状态删除（pet-bubble 独立端点接管）。rust-gate
> 升级：任何 rustc 警告即失败 + clippy correctness 检查（rustfmt 因既有 304
> 处差异暂不强制）。Rust 215 测试、前端 128 测试全绿。见
> [本轮 Eval](./evals/2026-08-14-dead-code-cleanup.md)。

> 2026-08-14 归档收尾：用户逐项验收通过后，9 个 OpenSpec 变更已全部同步主规格
> 并归档（window-registry-declarative、focus-ui-kit、cli-command-registry、
> event-grouping-thin-windows、panel-window-framework、remove-overview-panel、
> ui-layout-polish、content-sized-inputs、ui-feedback-fixes），主规格现有
> 12 个 capability。当前无进行中变更。未打 release tag（版本仍 v1.12.10）。


## 项目一句话

本地专注桌面 + Agent 桌宠系统（Windows 优先，AGPL-3.0-only）。技术栈：Tauri 2 + Vue 3 + TypeScript + Rust + SQLite（apps/desktop）；AgentEvent 协议 v1（packages/event-schema）。

## 当前实现与待验收清单
- v1.12.8（视图托盘稳定性与内嵌 Skill 输入，已验收发布，需求 #99）：视图托盘圆按钮和四个浮窗入口共用前端 in-flight 动作控制器；一次 `restore` 未返回时条目禁用，后续展开/收起/恢复动作被忽略，不再以 150ms 窗口猜测结束。后端以同一可见性操作门拒绝并发 `restore`/`collapse`，返回「窗口操作正在进行，请稍候」，不排队堆积；既有无空位不恢复、零重叠和无激活原生路径保持。聊天输入改为单行 `contenteditable`：Skill 是光标位置的不可拆分内嵌 Token，可连续叠加；相邻 Backspace/Delete 整块删除；发送严格按编辑器顺序直通 Provider，且不读取或注入 `SKILL.md`。自动化、完整构建、release 重建与用户发布验收均完成；发布证据见 [v1.12.8 Eval](./evals/2026-08-11-v1.12.8-release.md)。
- v1.12.7（对话连续性、常驻 Provider 与计划触发器，已实现待验收，需求 #83）：Claude 在 Focus 生命周期内按桌宠常驻；同日可见聊天按桌宠 x Provider 隔离回放，应用重启后首轮恢复当天 session；聊天仅显示消息、短暂连接/生成状态和真实错误。Skill 通过 ADR-0028 以可见 `$skill-name  text` 直通用户输入，不再注入 `SKILL.md`。工作流画布固定不可持久化的触发节点，定时支持间隔、每日和每周（周一 0 至周日 6 + 本地 `HH:MM`）；不增加执行图节点。调度在写入运行记录前原子领取工作流，避免 tick 重入遗留重复 `running` 记录。浮窗由 ADR-0029 规定为一次性宿主配置：透明客户区吞掉 `WM_ERASEBKGND`，其余消息走 Tauri/Windows 默认链路；移动、缩放、置顶只走无激活原生路径。扩展样式探针采集宿主/子窗口与前台状态。见 ADR-0026/0027/0028/0029 与最新 Eval。
- Agent 对话与工作流闭合（已实现，需求 #76–#78，ADR-0024）：正式桌面路径只使用真实 Provider，Provider 不可用直接显示实际错误，Mock 仅供测试注入；聊天仅保留 Agent 选择、连接/生成状态、消息、停止与输入。工作流列表固定展示全部日程，新建日程不绑定 Agent；新 Agent 节点默认当前聊天 Agent、仍可改目标。工作流只在 `showResult` 时向目标 Agent 对话与宠物泡泡各回流一次带「日程 · 名称」来源的最终结果，过程事件不进入聊天；`workflow:changed` 立即刷新列表。真实 Claude 控制面闭环已通过；聊天窗口视觉回流仍待人工验收。
- v1.12.4（桌面锁退出恢复，已实现待验收，需求 #73）：恢复 Shell_TrayWnd/Progman 的入口不再检查当前进程的 `LOCKED`；watchdog 在主进程被强制结束后调用该无状态入口；专注「跳过」和应用内退出先显式解锁。根因：watchdog 是独立进程，其 LOCKED 初值恒为 false，原先调用 unlock 会直接返回，导致桌面宿主持续隐藏。
- v1.12.6（三档专注模式，已实现待验收，需求 #75）：轻度不锁定，标准只拦截 Win/Alt+Tab/Alt+F4/Ctrl+Esc，学霸模式额外隐藏 Shell；当前模式保存到设置，新用户默认标准。专注轮次快照模式；开始、暂停、恢复、跳过、自然结束及连续点击均通过串行转换，暂停/休息在可见状态变化前完成桌面恢复；既有 `focus-cli desktop lock/unlock/status` 仍是严格桌面锁语义。
- v1.12.2（四问题修复，已实现待验收，需求 #71）：① 浮窗浅蓝条——position_window 尺寸路径原生化（SetWindowPos + SWP_NOACTIVATE，杜绝 Tauri set_size 激活画 caption）；② VPN 已解决（v1.12.1 env 合并，验证通过）；③ 「Agent 不存在」——refreshCharacters 空列表重试 3×500ms + ChatView 发送前校验 + selectCharacter 空保护；④ 锁接「开始专注」（startFocus/startFocusFor → desktop_lock；pause/专注结束 → desktop_unlock，新 Tauri 命令）。
- v1.12（桌面锁后端，已实现待验收，需求 #70，ADR-0023）：隐藏任务栏（Shell_TrayWnd）+ 桌面图标（Progman）+ 禁键（Win/Alt+Tab/Alt+F4/Ctrl+Esc，低级键盘钩子）；focus-cli `desktop lock/unlock/status`；六层崩溃检测/逃生（panic hook / Drop / watchdog 子进程 / focus-cli / 逃生文件 / explorer 重启）；模块化：核心 desktop_lock.rs（产品保留）+ 开发期防御 desktop_lock_escapes.rs（产品期删一文件移除）；失败不锁 + 尽力解锁。UI 触发 v1.12.2 已接（专注开始锁/结束解）。
- v1.11.3（M5 完善轮，已实现待验收，需求 #69，ADR-0022）：多 Agent 事件隔离（envelope agentId=character_id，前端按当前角色过滤）；Agent 崩溃=下次自动重启（去掉复杂 fallback）；记住上次 Agent（localStorage）；设置页 Agent 管理（列表/删除连带删工作区/打开工作区文件夹）；系统级输出纪律注入每次 turn。旧工作流的三开关展示语义已由 ADR-0024 收敛为仅 `showResult` 回流最终结果。
- v1.11.2（M5 Agent 看板 MVP，已实现待验收，需求 #65/#67，ADR-0022）：宠物=Agent 一对一（DB 0007：tool/workspace_dir/session_hash/session_date）；多实例 AgentRuntime（每角色一个 Codex 实例，懒构建）；懒生成工作区 + AGENTS.md（身份唯一来源，persona 退役）；每日会话旋转（哈希存 Rust，Agent 经 focus-cli agent session/list 读回看）；聊天顶部 Agent 下拉 + 去 thread 下拉；工作流 Agent 节点选择目标 Agent；VPN loopback 代理绕过（--proxy-bypass-list=<-loopback>）。
- v1.11.1（环状工作流执行语义修复，已实现待验收，需求 #68，ADR-0021）：focus/idle/ring 节点引擎侧阻塞等待（发事件后 sleep 到时长，100ms 轮询 cancel，取消立即中断）；顶部「停止」按钮（立即 cancel + 复位 UI，运行中「运行」↔「停止」互斥）；ringFor 单次响铃（不再 setTimeout 排秒叠加）+ playChime 时间戳/音量修正（防串音/破音）；屏蔽工作流 focus 倒计时归零触发 focus_end 联动（workflowDriven 标记）；触发标签「手动」→「保存」。根因：环飞快空转（focus/idle/ring 不阻塞）+ 无停止入口 + setTimeout 叠加声浪 + 系统卡死。
- v1.11（工作流退化为 Agent 日程工具，已实现待验收，需求 #67，ADR-0020）：空角色合法化（save 不挂回 / 删 repair_orphan / list 空串=全部含未绑定）；Agent 节点级目标（节点 `characterId` 参数 + 前端目标Agent下拉，含 Agent 节点=必然绑定）；JSON 文档 + focus-cli `workflow read/create/update/delete --payload`（Agent 只经 CLI 增删改查 JSON，JSON=唯一真相、画布=渲染器）；`workflow:changed` 事件广播；白名单放行 workflow 全部子命令（Agent=Boss）；孤儿测试数据删库清理（#62 不向后兼容）。推翻 ADR-0019 §4（孤儿挂回）；统一全量日程列表已由 ADR-0024 闭合。
- v1.10.5.1（修复轮，已实现待验收，需求 #64–#66 + 三样修复，ADR-0019）：存档角色绑定、连线箭头改 Vue Flow 原生 MarkerType.ArrowClosed、隐藏数字框 spinner。注意：v1.10.5.1 的「空角色挂默认 / 孤儿挂回」部分已被 v1.11 推翻（改空角色合法化）。
- v1.10.5（工作流画布收敛轮，已实现待验收，需求 #59–#63，ADR-0018）：7 类节点（移除气泡/IF）；参数面板词条卡片化 + 零变量（{{}} 仅引擎内部）；自动保存竞态修复（save 先刷新列表再改 id + selfSave）；启动 purge 不兼容旧工作流（绝对不向后兼容 #62）。旧「返回即展示」语义已由 ADR-0024 收敛为仅允许一条带来源的最终结果。
- v1.10.4（工作流 v2 重设计 + 白框/亮度修复 + 随机播放，已实现待验收，需求 #49–#58，ADR-0017）：#49 WS_POPUP+外框重置客户区消除四周白边；#50 拖动亮度中心按客户区；工作流 v2=8 类节点/Agent 填空槽/分支多路“选项1..N”/允许成环+箭头/触发头徽标/无模板/自动保存/三栏 150/210/窗口 6×5/运行记录移设置页；#58 音乐随机播放第 4 模式。
- v1.10.3.1（修复轮，已实现待验收，需求 #47/#48 + 回退 #42/#46，ADR-0015 已回退）：回退 SetWindowRgn（白色轮廓源）与隐藏创建+后置 show（尺寸膨胀/格心错位源）；#42 改 WebView2 透明背景色 + CSS 圆角；#46 改构建期初始矩形（非折叠窗出生即在最终格位，折叠窗仍隐藏）；#47 工作流拖动显示网格预览（GRID_LABELS 加入 workflow，松手吸附走既有 finalize）；新增 scripts/winrect-probe.ps1 客观验收实际矩形 vs settings 期望（格心重合）。
- v1.10.3（体验修复轮 v2 + 启动叠窗修复，已实现待验收，需求 #42–#46，ADR-0015/0016）：#42 浮窗 SetWindowRgn HWND 圆角裁剪（#37 CSS 方案无效复开）；#43 桌宠/音乐缩放改最近角点吸附（勾股距离到各候选档右下角选最近）；#44 工作流/统计 UI 结构收敛（UI Pro Max 已装全局 + gpt-taste 适用规则）；#45 内部页打开自动最近空位避让（restore 前查 occupied）；#46 浮窗隐藏创建、布局就位后再显示（消除启动叠窗闪现）。

- v1.10.2（体验修复轮 + 重叠卡死彻查，已实现待验收，需求 #35–#41，ADR-0014）：#35 重叠卡死受控取证 + position_window 位置操作改原生 HWND（ADR-0014）；#36 工作流默认 4×3 + 布局压缩；#37 根元素同圆角裁剪消除外层框；#38 音乐尺寸 [3×1,3×3,3×4]；#39 桌宠/音乐缩放位移滑块+最近档；#40 launch-focus.vbs 隐藏启动；#41 统计 line+tension 平滑曲线、0/24 刻度、nearest hover、默认 5×4。开发侧：布局迁移落盘验证通过，重叠复现 3 轮 + 音乐拖动 12s 均 0 HUNG。
- v1.10.1（拖动卡死修复轮，已实现待验收，需求 #34，ADR-0013）：拖动移动优先原生 SetWindowPos（SWP_ASYNCWINDOWPOS，绕过每 tick WebView2 SetBounds 同步 COM RPC）+ grid 预览 50ms 节流 + poll 24ms；hang-detector HUNG 期间每 3s STILL_HUNG 取证；开发侧受控拖动 2 轮 0 HUNG。
- v1.10（修复轮，已实现待验收，需求 #30/#31/#32/#33）：工作流入口改到最左侧视图托盘、去掉 + 内部页并清理 internal 卡片（迁移 0006）；快速开关窗口卡死修复（去冗余窗口操作 + 前端防抖 + scripts/hang-detector.ps1 独立检测）；宠物更换失败与 canvas tainted 修复（spritesheet 同源加载 + 淡化 try/catch）；#33 launch-focus.cmd monitor 同步启动 hang-detector（HUNG 抓 minidump，日志 %APPDATA%\com.focusdesktop.app\hang-detector.log）。

- v1.9.1（音乐窗口尺寸化，需求 #24/#25）：右下手柄 3×1~3×4 离散缩放（网格预览/冲突回弹/持久化）；chat/stats/music/pet 禁用原生拉伸（尺寸唯一由网格控制，准则 #23）；行数≥3 才显示播放列表（3 行→4 首可见、1 行→隐藏）；手柄 setPointerCapture 修复、紧凑 3×1 布局不裁切。
- v1.9（M2 本地音乐播放器，需求 #22）：选定文件夹（记住）扫描 MP3/FLAC/M4A，HTML5 audio + asset 协议（Range seek、运行时 scope 扩展），lofty 标签/封面（回退文件名/渐变），列表+控制条，单曲循环/列表循环/列表顺序三模式（ADR-0011）。
- v1.8.2（专注落库口径，需求 #21）：按墙钟经过时间记录（跳过也记录；分心/空闲时段计入专注；消除 2s 心跳粒度损失）。
- v1.8.1（统计链路加固，需求 #20）：强制单实例（CreateMutexW，重复启动秒退）+ SQLite busy_timeout(5s) + 会话记录失败打点；修复偶发「专注完成但统计全 0」的环境性丢记录。
- v1.8（M2 统计真实化，需求 #18）：统计窗口接真实 SQLite 数据（30 天热力图 / 今日 24h 分布 / 连续天数 / 今日汇总）；新增 focus-cli stats dashboard（白名单+审计）；分心/空闲/音乐类型暂为「暂无数据」占位。
- v1.7.2 交互修复（需求 #15/#16/#17）：文件夹快捷方式经 explorer.exe 直开（绕开本机损坏的 Downloads shell GUID，不再弹「找不到应用程序」）；视图按钮改点击式展开 + 外部点击关闭；桌宠缩放回归修复（overlay WS_EX_NOACTIVATE + pointercancel/lostpointercapture）；网格亮部中心对齐。
- v1.7.1 桌宠 UX（需求 #14，ADR-0010）：纯精灵图布局 + hover 对话按钮、缩放网格预览 + 冲突回弹、修复四尺寸拖拽移动、透明背景校验 + 淡化开关、外置 pet-builder skill。
- v1.7（M1 Pet Pack，需求 #13，ADR-0009）：吸收 hatch-pet 产物（pet.json + spritesheet.webp，8×9 / 192×208 契约），精灵图帧播放器替换几何占位；文件夹导入 + 校验（尺寸 + 透明背景）+ 持久化 + 四尺寸。
- 先前已验证：#12（菜单单行/真实图标/单飞去重/后台线程/release 生产协议）。
- 已知小问题（用户接受暂不处理）：#17 启动初期浮窗短暂堆叠后自动归位；极少数启动卡死（与 Agent 运行时探测相关）。

## 架构速览

- 7 窗口：desktop（全屏主界面：居中 2×5 快捷区 + 临时视图托盘 + 计时 hero + 三键 Dock）、chat/stats/music/pet（12×8 网格浮窗）、grid-overlay（拖拽/缩放预览，输入穿透）、topbar（置顶状态胶囊，点击穿透）。
- 拖拽：Rust 光标轮询（drag.rs，15ms，GetCursorPos + 屏幕钳制）；前端只发 drag_start/drag_end；网格光晕 = 整条网格线沿长度连续渐变（1.5 格衰减，floatRect 驱动）。
- 事件：Rust 事件总线（event_bus.rs）→ Tauri listen/emit；focus:tick、supervision:alert/status、grid:preview、`workflow:agent_result` 等。
- 控制面：focus-cli（localhost TCP + token；timer/stats/desktop/apps；ADR-0006）；M3 Agent 调用走白名单 + 审计（ADR-0007；debug 与未知命令拒绝）。
- DB：SQLite（schema_migrations 迁移；app_shortcuts、ui_layouts、focus_sessions、supervision_events、spike_probes）。
- App data：%APPDATA%/com.focusdesktop.app/（settings.json + pet-packs/<id>/ + spike.db）。
- 桌宠素材：pet-packs/focus-demo-pet（透明背景已重建，validate 通过）；生成通道 ~/.codex/skills/pet-builder（SenseNova）。

## 命令

- 启动/重建：launch-focus.cmd；launch-focus.cmd rebuild 强制重建 release；launch-focus.cmd monitor 启动应用并同步开启卡死检测（scripts/hang-detector.ps1，日志 %APPDATA%\com.focusdesktop.app\hang-detector.log，HUNG 时抓 minidump 到 hangs\）。
- 测试：cd apps/desktop && npm run build；cd apps/desktop/src-tauri && cargo test --lib；cd packages/event-schema && npm test。
- 控制面：apps/desktop/src-tauri/target/release/focus-cli.exe timer status（需应用运行）。

## 已知问题与环境注意

- #17 启动堆叠（用户接受，暂不处理）。
- CDP 调试：启动前设 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222，偶发对新实例不生效（已多方式验证，非产品缺陷）。
- git push：直连易 reset；Clash 开启时走代理，否则 git -c http.proxy= -c https.proxy= push。
- cargo 拉新依赖：清 HTTP(S)_PROXY / ALL_PROXY，设 NO_PROXY=crates.io,index.crates.io,static.crates.io,github.com,*.crates.io。
- PowerShell 管道传中文会变 ?；写中文文件用 .NET WriteAllText（UTF-8 无 BOM）。
- 本机单显示器；多显示器 N/A。

## 下一步候选

- M1 剩余：系统托盘（可做）；全局快捷键绑定**暂缓**（需求 #19，开发完成前不绑）。
- M2：统计真实化（v1.8）已实现；本地音乐播放器（v1.9）已实现。
- M3 剩余：plan mode / Diff / 终端面板；Claude Code Provider 已由 ADR-0025 接入，后续只做按 Eval 的回归维护。
- M4：内置工作流引擎（精简 n8n）——2026-08-07 方向锁定（#26/#28/#29），v1.10.5 已收敛为 7 类节点并实现（ADR-0012/0017/0018）；2026-08-08 起冻结不再更新（#64，ADR-0019），**v1.11 退化为 Agent 日程工具**（ADR-0020：空角色合法化 / 节点级目标 / JSON 文档+CLI 通道），**v1.11.1 修复环状执行语义**（ADR-0021：focus/idle/ring 阻塞 + 停止按钮 + 响铃正确性）。
- M5：新的 Agent（外部 Agent 驱动的角色循环）——2026-08-07 已锁定方向（#27）：内核驱动 / 事件+兜底 / 先单角色；Journal/Task 全家桶保持外接 skill 不内置。2026-08-08 Agent 概念定稿（#65，ADR-0019）：每宠物↔一个 Agent、共享对话框、切换替换上下文、过去一天上下文存储但 UI 清空；v1.11.2/1.11.3 建立宠物=Agent、多实例、工作区、会话、隔离与管理；ADR-0024 已闭合真实 Provider、极简聊天、统一日程列表与最终结果回流；ADR-0025 增加 Claude Provider。当前门槛按 [Eval 检查点](./evals/README.md) 执行，不以 Mock 替代。
- 悬而未决：图标区是否恢复拖动（当前为居中 2×5 固定）；多屏验证；毛玻璃在部分驱动下透明性（FOCUS_NO_ACRYLIC=1 降级开关）。

## 文档索引

- 需求原话：docs/requirements-verbatim.md（#1–#100，只追加、不改历史原话）。
- ADR：docs/decisions/ADR-0001~0028（0012=M4 工作流引擎；0017=工作流 v2；0018=画布收敛；0019=Agent 概念+工作流冻结；0020=工作流退化为 Agent 日程工具；0021=环状工作流执行语义；0022=M5 Agent 看板；0023=桌面锁；0024=Agent 对话与工作流闭合；0025=Claude Code Provider；0026=可见聊天历史与常驻 Provider；0027=工作流触发节点与每周计划；0028=可见 Skill 用户输入）。
- 设计稿：local-focus-desktop-agent-design-v0.2.md（权威，保持原样、不移动、不改章节编号）。
- 质量：[docs/evals/README.md](./evals/README.md)（长期检查点）与 [docs/production-incidents.md](./production-incidents.md)（生产事故台账）。
- 其它：README.md（版本摘要）、docs/next-phase.md（路线）、docs/architecture/（spike/风险/可行性）。
## 质量检查点状态（2026-08-11）

- #74 已实现待验收：桌面锁串行化、启动时无状态 Shell 恢复、浮窗无激活显示，以及 Agent 初始化竞态修复。
- 原生浮窗验收可运行 `scripts/window-style-probe.ps1`；结果应无 caption/thick frame，且内部浮窗不应为前台窗口。#84 的原生/自动化检查已通过，淡蓝条视觉复验仍为 `Pending`；#85 的恢复重叠回归已在 release UI Automation 中通过。
- 本轮窗口拖动、可见 Skill 输入与 Claude 隐藏控制台证据已写入 [2026-08-11 快照](./evals/2026-08-11-float-host-ownership-and-stacked-skills.md)；移动后的蓝白条视觉、Skill 原子删除、真实 Provider 输入、Claude 无控制台和桌面锁手工回归保持 `Pending`，不得写成通过。

## Agent 对话与工作流验收状态（2026-08-10）

- 实现与自动测试已完成；真实 Claude 控制面已完成一次对话和临时工作流 create/read/update/run/delete 闭环。聊天窗口直接发送与视觉回流仍须手工验收；Codex 认证状态不作为 Claude 验收替代条件。
- 该工作流仅含目标为当前宠物的 Agent 节点、无桌面副作用；应得到一次合理真实回复、一次「日程 · 名称」来源消息与宠物泡泡、自动刷新的列表、成功运行记录，最后确认临时数据已删除。Provider 不可用、Mock 回退、重复消息或清理失败均不通过。
## 2026-08-14 Visible pet and topbar rework

Requirements #116-#119 remain active. The earlier automated result did not
match the user's Windows observation and is not treated as visual acceptance.

- `pet-bubble` is now a local delivery endpoint: it listens before bootstrap,
  then claims the 30-second current-Agent envelope without bootstrapping the
  separate chat Agent store.
- The pet surface is an explicit translucent package-tint gradient with blur
  and highlight, while no-package Agents remain hidden.
- Topbar reuses the accepted hidden float-host setup and then applies an exact
  half-height pill region from Tauri physical inner size; it remains outside
  float-label/grid/tray ownership (ADR-0034).

Automated verification and `launch-focus.cmd rebuild` are still required after
this rework. Both OpenSpec changes stay open until user mouse-driven Windows
acceptance reports real bubble, streaming, pet glass, and capsule behavior.

## Current Checkpoint Update (2026-08-11)

The recurring float-frame incident is under dedicated repair. The user has
confirmed that the residual top strip contains the native host title; it is a
Windows caption, not a Focus web component. ADR-0029 now specifies a one-time
direct window procedure during hidden creation: `WM_NCCALCSIZE -> 0`,
`WM_ERASEBKGND -> 1`, and `WM_NCACTIVATE -> 1`; every other message is
forwarded to Tauri's original procedure. The focused test was red before the
activation handling and green after it. Build, 173 Rust library tests, and
schema tests pass; the rebuilt-release manual drag gate remains pending.
INC-001 is Verified: the user has confirmed the original caption symptom is
gone after real dragging. The maintenance document retains the evidence.

Native acrylic corners are now a separate open visual regression (INC-017):
the native composition layer previously escaped the WebView's CSS-rounded
edge. The current release candidate applies the DWM `ROUND` corner preference
once while each float host is hidden at creation. Its focused test and all
automated build gates pass; user visual verification remains required.

The stacked Skill composer is implemented and covered by the 46 frontend
tests: ordered visible tokens, exact `$skill-a  $skill-b  text` input, and one
adjacent token removed by Backspace/Delete at the input boundary. The latest
evidence is [the 2026-08-11 Eval](./evals/2026-08-11-float-host-ownership-and-stacked-skills.md).

## 用户文档与设置说明检查点（2026-08-11）

需求 #100 的 README 与设置说明已纳入 v1.12.9；四张只含 Focus 窗口的演示截图已复制到 README 资产目录。当前发布验证见 [v1.12.9 Eval](./evals/2026-08-11-v1.12.9-agpl-user-guide.md)。

## v1.12.9 Public Release Checkpoint (2026-08-11)

Requirements #100/#101 add the ordinary-user README, four real Focus screenshots,
and an AGPL public-release policy. From v1.12.9 onward Focus Desktop's own source
code is AGPL-3.0-only; v1.12.8 remains the historical MIT tag. The release
metadata is aligned across the desktop package, Rust crate, event schema, Settings
UI, README, LICENSE, and third-party notice. See [ADR-0030](./decisions/ADR-0030-agpl-public-release.md)
and [the v1.12.9 Eval](./evals/2026-08-11-v1.12.9-agpl-user-guide.md). The pet-host
visual follow-up remains outside this release scope.
## v1.12.10 Float Geometry Checkpoint (2026-08-11)

Requirement #102 records a first-open/restore drag regression: the grid glow
could appear about two cells left of the visible float. v1.12.10 replaces the
parallel client/outer geometry paths with one native ClientGeometry snapshot
used by drag preview, snap, and final placement. Caption handling, acrylic,
corners, window sizing, view tray, and zero-overlap policy are unchanged. See
[the v1.12.10 Eval](./evals/2026-08-11-v1.12.10-float-geometry.md) and INC-019.
The user completed the real mouse-drag acceptance gate: glow center, caption/title, overlap, and tray behavior all passed.
> 2026-08-13 当前实现候选：Agent-first pets and focus controls（需求 #103/#104，ADR-0031）。Agent 是主身份，桌宠是其唯一、可选且不可转移的外观；旧全局 pet-packs 启动时迁入 `Focus-Agents/<agent-id>/pet-pack/`，当前 Agent 无包时不显示桌宠或气泡。设置页提供创建、选择、Provider、工作区、宠物导入/删除与 Agent 删除；删除会级联清理引用该 Agent 的工作流，但保留工作区文件。监督应用改为已安装和可见程序的精确选择器，只有黑名单。标准工作阶段隐藏应用内退出，学霸额外隐藏暂停和跳过；后端拒绝语义保持。自动化与重建、真实 Provider、原生气泡和桌面锁人工项仍以本轮 [Eval](./evals/2026-08-13-agent-first-pets-and-focus-controls.md) 为准。

> 2026-08-13 流程接入：需求 #105 已在仓库根目录启用 OpenSpec core。后续行为变更先写入逐字需求，再通过 OpenSpec `explore → propose → apply → archive`；进行中变更在 `openspec/changes/`，归档规格在 `openspec/specs/`。既有 ADR、Eval、事故台账和逐字需求继续保持各自权威职责。本次流程接入证据见 [OpenSpec Eval](./evals/2026-08-13-openspec-adoption.md)。
> 2026-08-13 当前 OpenSpec 变更：`pet-state-pack-and-settings` 的阶段一拖动死锁已人工验证；阶段二/三实现已覆盖官方 Hatch Pet + `focus-hatch-pet`、稳定主体框、DPI-safe contain、包级校正、Focus 持续状态和独立气泡。当前只剩包/气泡的 Windows 人工门槛与最终文档关闭；顶端状态胶囊已登记为下一项独立 OpenSpec 候选，不在本变更中重设计。
