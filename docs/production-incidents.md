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

## 统计（2026-08-14 当前）

| 维度 | 统计 |
| --- | --- |
| 总事故数 | 22 |
| 状态 | Open 1；Fixed pending verification 13；Verified 7；Accepted limitation 1 |
| 严重性 | S1 2；S2 15；S3 4 |
| 类别 | Window 7；Automation 2；Data 1；Launch 1；Pet 4；Desktop lock 2；Agent/workflow 5 |
| 缺少自动回归覆盖 | 1（INC-005）；其余为自动或部分自动覆盖，仍可能要求 Windows 手工验收。 |

2026-08-13 quality update: the pet drag freeze reported after importing a
package is tracked as INC-020. A real release reproduction after the previous
automated ownership change reopened it. The diagnostic reproduction confirmed
a settings-mutex self-deadlock in pet post-placement work; the minimal lock-scope
repair was later accepted by the user and is now Verified. Package calibration,
mapping, and companion visual work remain a separate Pending acceptance scope.

## 记录

### INC-022 Successful direct replies can miss the pet bubble
| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Pet / S2 / Verified |
| 首次报告 | 2026-08-14，需求 #115。 |
| 影响与复现 | 成功直接与 Agent 对话后，桌宠旁没有气泡；聊天窗口打开或关闭均应不影响此行为。 |
| 根因证据 | 静态排查确认 Codex/Claude 成功最终回复发送 `bubble:requested`。独立 `pet-bubble` WebView 有自己的 Pinia store，可能在其完成当前 Agent 初始化前因目标 `agentId` 过滤事件；原生气泡宿主也未复用其他浮窗的隐藏创建配置。完整 Windows 时序仍待本轮验收。 |
| 修复 | 当前 Agent 的 30 秒单条内存投递、一次领取和 `deliveryId` 去重；气泡窗口完成初始化后补领，并在隐藏创建时一次性配置浮窗宿主。工作流与监督提醒不进入此补领缓存。 |
| 验证证据 / 回归覆盖 | 红灯前端去重测试、单次领取/过期 Rust 测试已转绿；90 个前端测试、211 个 Rust 测试、schema、构建、OpenSpec 严格校验和 release rebuild 通过。Windows 人工验收仍 Pending。关联 OpenSpec `pet-state-pack-and-settings` 与本轮 Eval。 |

2026-08-14 follow-up: the user still observed no bubble for a direct reply.
Source tracing found a further lifecycle condition: `pet-bubble` restores the
same current Agent through `agent_set_current`, whose unconditional pending
delivery clear consumes the envelope before its claim. The active fix is scoped
to preserve the envelope for an identical current-Agent write while retaining
real switch/deletion clearing; automated and Windows evidence are pending.

2026-08-14 automated checkpoint: the identical-current-Agent write now retains
the pending delivery, while a real switch/deletion continues to clear it. The
full frontend/Rust/schema/build/OpenSpec/diff/rebuild gate passed; Windows
delivery acceptance remains Pending.

2026-08-14 rework after the failed Windows report: the earlier same-Agent cache
repair was insufficient because `PetBubbleView` still depended on its own full
Pinia Agent-store initialization. The direct reply can arrive before that store
has registered identity/listeners. The repaired endpoint registers the bubble
listener first, reads current identity from bootstrap, and claims the pending
delivery independently; immediate and claimed paths retain `deliveryId`
deduplication. It continues to use the accepted hidden float-host setup and
no-activate positioning path. The user has not yet verified this rework, so
INC-022 remains `Fixed pending verification`.

2026-08-14 Requirement #120 rework: the claim-on-receipt protocol is replaced
by a native Bubble Controller. It retains the direct-reply envelope through
host reload until the matching ready generation confirms rendering and native
placement/show succeeds. Read-only diagnostics now identify queue, ready, ack,
placement, and show state. This is not Windows visual acceptance; INC-022 stays
pending user verification.

