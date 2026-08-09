# 下一阶段拆分建议（M0 收尾 → M1）

> 对应设计稿 §25 最终交付项「下一阶段拆分建议」。2026-08-05。

## 首轮已完成（任务 A–D）

- 许可证审计（`docs/licenses/audit-v0.md`）、AgentEvent 协议 v1（`packages/event-schema`）、技术 Spike（`apps/desktop`）、Windows 可行性（`docs/architecture/windows-feasibility-v0.md`）、ADR-0001~0004、风险清单、本建议。

## M0 收尾（人工验收清单）

1. 透明窗口（Pet/Music）目视确认；必要时调整圆角/不透明度。
2. Alt+Tab、Win+D、全屏共存、睡眠/休眠恢复逐项手动验证并记录。
3. 200% DPI 下核对 `inner_size` 逻辑/物理映射，按需改用 `PhysicalSize`。
4. 多显示器：外接或虚拟显示器驱动验证窗口位置与 Desktop 覆盖。
5. 连续运行 2 小时观察内存与事件丢包（M0 验收）。

## M1：桌宠与桌面壳层（2026-08-08 现状）

### v1.12 桌面锁后端（2026-08-08 已实现待验收，需求 #70，ADR-0023）
- 隐藏任务栏+桌面图标+禁键（Win/Alt+Tab/Alt+F4/Ctrl+Esc）；focus-cli desktop lock/unlock/status；六层崩溃检测/逃生（panic hook/Drop/watchdog/focus-cli/逃生文件/explorer 重启）；核心+开发期防御双模块（产品期删防御文件）。UI 触发留给专注模式。

### Agent 对话与工作流闭合（2026-08-09 已实现，真实 Codex 验收待执行，需求 #76–#78，ADR-0024）
- 正式桌面路径不自动回退 Mock；聊天极简；工作流固定为统一全量日程列表；仅 `showResult` 的 Agent 节点最终结果可一次性带来源回流到目标 Agent 对话与宠物泡泡。
- **下一门槛**：真实 Codex 与当前宠物完成对话，并经 focus-cli 创建、读取、更新、运行、删除唯一命名的临时工作流；不以 Mock 或模拟结果替代。

### v1.11.3 M5 完善轮（2026-08-08 已实现待验收，需求 #69，ADR-0022）
- 多 Agent 事件隔离（envelope agentId=character_id）；崩溃=下次自动重启（去 fallback）；记住上次 Agent；设置页 Agent 管理（删除连带删工作区/打开工作区文件夹）；系统级输出纪律注入每次 turn。工作流展示语义已由 ADR-0024 收敛。

### v1.11.2 M5 Agent 看板 MVP（2026-08-08 已实现待验收，需求 #65/#67，ADR-0022）
- 宠物=Agent 一对一（DB 0007）；多实例 AgentRuntime；懒生成工作区+AGENTS.md（身份唯一来源）；每日会话旋转（哈希存 Rust + focus-cli agent session/list）；聊天 Agent 下拉；VPN loopback 绕过。

### v1.11.1 环状工作流执行语义修复（2026-08-08 已实现待验收，需求 #68，ADR-0021）
- focus/idle/ring 引擎侧阻塞等待（发事件后 sleep 到时长，100ms 轮询 cancel，取消立即中断）；环「空闲3秒→响铃」真实按 3 秒节奏。
- 顶部「停止」按钮（立即 cancel + 复位 UI）；ringFor 单次响铃 + chime 时间戳/音量修正；屏蔽工作流 focus 触发 focus_end 联动；「手动」→「保存」。

### v1.11 工作流退化为 Agent 日程工具（2026-08-08 已实现待验收，需求 #67，ADR-0020）
- 空角色合法化：save 不挂回 / 删 repair_orphan / list 空串=全部含未绑定；孤儿测试数据删库清理。
- Agent 节点级目标：节点 `characterId` 参数（含 Agent 节点=必然绑定，节点决定调谁）；前端目标Agent下拉。
- JSON 文档 + focus-cli `workflow read/create/update/delete --payload`（Agent 只经 CLI 增删改查 JSON，JSON=唯一真相）；`workflow:changed` 事件广播；白名单放行 workflow 全部子命令。
- 统一全量日程列表与未绑定工作流入口已由 ADR-0024 完成。

