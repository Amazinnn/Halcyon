# Focus Desktop 生产事故台账

## 使用规则

本台账登记已经在真实 Focus 使用中出现的崩溃、卡死、黑屏、核心功能失效、数据错误和明显的生产视觉回归。功能愿望、未发生的设计风险和普通美化不入册。

一条记录代表一个可归因事故；同一症状的复发写入该记录的时间线。新问题先追加原话到 `requirements-verbatim.md`，再新增或重开这里的记录；修复后把测试、人工验收和对应 Eval 快照补入同一条记录。

| 严重性 | 含义 |
| --- | --- |
| S1 | 桌面不可用、数据明显损坏或系统级阻断。 |
| S2 | 专注、Agent、工作流或窗口等核心能力不可用。 |
| S3 | 明显影响使用或可信度的视觉/交互回归，但存在可继续工作的路径。 |

状态只能为 `Open`、`Fixed pending verification`、`Verified` 或 `Accepted limitation`。`Fixed pending verification` 不等于用户验收通过。

## 统计（2026-08-10 基线）

| 维度 | 统计 |
| --- | --- |
| 总事故数 | 12 |
| 状态 | Open 0；Fixed pending verification 9；Verified 2；Accepted limitation 1 |
| 严重性 | S1 2；S2 8；S3 2 |
| 类别 | Window 4；Automation 1；Data 1；Launch 1；Pet 1；Desktop lock 2；Agent/workflow 2 |
| 缺少自动回归覆盖 | 3（INC-001、INC-002、INC-005）；其余为自动或部分自动覆盖，仍可能要求 Windows 手工验收。 |

## 记录

### INC-001 浮窗非客户区、白边与淡蓝标题条复发

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S3 / Fixed pending verification |
| 首次报告 | 2026-08-06，需求 #5；后续 #49、#71、#74 复发。 |
| 影响与复现 | 内部浮窗重新出现普通窗口标题区、白边或激活时的淡蓝条；移动、隐藏时曾伴随异常。 |
| 根因证据 | 非客户区样式、Tauri 尺寸路径和窗口激活路径曾分别遗留边框或 caption 高亮；见 ADR-0015、`157d173`、`7e97c1c`、`aef2512`。 |
| 修复 | 清除完整边框样式、以 `WS_POPUP` 显示；尺寸和显示路径使用 `SWP_NOACTIVATE` / `WS_EX_NOACTIVATE`。 |
| 验证与回归 | `scripts/window-style-probe.ps1` 和连续窗口交互手测；当前无自动 Windows 风格回归。关联 Eval：`evals/2026-08-10-claude-provider-checkpoint.md`。 |

### INC-002 频繁打开或点击内部窗口导致 Focus 卡死

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S2 / Fixed pending verification |
| 首次报告 | 2026-08-06，需求 #12；2026-08-08 需求 #31 再次报告。 |
| 影响与复现 | 连续双击快捷方式或反复打开内部页时，对话、统计、音乐等窗口可能卡死，用户需要结束进程。 |
| 根因证据 | 窗口恢复、置顶、定位等重复操作在高频点击下堆叠；修复轮记录于 STATUS 的 v1.10。 |
| 修复 | 去重窗口操作、前端 150ms 防抖、独立 hang detector 和 `launch-focus.cmd monitor`。 |
| 验证与回归 | 受控连续打开/关闭仍待人工复验；无针对高频窗口序列的自动回归。 |

### INC-003 拖动或重叠浮窗触发 AppHangB1

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S2 / Fixed pending verification |
| 首次报告 | 2026-08-08，需求 #34/#35。 |
| 影响与复现 | 拖动 chat/stats/music/workflow 或让浮窗重叠时，Focus 挂起约 28 秒，Windows 可能关闭应用。 |
| 根因证据 | WER AppHangB1 与 COM RPC 等待表明每 15ms 的 WebView2 `SetBounds` 可阻塞主线程；见 ADR-0013、ADR-0014。 |
| 修复 | 移动改为原生 `SetWindowPos(SWP_ASYNCWINDOWPOS)`，预览节流至至少 50ms，poll 频率降为 24ms。 |
| 验证与回归 | Rust 覆盖 grid/节流纯函数；受控拖动和重叠曾 0 HUNG。仍须按 Eval 做真实窗口交互回归。 |

### INC-004 工作流循环空转、响铃叠加并导致系统级卡死

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Automation / S1 / Fixed pending verification |
| 首次报告 | 2026-08-08，需求 #68。 |
| 影响与复现 | 「空闲 3 秒 → 响铃」循环快速空转，声音叠加且无停止入口；一次报告中整个屏幕冻结并由 Windows 重启。 |
| 根因证据 | focus/idle/ring 节点不阻塞、无取消入口、`setTimeout` 累积响铃；见 ADR-0021。 |
| 修复 | 引擎侧阻塞等待并 100ms 轮询取消，新增停止按钮，响铃单次化。 |
| 验证与回归 | workflow engine 单测覆盖阻塞、取消和循环逃逸；真实长时循环仍待人工确认。 |

### INC-005 启动浮窗短暂堆叠及极少数启动卡死

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S3 / Accepted limitation |
| 首次报告 | 2026-08-07，需求 #17。 |
| 影响与复现 | 启动时浮窗先堆叠再归位；极少数情况下启动卡死，和 Agent 运行时探测相关。 |
| 根因证据 | 用户接受暂不处理；现有记录不足以确认单一根因。 |
| 修复 | 后续隐藏创建和构建期初始矩形降低闪现，但不把它写作对稀有启动卡死的已验证修复。 |
| 验证与回归 | 无自动回归；保留为接受的限制，若再次出现则重开。 |