### INC-021 Pet spritesheet does not resize with its grid host
| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Pet / S2 / Fixed pending verification |
| 首次报告 | 2026-08-14, requirement #114. |
| 影响与复现 | A loaded pet kept its previous canvas dimensions when the host switched among 1x1, 1x2, 2x1, and 2x2 grid sizes. |
| 根因证据 | The observer was bound after asynchronous package loading and targeted the parent of a `v-if` canvas, which could be absent at binding time. Consequently host resize never invoked `fitCanvas()`. |
| 修复 | Bind one observer to the stable `pet-stage`; after mount and package DOM commit, reconnect it and recompute CSS plus DPR backing dimensions from the same proportional metrics path. Native window, drag, tray, and brightness geometry are unchanged. |
| 验证证据 / 回归覆盖 | A red-first `PetView.test.ts` became green. Full frontend, Rust, schema, OpenSpec, diff, and release rebuild gates passed. Windows visual verification across the four host sizes remains pending. See `docs/evals/2026-08-13-agent-first-pets-and-focus-controls.md`. |

### INC-020 桌宠拖动后 Focus 无响应

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Pet / S2 / Verified |
| 首次报告 | 2026-08-13，需求 #107；2026-08-13 需求 #108 真实复发后重开。 |
| 影响与复现 | 当前 Agent 导入宠物包后，拖动桌宠并松开鼠标表现正常；之后点击任何 Focus 区域会卡死。无包 Agent 还会留下空方框或透明宿主。 |
| 根因证据 | 真实诊断依次完成 `pointerup`、poller 停止、release 领取、overlay 隐藏和 geometry 读取，最后停在 `finalize:snap:start`，未出现 `finalize:snap:complete`。该边界内 `place_window_inner()` 持有 `state.settings` 后调用气泡定位，而气泡定位再次获取同一把非重入 `std::sync::Mutex`，使 Tauri 主线程确定性自锁。 |
| 修复 | 将吸附拆为两段：锁内只计算最终格位并在成功时持久化；锁释放后才定位原生窗口、跟随宠物气泡并置顶 topbar。占用格仍回到原格，不改变移动、overlay、样式、亮度中心、托盘或桌面锁。 |
| 验证与回归 | 红灯测试先因缺少 `resolve_window_placement` 失败；实现后 `successful_placement_releases_settings_before_post_placement_work` 与 `occupied_placement_releases_settings_before_snap_back_work` 均通过。用户在 release 中完成“拖动、松手、点击其他 Focus 区域、再次拖动”后报告正常；完整证据见 `docs/evals/2026-08-13-pet-drag-post-release-freeze-diagnostics.md`，OpenSpec 归档为 `2026-08-13-pet-drag-post-release-freeze`。 |

### INC-001 浮窗非客户区、白边与淡蓝标题条复发

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S2 / Verified |
| 首次报告 | 2026-08-06，需求 #5；后续 #49、#71、#74、#83、#84、#86 复发。 |
| 影响与复现 | 内部浮窗重新出现普通窗口标题区、白边或激活时的淡蓝条；本轮确认窗口初次打开正常，但开始移动后出现蓝白条边框。 |
| 根因证据 | 用户确认五类浮窗首次打开正常、拖动中和松开后均出现蓝白条。原生拖动自 v1.10.1 起一直使用 `SetWindowPos(...SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS)`；真正改变窗口过程语义的是 `c906df5` 将此前一次性的直接过程改为 managed subclass，并在多个生命周期重配。完全移除过程又暴露原生标题栏，证明透明宿主仍需要全客户区处理。 |
| 修复 | 恢复历史的直接 `GWLP_WNDPROC`，但仅在隐藏创建时安装一次。它处理 `WM_NCCALCSIZE`/`WM_ERASEBKGND`/`WM_NCACTIVATE`，其他消息转发原 Tauri 过程；移动、恢复、缩放、置顶继续只走无激活 `SetWindowPos`，不重配过程或样式。 |
| 验证与回归 | 回归测试先在 `WM_NCACTIVATE -> None` 下失败，再在已处理返回后通过；构建、173 项 Rust 测试、schema 测试和 release 重建通过。用户已真实拖动确认不再显示标题条或标题文字。关联 `maintenance/float-window-blue-border-repair.md` 与 `evals/2026-08-11-float-window-repair-session.md`。 |

