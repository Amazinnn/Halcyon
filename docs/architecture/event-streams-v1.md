# 事件流与订阅矩阵 v1（extensibility plan C3）

Date: 2026-08-14
Status: 随 C3 变更交付；新事件/新监听必须同步本表

## 事件命名空间与领域分组

所有核心事件经 Rust 事件总线（event_bus.rs，ADR-0002）以 Tauri 事件广播。
每个事件映射到一个 Domain（`CoreEvent::domain()`）：Focus / Stats / Agent /
Workflow / Supervision / Pet / Music / Probe / Panel。领域是文档与未来过滤
的边界，不是新机制。

| 事件名 | Domain | 发出者 | 当前监听者 |
| --- | --- | --- | --- |
| focus:tick | Focus | 桌面视图计时器 | topbar、stats 等 UI |
| focus:state_changed | Focus | 桌面视图 | agent store（宠物状态基底）、Rust 引擎（focus:core_state 中继） |
| focus:core_state | Focus | Rust 引擎 | workflow 引擎（focus_end 触发） |
| stats:changed | Stats | Rust 引擎（专注落库后） | stats 视图 |
| agent:event | Agent | Agent 适配器 | chat/desktop（agent store 完整模式） |
| agent:status | Agent | Rust（状态变更广播） | agent store（完整+薄模式） |
| agent:selected | Agent | 前端（切换 Agent） | agent store |
| bubble:requested | Agent | Agent 适配器/工作流结果 | agent store、pet-bubble（独立端点） |
| pet:state_changed | Pet | Rust（宠物状态映射） | agent store（state）、pet 视图 |
| workflow:changed | Workflow | Rust（工作流 CRUD） | workflow store、overview 面板 |
| workflow:runs_changed | Workflow | Rust 引擎 | workflow store、设置页运行记录 |
| workflow:system-action | Workflow | Rust 引擎（focus/idle/ring 节点） | 桌面视图 ui store |
| workflow:agent_result | Workflow | Rust（工作流最终结果） | agent store（完整模式） |
| supervision:status | Supervision | Rust 监督引擎 | topbar、ui store |
| supervision:alert | Supervision | Rust 监督引擎 | 桌面视图（响铃） |
| music:playback_tick | Music | 音乐视图 | Rust 引擎（CoreEvent::MusicTick） |
| probe:recorded | Probe | 活动探针 | （诊断） |
| panel:mode_changed | Panel | （保留） | — |
| settings:acrylic-changed | —（设置域） | Rust 设置命令 | 各窗口玻璃层 |
| window:visibility | —（窗口域） | Rust 窗口管理 | 桌面视图（chatOpen） |

## 薄窗口订阅模式

轻量窗口只初始化自己需要的最小状态，不初始化完整 Agent store
（extensibility plan C3）：

| 窗口 | Agent store 模式 | 订阅集 |
| --- | --- | --- |
| desktop | 完整 | focus:state_changed、cli:timer、supervision:alert 等 |
| chat | 完整 | agent:event、agent:status、workflow:agent_result 等 |
| stats / music / workflow / pet | 完整（保守） | 各自视图所需 |
| topbar | 薄（仅 pet:state_changed） | focus:tick、supervision:status、settings:acrylic-changed |
| pet-bubble | 薄（不 init store，独立端点） | bubble 投递事件（pet-bubble lib） |
| grid-overlay | 薄 | drag:start/drag:end（拖动控制器） |

规则：
1. 新窗口默认薄模式；只有需要字符列表/会话/工作流状态的窗口才用完整模式。
2. 新事件先加进本表（Domain + 发出者 + 监听者）再实现。
3. 事件名保持冒号命名空间；不重命名既有事件（不向后兼容原则之外，事件名是跨窗口契约）。
