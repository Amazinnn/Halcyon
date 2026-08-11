# Focus Desktop 当前状态（压缩交接页）

> 更新：2026-08-11（需求 #86/#87，ADR-0028）。Skill 选择现在是聊天输入中的可见 `$skill-name` 原子字段，发送时按用户看到的完整字符串直通真实 Provider；Focus 不再读取或注入 `SKILL.md`。Agent 消息作者显示当前宠物名。Claude Windows 子进程使用 `CREATE_NO_WINDOW`，保留常驻 stream-json 和 session resume。拖动浮窗时委托 `WM_ERASEBKGND`，并在拖动开始/结束重申无边框/无激活约束。自动测试与构建已通过；移动后的蓝白条视觉、Skill 交互和 Claude 无控制台仍待人工验收，详见 [本轮 Eval](./evals/2026-08-11-float-drag-and-skill-checkpoint.md)。

> 更新：2026-08-10（需求 #84/#85，窗口回归修复）。浮窗折叠改用异步原生隐藏，release UI Automation 已确认对话窗口真实隐藏；恢复窗口先原子决定空闲槽位，满格时保持折叠并在主界面提示，避免任何重叠。#83 同步补正跨午夜聊天轮换与每周参数验证。`window-style-probe.ps1` 显示所有内部宿主无 caption/thick frame、未异常激活；淡蓝条需要用户视觉复验，不能写作已验收。后续移动回归见最新 [窗口/Skill Eval](./evals/2026-08-11-float-drag-and-skill-checkpoint.md)。

> 更新：2026-08-10（需求 #83，ADR-0026/0027）。Claude 现为按桌宠常驻的 stream-json Provider；重启后当天首轮 `--resume`，可见聊天消息按“桌宠 x Provider x 本地日期”回放。聊天去除生命周期噪音，Skill 选择和用户输入的可见拼接语义由 ADR-0028 修正。工作流画布增加不持久化的触发节点，计划支持间隔、每日和每周。浮窗统一走无激活原生路径；淡蓝条事故仍为 `Fixed pending verification`，不宣称视觉修复已通过。
>
> 上一轮自动证据保留在 [2026-08-10 快照](./evals/2026-08-10-conversation-continuity-checkpoint.md)；本轮新增测试、构建和 Claude 隐藏控制台证据保留在 [2026-08-11 快照](./evals/2026-08-11-float-drag-and-skill-checkpoint.md)。真实 Provider、浮窗移动后的视觉和桌面锁仍按 Eval 标为 Pending。

> 更新：2026-08-10（Claude Code 作为第二个真实 Provider 已接入；需求 #79/#80，ADR-0025）。每个桌宠在设置页固定选择 Codex 或 Claude；Focus Demo Pet 的历史迁移使用 SQLite 原子标记，按“桌宠 x Provider”隔离当日 session。聊天窗口不提供 Provider 切换。
>
> 本轮修复：切换当前桌宠的 Provider 后立即清空 UI 会话，防止将 Codex thread 交给 Claude（或反向）恢复；`agent:status` 带 `characterId`，非当前桌宠的状态不会覆盖当前聊天；设置页失败切换会回读持久 Provider。
>
> 已验证：`npm run build`、`cargo test --lib`（158）、`packages/event-schema` 测试。实机控制面已用 `Focus Demo Pet` 的真实 Claude 完成一次成功运行，并完成临时工作流的创建、读取、更新、运行、删除，删除后列表为空；不得以 Mock 代替。仍待人工确认聊天窗口的直接发送、来源消息与宠物气泡视觉回流，以及桌面锁和浮窗的 Windows 手工回归。详见 [Eval 检查点](./evals/README.md) 与 [本轮快照](./evals/2026-08-10-claude-provider-checkpoint.md)。

> 更新：2026-08-10（Claude 已成为第二个真实 Provider；#79/#80 已实现，#81/#82 建立文档与每轮 Eval 更新约束）。新对话请先读本页，再按需查阅 next-phase / requirements / ADR、Eval 与生产事故台账。
> 远程：github.com/Amazinnn/Halcyon（private，main）；本地 D:/Projects/Focus。