时间线更新（2026-08-11，需求 #92）：用户在当前 release 确认所有五类浮窗初次打开正常，但拖动中及松开后均出现上端蓝白条。自动探针、脚本移动和截图均不再作为视觉修复依据；维修范围与后续人工验收见 `maintenance/float-window-blue-border-repair.md` 与 `evals/2026-08-11-float-window-repair-session.md`。

时间线更新（2026-08-11，需求 #93）：移除 subclass 的候选经用户否决。浮窗随即暴露上端及左端原生轮廓，WebView 左上角显示默认黑色窗口标题。该结果证明“完全不接管非客户区”不是当前 Tauri 宿主的完整修复；事故保持 `Open / S2`，回到历史路径与创建时序调查。

时间线更新（2026-08-11，需求 #94）：恢复创建期直接窗口过程后，用户确认左侧轮廓消失但上端条仍存。候选仅部分改善；在确认残留是否是系统 caption 或 Focus 网页头部前，不继续添加原生消息或样式处理。

时间线更新（2026-08-11，需求 #95）：用户确认残留上端条含有窗口标题文字，证实它是 Windows 原生 caption。当前根因假设收敛为拖动时 `WM_NCACTIVATE` 落入原 Tauri/default 过程并触发非客户区重绘；仍待以单一回归测试和用户手工验收验证。

时间线更新（2026-08-11，需求 #95）：回归测试先以 `WM_NCACTIVATE -> None` 失败；当前候选仅令创建期直接窗口过程对该消息返回已处理，阻止默认 caption 绘制。未改样式或拖动路径；自动测试已通过，人工拖动验收待执行。

时间线更新（2026-08-11，需求 #96）：用户已确认候选通过原生标题条与标题文字的真实拖动验收。INC-001 的 caption 症状标记为 `Verified`；同次发现原生毛玻璃层在 WebView 圆角外的矩形外溢，作为独立合成/圆角缺陷继续维修。

### INC-017 浮窗原生毛玻璃层越出圆角

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S3 / Fixed pending verification |
| 首次报告 | 2026-08-11，需求 #96。 |
| 影响与复现 | 五类内部浮窗的网页内容为圆角，但启用原生 SWCA 毛玻璃时，四角仍保留矩形的毛玻璃凸出部分。 |
| 根因证据 | 网页 `html` 与 `body` 的 `border-radius + overflow: hidden` 已裁切 WebView 内容；毛玻璃由宿主 HWND 的 `SetWindowCompositionAttribute` 产生，不受网页裁切影响。 |
| 修复 | DWM `DWMWA_WINDOW_CORNER_PREFERENCE = ROUND` 在隐藏创建时一次性配置；随后将透明 WebView 裁切与五类浮窗外层统一为 10px 的 `--window-host-radius`，并以无网页外壳的桌宠窗口作为视觉基准。未恢复 `SetWindowRgn`，未改变窗口过程或拖动路径。 |
| 验证证据 / 回归覆盖 | Rust DWM attribute/value 测试与前端五类宿主半径测试均已通过；构建、174 项 Rust 测试、schema 测试和 release 重建通过。用户视觉验收待完成。关联需求 #96/#97、ADR-0029、维修文档与本轮 Eval。 |

### INC-002 频繁打开或点击内部窗口导致 Focus 卡死

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S2 / Verified |
| 首次报告 | 2026-08-06，需求 #12；2026-08-08 需求 #31 再次报告。 |
| 影响与复现 | 连续双击快捷方式或反复打开内部页时，对话、统计、音乐等窗口可能卡死，用户需要结束进程。 |
| 根因证据 | 原前端只按 label 做 150ms 时间窗去重，无法代表一次原生 `restore` 已结束，也不覆盖托盘展开/收起与不同窗口混合点击；后端 `restore`/`collapse` 没有共享可见性门，显示、隐藏、定位和置顶可并发进入同一浮窗生命周期。 |
| 修复 | v1.12.8 以单一前端 in-flight 控制器统一托盘动作；未返回期间禁用条目并忽略新动作。Rust 端增加共享可见性操作门，并发 `restore`/`collapse` 返回「窗口操作正在进行，请稍候」，不再排队。无空位不恢复、零重叠及 `SWP_NOACTIVATE` 路径不变。 |
| 验证与回归 | 前端覆盖未完成 restore 期间的重复托盘动作；Rust 覆盖可见性门拒绝重入并在释放后重新开放；v1.12.8 的 48 项前端、175 项 Rust、schema、release 重建和用户发布验收均已记录。关联需求 #31/#99、v1.12.8 Eval。 |

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

