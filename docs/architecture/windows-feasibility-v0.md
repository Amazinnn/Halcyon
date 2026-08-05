# Windows 可行性测试报告 v0（任务 C）

> 日期：2026-08-05
> 对象：`apps/desktop` 技术 Spike（Tauri 2.11 + WebView2，debug 构建）
> 环境：Windows 11，单显示器；物理最大分辨率 3072×1920，当前有效桌面 1536×960（OS 缩放 200%，`GetDpiForWindow` 实测所有窗口 DPI=192）
> 方法：启动 Spike（`npm run tauri dev`），用 user32 `EnumWindows`/`GetWindowLong`/`GetDpiForWindow`/`GetWindowRect` 做机械实测；截图存 `docs/architecture/evidence/`。

## 测试矩阵

| # | 测试项 | 方法 | 结果 | 说明 / 风险 + 退路 |
|---|---|---|---|---|
| 1 | 四窗口创建与属性 | EnumWindows 按标题枚举 | **PASS** | 单进程创建 Desktop / Pet / Panel / Music 四个 WebView 窗口（类名 `Tauri Window`）：Desktop 全屏无边框；Pet 200×200 置顶、跳过任务栏；Music 340×110 置顶；Panel 可调大小 |
| 2 | DesktopWindow 全屏覆盖 | GetWindowRect | **PASS** | Desktop 外框 (0,0,1536,960) 恰好覆盖当前屏幕；普通全屏窗口，不替换 Windows Shell |
| 3 | 置顶（宠物/音乐） | GetWindowLong WS_EX_TOPMOST | **PASS** | Pet、Music `Topmost=True`；Panel、Desktop 非置顶 |
| 4 | 透明窗口 | 扩展样式 + 截图 | **部分（待目视）** | Pet/Music 未置 `WS_EX_LAYERED`——透明由 WebView2 合成实现，非经典分层窗口；截图已存 evidence。风险：个别 GPU/驱动下 WebView2 透明异常；退路：改用不透明圆角窗口或 DWM 检测 |
| 5 | DPI 缩放（200%） | GetDpiForWindow | **PASS（记录）** | 四窗口均 DPI=192。注意：请求 Panel `inner_size(440,680)` 逻辑尺寸，实测外框 453×716——差异来自非客户区（标题栏+可调边框）与 Tauri 尺寸语义；M0 需在 200% 缩放下核对 `inner_size` 的逻辑/物理映射 |
| 6 | Alt+Tab | 未自动执行 | **手动** | 需人工确认：全屏 Desktop 不抢占焦点、四个窗口在 Alt+Tab 中的次序与显示 |
| 7 | Win+D（显示桌面） | 未自动执行 | **手动** | 需人工确认 Win+D 后 Overlay 行为（最小化或保持），并决定是否需要 `toggle` 处理 |
| 8 | 全屏应用共存 | 未自动执行 | **手动** | 计划中音乐浮窗/宠物在全屏应用下自动收缩为 V1 未实现项，需人工观察置顶窗口与全屏应用的层级关系 |
| 9 | 多显示器 | 本机单屏 | **N/A** | 列入「需外接/换机或虚拟显示器驱动」验证清单 |
| 10 | WorkerW 桌面层 | `workerw_probe` 独立探针 | **FAIL（不可行）** | 本机 15 个 WorkerW 均无 `SHELLDLL_DefView` 子窗口；跨进程 `SetParent` 到 WorkerW 后，测试窗口立即被系统销毁（`GetParent` 报"句柄无效"）。这是 Windows 已知行为：跨进程 SetParent 会销毁子窗口 → **对 Tauri/WebView2 窗口，WorkerW 桌面层路线不成立**；DesktopWindow 保持普通全屏 |
| 11 | 虚拟桌面 | 源码调查（MScholtes/VirtualDesktop） | **高风险，推迟 M6** | 其 C# 源码通过 `[ComImport]`+硬编码 CLSID 调用未公开接口（`IVirtualDesktopManagerInternal`、`IApplicationView`、`IVirtualDesktopPinnedApps`），且 README 明示 Win11 23H2/24H2 曾改 GUID。首轮不实现、不用私有 API；若 M6 推进，倾向复用该 MIT C# helper 作独立兼容层，保留 Overlay 回退 |
| 12 | 睡眠/休眠恢复 | 未自动执行 | **手动** | 设计注记：计时状态与最近事件入库，恢复后对账；需人工实测休眠→唤醒后四窗口与计时是否正常 |
| 13 | 前台窗口探针 | GetForegroundWindow→进程名+标题→SQLite | **PASS** | `spike_probes` 每 5s 一条，实测 8 条连续记录（`desktop.exe | Focus Music` 等），迁移与写入正常 |
| 14 | Rust 事件总线→四窗口 | 代码路径 + 运行日志 | **PASS（实现层面）** | 事件名因 Tauri 限制不能用 `.`（`IllegalEventName`），已统一改为冒号分隔（`agent:event` 等，见 ADR-0002）；relay 任务 + 前端 `listen` 均已接线；前端→core 的 `music:playback_tick` 路径已注册（点击播放属手动验证） |
| 15 | 面板 Chat/Statistics 切换 | 代码实现（vue-router + Pinia） | **实现完成，待目视** | 路由 `/panel/chat`、`/panel/statistics`，切模式广播 `panel:mode_changed` |
| 16 | 宠物气泡互斥（Chat 打开时抑制） | 代码实现（PetView computed） | **实现完成，待目视** | §7.1：`panelMode==='chat'` 时不显示普通气泡 |

## 结论与建议

1. **核心路线成立**：Tauri 2 多窗口 + 普通全屏 Overlay + 置顶小窗（宠物/音乐）在本机（Win11 / 200% DPI）可稳定运行，前台探针与 SQLite 写入正常。
2. **放弃 WorkerW 桌面层**：对 WebView2 窗口不可行（跨进程 SetParent 销毁子窗口），DesktopWindow 采用普通全屏即可（ADR-0003）。
3. **虚拟桌面保持推迟**：依赖未公开 COM 接口，按计划 M6 处理，且优先 C# helper 方案。
4. **需人工/后续确认项**：透明视觉效果、Alt+Tab、Win+D、全屏共存、睡眠恢复、面板聊天渲染与气泡互斥、200% DPI 下 `inner_size` 语义。