### v1.10.5.1 修复轮：文档固化 + 三样修复（2026-08-08 已实现待验收，需求 #64–#66，ADR-0019）
- 存档角色绑定：ensure_characters 永不静默返回空（锁失败 into_inner；无宠物包确保 char-default）；workflow_save 空角色自动挂默认角色；启动 repair_orphan_workflows 孤儿挂回默认角色（一次性找回）；前端刷新空角色重试 3×500ms、toDraft 空角色拦截、保存中/已保存✓ + beforeunload flush。
- 连线箭头：手写 SVG marker 未渲染 → Vue Flow 原生 MarkerType.ArrowClosed。
- spinner：CSS 全局隐藏 number input 上下箭头。
- 定稿：Agent 概念（每宠物↔一个 Agent、共享对话框、切换替换上下文、过去一天存储但 UI 清空）与工作流冻结（不再更新、可绑定/不绑定=日程表），M5 待实施。

### v1.10.5 工作流画布收敛轮（2026-08-08 已实现待验收，需求 #59–#63，ADR-0018）
- 7 类节点（移除气泡/IF）；参数面板词条卡片化 + 零变量；Agent=唯一展示通道 + 输出纪律提示词；自动保存竞态修复；启动 purge 不兼容旧工作流（绝对不向后兼容）。


### v1.10.4 工作流 v2 重设计 + 白框/亮度修复 + 随机播放（2026-08-08 实施中，需求 #49–#58，ADR-0017）
- #49 四周白边：WS_POPUP + 清样式 + 外框重置客户区（ncdelta=0）；#50 亮度中心按客户区。
- 工作流 v2：8 类节点（气泡/Agent/显示窗口/等待/分支/专注/空闲/响铃）；Agent 填空槽；分支=单选多路“选项1..N”；允许成环不限次数+箭头；触发头徽标；无模板、自动保存、三栏 150/210、窗口 6×5、运行记录进设置。
- #58 音乐随机播放（第 4 模式）。
### v1.10.3.1 修复轮：回退窗口层回归 + 工作流网格预览（2026-08-08 已实现待验收，需求 #47/#48，ADR-0015 已回退）
- 回退 SetWindowRgn（#48 白色轮廓源）与隐藏创建+后置 show（#48 尺寸膨胀/格心错位源）。
- #42 改 WebView2 透明背景色 + CSS 圆角；#46 改构建期初始矩形（非折叠窗出生即在最终格位）。
- #47 工作流拖动网格预览（GRID_LABELS 加入 workflow）；新增 scripts/winrect-probe.ps1 客观验收。
### v1.10.3 体验修复轮 v2 + 启动叠窗修复（2026-08-08 已实现待验收，需求 #42–#46，ADR-0015/0016）
- #42：SetWindowRgn HWND 圆角裁剪（#37 CSS 方案无效复开；ADR-0015）。
- #43：桌宠/音乐缩放改最近角点吸附（勾股距离到各候选档右下角选最近，删除位移滑块）。
- #44：工作流/统计 UI 结构收敛（UI Pro Max 全局 skill + gpt-taste 适用规则；不换 Chart.js、不新增依赖）。
- #45：内部页打开自动最近空位避让（restore 前查 occupied；ADR-0016）。
- #46：浮窗隐藏创建、布局就位后再显示（消除启动叠窗闪现）。
### v1.10.2 体验修复轮 + 重叠卡死彻查（2026-08-08 已实现待验收，需求 #35–#41，ADR-0014）
- #35：重叠卡死（09:29:20 AppHangB1）受控取证；position_window 位置操作统一原生 HWND。
- #36 工作流默认 4×3 + 布局压缩；#37 根元素同圆角裁剪（单层框）；#38 音乐尺寸 [3×1,3×3,3×4]；#39 桌宠/音乐缩放位移滑块+最近档；#40 launch-focus.vbs 隐藏启动；#41 统计平滑曲线/0-24 刻度/nearest hover/默认 5×4。