## 项目一句话

本地专注桌面 + Agent 桌宠系统（Windows 优先，MIT）。技术栈：Tauri 2 + Vue 3 + TypeScript + Rust + SQLite（apps/desktop）；AgentEvent 协议 v1（packages/event-schema）。

## 当前实现与待验收清单
- v1.12.7（对话连续性、常驻 Provider 与计划触发器，已实现待验收，需求 #83）：Claude 在 Focus 生命周期内按桌宠常驻；同日可见聊天按桌宠 x Provider 隔离回放，应用重启后首轮恢复当天 session；聊天仅显示消息、短暂连接/生成状态和真实错误。Skill 通过 ADR-0028 以可见 `$skill-name  text` 直通用户输入，不再注入 `SKILL.md`。工作流画布固定不可持久化的触发节点，定时支持间隔、每日和每周（周一 0 至周日 6 + 本地 `HH:MM`）；不增加执行图节点。调度在写入运行记录前原子领取工作流，避免 tick 重入遗留重复 `running` 记录。浮窗恢复、移动、缩放、置顶统一为无激活原生路径，扩展样式探针采集宿主/子窗口与前台状态。见 ADR-0026/0027/0028 与最新 Eval。
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

- 需求原话：docs/requirements-verbatim.md（#1–#87，只追加、不改历史原话）。
- ADR：docs/decisions/ADR-0001~0028（0012=M4 工作流引擎；0017=工作流 v2；0018=画布收敛；0019=Agent 概念+工作流冻结；0020=工作流退化为 Agent 日程工具；0021=环状工作流执行语义；0022=M5 Agent 看板；0023=桌面锁；0024=Agent 对话与工作流闭合；0025=Claude Code Provider；0026=可见聊天历史与常驻 Provider；0027=工作流触发节点与每周计划；0028=可见 Skill 用户输入）。
- 设计稿：local-focus-desktop-agent-design-v0.2.md（权威，保持原样、不移动、不改章节编号）。
- 质量：[docs/evals/README.md](./evals/README.md)（长期检查点）与 [docs/production-incidents.md](./production-incidents.md)（生产事故台账）。
- 其它：README.md（版本摘要）、docs/next-phase.md（路线）、docs/architecture/（spike/风险/可行性）。
## 质量检查点状态（2026-08-11）

- #74 已实现待验收：桌面锁串行化、启动时无状态 Shell 恢复、浮窗无激活显示，以及 Agent 初始化竞态修复。
- 原生浮窗验收可运行 `scripts/window-style-probe.ps1`；结果应无 caption/thick frame，且内部浮窗不应为前台窗口。#84 的原生/自动化检查已通过，淡蓝条视觉复验仍为 `Pending`；#85 的恢复重叠回归已在 release UI Automation 中通过。
- 本轮窗口拖动、可见 Skill 输入与 Claude 隐藏控制台证据已写入 [2026-08-11 快照](./evals/2026-08-11-float-drag-and-skill-checkpoint.md)；移动后的蓝白条视觉、Skill 原子删除、真实 Provider 输入、Claude 无控制台和桌面锁手工回归保持 `Pending`，不得写成通过。

## Agent 对话与工作流验收状态（2026-08-10）

- 实现与自动测试已完成；真实 Claude 控制面已完成一次对话和临时工作流 create/read/update/run/delete 闭环。聊天窗口直接发送与视觉回流仍须手工验收；Codex 认证状态不作为 Claude 验收替代条件。
- 该工作流仅含目标为当前宠物的 Agent 节点、无桌面副作用；应得到一次合理真实回复、一次「日程 · 名称」来源消息与宠物泡泡、自动刷新的列表、成功运行记录，最后确认临时数据已删除。Provider 不可用、Mock 回退、重复消息或清理失败均不通过。
