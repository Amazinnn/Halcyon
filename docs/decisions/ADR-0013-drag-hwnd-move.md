# ADR-0013：拖动浮窗改原生 HWND 移动（v1.10.1 卡死修复）

- 状态：已接受（2026-08-08，v1.10.1 修复轮）
- 关联：需求 #31/#34；ADR-0003（桌面窗口层级）、ADR-0005（视觉与窗口管理）

## 背景

v1.10 去冗余窗口操作后，用户于 2026-08-08 08:49 在拖动四浮窗（对话/统计/音乐/工作流）时再次触发进程级卡死：AppHangB1，挂起约 28.7s 后被 Windows 关闭。彻查取证：

- WER 1001（AppHangB1）P4=0x703e≈28734ms；挂起栈相关模块含 ole32/combase/RPCRT4（COM RPC 等待）与 user32/win32u/GDI；同一时段无 msedgewebview2 崩溃报告（浏览器进程未崩）。
- 拖动实现：poller 每 15ms 调 `WebviewWindow::set_position`（tauri 异步 post）→ 主线程 wry `set_bounds_inner` → `ICoreWebView2Controller::SetBounds`（同步跨进程 COM RPC）+ `SetWindowPos(SWP_ASYNCWINDOWPOS)`；GRID_LABELS（chat/stats/music/pet）每 tick 还 `emit grid:preview`，grid-overlay 每 15ms 重建 22 条全屏渐变（约 726 个 gradient stop）。
- 受控复现（拖 workflow 8s / 拖 music 12s）未触发 HUNG，但拖动期间 msedgewebview2 CPU 持续约 0.7 核——压力路径成立，卡死为偶发时序。

结论：主线程在 WebView2 同步 COM RPC 上等待繁忙/延迟响应的浏览器进程；v1.10 修复未触及拖动路径，故仍复发。

## 决策

1. **拖动移动优先原生 HWND**：poller 每 tick 用 `SetWindowPos(hwnd, x, y, 0, 0, SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE|SWP_ASYNCWINDOWPOS)` 直接移动窗口，不触发 WebView2 `SetBounds` 同步 COM RPC；浏览器端由 WM_MOVE → `NotifyParentWindowPositionChanged` 异步跟随。hwnd 获取失败时回退现有 `set_position`。
2. **grid:preview 节流**：poller 内预览至少间隔 50ms（≈20fps），亮度跟随仍连续（位置变化在下一 tick 补发）；节流判定提取纯函数 `should_emit_preview` 便于单测。
3. **poll 频率降载**：`POLL_MS` 15→24（≈41fps），消息率降约 37%。
4. **取证增强**：hang-detector 在 HUNG 持续期间每 3s 追加 `STILL_HUNG` 记录（线程数/窗口标题），配合已有 2 份 minidump 定位主线程栈。

## 风险与回退

- 原生 HWND 移动可能使 WebView2 内容短暂滞后（浏览器端异步跟随）。以手测「拖动跟手/网格亮度跟随」判定；若明显滞后，回退为「仅预览节流 + poll 24ms + SetBounds 降频至 50ms」。
- 不改 grid-overlay 前端渲染方式、不引入新依赖、不动窗口尺寸/网格/吸附逻辑。

## 后果

- drag.rs：新增原生 HWND 移动路径与预览节流；POLL_MS 调整；模块 threading 注释更新。
- hang-detector.ps1：新增 STILL_HUNG 周期记录。
- 文档：需求 #34；本 ADR；STATUS/next-phase 标记 v1.10.1 修复轮。