### v1.10.1 拖动卡死修复轮（2026-08-08 已实现待验收，需求 #34，ADR-0013）
- 根因：拖动 poller 每 15ms set_position → 主线程 WebView2 SetBounds（同步 COM RPC）+ grid 预览每 15ms 全屏渐变，偶发等待浏览器进程 28s（AppHangB1 已验证）。
- 修复：拖动移动优先原生 SetWindowPos（SWP_ASYNCWINDOWPOS，绕过 SetBounds）；grid:preview 节流 ≥50ms；POLL_MS 15→24；hang-detector STILL_HUNG 每 3s 取证。开发侧受控拖动 2 轮 0 HUNG。

### v1.10 修复轮（2026-08-08 已实现待验收，需求 #30/#31/#32/#33）
- 工作流入口：移到最左侧视图托盘；+ 菜单移除「内部页」；迁移 0006 清理 internal 卡片。
- 快速开关窗口卡死：去冗余窗口操作（restore/collapse/topmost/raise/position 去重与节流）+ 前端 150ms 防抖 + `scripts/hang-detector.ps1` 独立检测。
- 宠物更换失败 + canvas tainted：spritesheet 改同源加载（pet_sheet_data + createImageBitmap），applyEdgeFade try/catch 兜底。
- 启动卡死同步检测（#33）：launch-focus.cmd monitor 启动应用并同步开启 scripts/hang-detector.ps1（HUNG 时抓 minidump + 记录窗口标题/句柄，恢复时记录时长）。

已完成（v1.5–v1.7.2）：
- 桌面快捷方式与图标自由布局（v1.5）：应用/文件/文件夹/URL/内部页，真实图标提取，12×8 网格吸附 + DB 持久化。
- Pet Pack 导入与精灵图播放（v1.7，ADR-0009）：吸收 hatch-pet 产物（pet.json + spritesheet.webp，8×9 / 192×208），文件夹导入 + 校验（尺寸 + 透明背景）+ 持久化 + 四尺寸。
- 桌宠 UX（v1.7.1，ADR-0010）：纯精灵图布局 + hover 对话按钮、缩放网格预览 + 冲突回弹、修复拖拽移动、淡化开关、外置 pet-builder skill。
- 交互修复（v1.7.2）：文件夹快捷方式经 explorer.exe 直开、视图按钮点击式展开、缩放回归修复（overlay WS_EX_NOACTIVATE）。

剩余（下一迭代候选）：
- **托盘**：`TrayController` 基础托盘菜单（显示/隐藏面板、开始/停止专注）。全局快捷键绑定**暂缓**（需求 #19：所有开发完成前不绑定任何快捷键）。
- 验收：先完成 ADR-0024 的真实 Codex 对话与工作流闭环；重启后布局恢复；宠物状态切换流畅。

## 后续里程碑（保持 M2→M7 顺序）

- M2：时间统计 + 本地音乐播放器——**已实现**（v1.8 统计真实化 + v1.9 本地音乐播放器，ADR-0011）；M3：Codex CLI 嵌入（Claudian 式；2026-08-06 已实现垂直切片：对话 + focus-cli 白名单审计 + skills 透传，ADR-0007；plan mode/Diff/终端面板/Claude Code 为后续，**2026-08-08 冻结**「以后未必做」）；M4：内置工作流引擎（精简 n8n；2026-08-07 方向锁定 #26/#28/#29，v1.10.5 已实现；2026-08-08 冻结（#64/#66，ADR-0019）并**v1.11 退化为 Agent 日程工具**（ADR-0020），**v1.11.1 修复环状执行语义**（ADR-0021））；M5：新的 Agent（ADR-0022 MVP/完善已落地，ADR-0024 闭合真实 Provider、极简聊天、统一日程与最终结果回流；真实 Codex 验收为当前门槛）；后续顺延：监督与软限制、实验性虚拟桌面（C# helper）、多 Agent 生态。
- M1 与 M2 模块边界清晰，可部分并行；M3 依赖 M1 的对话面板可用性。

- M4/M5 方向见 docs/requirements-verbatim.md #26/#27；M4 已冻结并退化为 Agent 日程工具（#64/#67/#68，ADR-0019/0020/0021）；M5 Agent 概念已定稿并完成 ADR-0024 所界定的闭合实现，当前只等真实 Codex 验收。