### INC-013 同日聊天历史缺失且 Claude 每轮新建进程

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Agent/workflow / S2 / Fixed pending verification |
| 首次报告 | 2026-08-10，需求 #83。 |
| 影响与复现 | 同一宠物当天重新打开聊天时没有可见历史；每次发送都像一次性新会话，无法稳定进行连续追问。 |
| 根因证据 | SQLite 只保存 Provider session id，不保存可见消息；Claude Runtime 每个 turn 都启动新 CLI 进程，虽可 resume 但不保持 Focus 运行期间的进程上下文。 |
| 修复 | 可见消息按宠物 x Provider x 本地日期落库并回放；Claude 改为 stdin `stream-json` 常驻进程，Focus 重启后只在首轮 `--resume`。 |
| 验证与回归 | Rust 覆盖常驻多轮输入、取消后恢复、首轮 resume；前端覆盖 Provider 隔离回放、生命周期消息移除及跨午夜只写当天历史。仍待真实 Claude 三轮追问和 Focus 重启后的上下文恢复，关联 ADR-0026 与 `evals/2026-08-10-window-regression-checkpoint.md`。 |

### INC-014 定时工作流重入遗留重复 `running` 记录

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Automation / S2 / Fixed pending verification |
| 首次报告 | 2026-08-10，需求 #83 的每周计划 release 验收中发现。 |
| 影响与复现 | 已调度的工作流仍在运行时，每个 15 秒 scheduler tick 都插入一条新的 `running` 记录；实际节点只执行一次，但运行历史被污染且 UI 显示多个永不完成的条目。 |
| 根因证据 | 旧 `run_workflow` 先持久化运行记录，`start_run` 随后发现 `running` 已含该工作流时返回 `Ok(())`；scheduler 忽略返回值。真实 release 在一个临时 Agent 日程中连续留下 13 条记录。 |
| 修复 | 在持久化前原子领取 workflow id；未领取时直接返回“工作流正在运行”，启动函数只接受已领取的运行。写入失败会释放领取。 |
| 验证与回归 | Rust 回归 `workflow_run_claim_prevents_scheduler_reentry_until_released`；重建 release 中临时每周 wait 日程运行 35 秒期间始终只有 1 条记录并以 success 收束，随后已删除。真实 Claude 日程仍受当前 Provider admission 阻塞，关联本轮 Eval。 |

### INC-015 恢复浮窗时允许与现有窗口重叠

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Window / S2 / Fixed pending verification |
| 首次报告 | 2026-08-10，需求 #85。 |
| 影响与复现 | 已有 chat、stats、pet 可见时，从视图托盘打开 music 或 workflow；保存位置无空位时，窗口仍以原位置显示并与其他窗口大面积重叠。 |
| 根因证据 | `restore_window()` 调用 `find_free_slot()` 后只处理 `Some`，`None` 仍先移除 `collapsed` 并显示窗口。release 探针曾记录 workflow `155,105 933x527` 与 stats `311,211 778x421` 重叠。 |
| 修复 | `GridManager::restore_slot()` 把无空位表示为明确失败；恢复在状态持久化前完成槽位决策。无空位时维持折叠，前端显示“没有可用位置，请先折叠一个窗口”。 |
| 验证与回归 | Rust 覆盖满格拒绝；release UI Automation 验证打开 music 时提示可见、music 保持折叠；折叠 chat 后打开 music，三个可见浮窗的矩形两两不重叠。关联需求 #85 与 `evals/2026-08-10-window-regression-checkpoint.md`。 |