### INC-006 专注完成但统计窗口全部为 0

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Data / S2 / Verified |
| 首次报告 | 2026-08-07，需求 #20。 |
| 影响与复现 | 已完成专注和休息后统计仍为 0，用户无法验证数据是否真实落库。 |
| 根因证据 | 单实例竞争、SQLite 锁等待和 UTC 归属造成环境性丢记录；STATUS v1.8.1 有实证。 |
| 修复 | `CreateMutexW` 单实例、SQLite `busy_timeout(5s)`、失败打点和时区修正。 |
| 验证与回归 | storage dashboard 聚合与会话记录单测；需求 #20 后续反馈确认无异常。 |

### INC-007 文件夹快捷方式无法打开

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Launch / S2 / Verified |
| 首次报告 | 2026-08-07，需求 #15。 |
| 影响与复现 | 为“下载”创建的快捷方式弹出“找不到应用程序”。 |
| 根因证据 | 本机 Downloads shell GUID 异常；直接 Shell 路径打开失败。 |
| 修复 | 文件夹统一经 `explorer.exe` 直接启动。 |
| 验证与回归 | 需求 #15 已验收；shortcuts 单测覆盖文件夹与应用识别。 |

### INC-008 更换桌宠时 Canvas 被跨域污染

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Pet / S2 / Fixed pending verification |
| 首次报告 | 2026-08-08，需求 #32。 |
| 影响与复现 | 更换宠物失败，桌宠窗口显示 `SecurityError ... CanvasRenderingContext2D has been tainted by cross-origin data`。 |
| 根因证据 | 精灵图经跨源加载后调用 `getImageData`；STATUS v1.10 记录。 |
| 修复 | 改同源 `pet_sheet_data` + `createImageBitmap` 加载；边缘淡化失败不阻断显示。 |
| 验证与回归 | pet 包格式与透明背景单测覆盖；真实更换桌宠仍待人工复验。 |

### INC-009 桌面锁后任务栏/桌面持续隐藏并出现黑屏

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Desktop lock / S1 / Fixed pending verification |
| 首次报告 | 2026-08-09，需求 #72/#73。 |
| 影响与复现 | 结束或强杀 Focus 后任务栏不恢复；甚至只剩新窗口亮屏，其余桌面区域全黑。 |
| 根因证据 | watchdog 独立进程的 `LOCKED` 初值为 false，旧 `unlock` 空操作；早期 Drop guard 生命周期和 watchdog 等待方式也不可靠。见 ADR-0023、`aa8d489`、`db619d9`。 |
| 修复 | 增加无状态 Shell 恢复入口，启动/退出/跳过和 watchdog 共用；锁状态转换串行化。 |
| 验证与回归 | desktop_lock Rust 单测覆盖无本地锁状态恢复和幂等转换；正常结束与强杀 Shell 恢复仍待 Windows 手工验收。 |

### INC-010 Agent 已存在但聊天无法选择或发送

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Agent/workflow / S2 / Fixed pending verification |
| 首次报告 | 2026-08-09，需求 #71/#74。 |
| 影响与复现 | 对话窗口要求选择 Agent，但下拉没有选项，发送提示“Agent 不存在/未创建”。 |
| 根因证据 | WebView 创建与 `ensure_characters()` 的启动竞态，以及空列表被静默保留。 |
| 修复 | WorkflowManager 与角色初始化前置；前端有限重试、空态和发送前校验。 |
| 验证与回归 | 前端 Agent store/聊天测试覆盖角色选择与状态隔离；真实聊天直接发送仍待人工验收。 |

### INC-011 有效专注期间 Win 键可漏出搜索界面

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Desktop lock / S2 / Fixed pending verification |
| 首次报告 | 2026-08-09，需求 #74。 |
| 影响与复现 | 开始专注后按 Win 仍可唤起搜索，标准锁机语义失效。 |
| 根因证据 | 连续点击时旧 unlock 请求可与新 lock 请求乱序，造成键盘钩子状态提前解除。 |
| 修复 | 前端和 desktop_lock 统一经过串行锁状态转换；暂停完整解锁，恢复按本轮模式重新锁定。 |
| 验证与回归 | focus mode 与 lock queue 前端测试、desktop_lock Rust 状态机测试；键盘组合仍待真实 Windows 验收。 |

### INC-012 Agent/工作流真实 Provider 闭环未被证明

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Agent/workflow / S2 / Fixed pending verification |
| 首次报告 | 2026-08-09，需求 #77/#78。 |
| 影响与复现 | 仅有默认 Agent 和模拟/自动化覆盖时，无法确认宠物能真实对话、分配工作流并返回正确结果。 |
| 根因证据 | 原实现缺少真实 Provider admission；Mock 成功不构成产品证据。见 ADR-0024/0025。 |
| 修复 | 正式路径禁用 Mock 自动回退；接入 Claude Runtime、Provider 隔离 session 与最终结果回流。 |
| 验证与回归 | 真实 Claude 已完成一次合理回复及 `focus-cli` 工作流 CRUD/运行/删除闭环；聊天窗口来源消息和宠物气泡仍待人工验收。 |