### INC-016 Claude CLI 子进程弹出黑色终端

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Agent/workflow / S3 / Verified |
| 首次报告 | 2026-08-11，需求 #87。 |
| 影响与复现 | 每次 Focus 调用 Claude Code 时出现独立的纯黑色控制台窗口，打断聊天并暴露实现细节。 |
| 根因证据 | Windows 子进程通过 `cmd.exe`/`claude.cmd` 启动时未设置 `CREATE_NO_WINDOW`；现有 stdin/stdout/stderr 管道本身不要求可见控制台。 |
| 修复 | `ClaudeProvider` 在所有 Claude 子进程创建路径设置 `CREATE_NO_WINDOW`，保留现有流式管道、session resume 与取消语义。 |
| 验证与回归 | Rust `claude_child_uses_no_console_creation_flag`、stream-json 参数和 stdin 传输测试通过；真实 release 聊天启动是否完全不显示控制台仍待 Windows 人工确认，关联 Eval：`evals/2026-08-11-float-drag-and-skill-checkpoint.md`。 |

### INC-018 Skill 输入被实现为输入框外 chip，不能作为真实混排文本编辑

| 字段 | 内容 |
| --- | --- |
| 类别 / 严重性 / 状态 | Agent/workflow / S3 / Fixed pending verification |
| 首次报告 | 2026-08-11，需求 #88/#90/#99。 |
| 影响与复现 | 点击 Skill 后标记在输入框外或仅由 store 预拼接；用户无法在文字内按光标插入、连续叠加或只删除相邻一个 Skill，所见顺序与真实 Provider 收到的文本可能不一致。 |
| 根因证据 | 旧 ChatView 由独立 chip + `<input>` 组成，发送时 `agent` store 再把 `selectedSkills` 前置拼接到输入值；它不是浏览器编辑模型中的同一文本序列。 |
| 修复 | 改为单行 `contenteditable`，Skill 为 `contenteditable=false` 的内嵌 Token；选择时依当前 selection 插入并留空格，Backspace/Delete 只整体移除紧邻 Token。发送根据 DOM 子节点实际顺序序列化，store 不再重写消息或读取 `SKILL.md`。 |
| 验证与回归 | 前端覆盖 Token 插入、叠加、左右相邻删除、文本混排、序列化与清空；48 项前端、175 项 Rust、schema、release 重建及用户发布验收已记录。关联 ADR-0028、需求 #99、v1.12.8 Eval。 |

### INC-019 Float grid-glow center offset

| Field | Detail |
| --- | --- |
| Category / severity / status | Window / S3 / Verified |
| First report | 2026-08-11, requirement #102 |
| Impact and reproduction | On the first drag after opening or restoring an internal float, its grid brightness center could appear about two cells left and up to one cell below the visible window. A later drag could appear normal. |
| Root-cause evidence | v1.12.8's native-host repair introduced parallel client/outer geometry paths: positioning sampled a live frame while preview and snap carried independent origin and size values. First-show lifecycle timing could leave those values inconsistent. |
| Fix | v1.12.10 uses one ClientGeometry snapshot for preview, snap, and final client-to-outer conversion. |
| Verification and regression coverage | The red-first Rust test proves preview client geometry converts back to final outer placement; 49 frontend tests, 176 Rust tests, schema tests, build, and release rebuild passed. User completed the real mouse-drag gate with no glow, caption, overlap, or tray-freeze regression. |
| Links | Requirement #102; docs/maintenance/float-window-blue-border-repair.md; docs/evals/2026-08-11-v1.12.10-float-geometry.md |

2026-08-14 root cause verified: the independent pet-bubble WebView was never in
the Tauri capability window list, so every event `listen` was ACL-rejected and
the host never reported a ready generation; the native Bubble Controller could
never dispatch. `pet-bubble` is now in the default capability window list and
the diagnostics command expires stale pending envelopes. INC-022 stays
`Fixed pending verification` until the user confirms a real bubble next to
the pet on a successful reply.

2026-08-14 Verified: the user confirmed the bubble appears beside the pet on
every successful direct reply with chat open and closed, after the capability
ACL and resident assistant-message streaming fixes.
