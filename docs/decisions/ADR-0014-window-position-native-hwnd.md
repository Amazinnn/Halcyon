# ADR-0014：窗口位置操作统一原生 HWND（v1.10.2 防御，延续 ADR-0013）

- 状态：已接受（2026-08-08，v1.10.2 修复轮）
- 关联：需求 #34/#35；ADR-0013（拖动浮窗改原生 HWND 移动）

## 背景

v1.10.1 将拖动 poller 改为原生 SetWindowPos 后，2026-08-08 09:29:20 仍出现 AppHangB1（v1.10.1 构建，挂起约 28.7s）：用户在对话/统计/桌宠三窗重叠时卡死。除拖动路径外，恢复/吸附/回弹仍经 `position_window` 调用 `set_position`（主线程 WebView2 SetBounds 同步 COM RPC）。重叠触发的高频窗口操作（restore/回弹/raise）可能再次把主线程阻塞在 WebView2 COM 等待上。

## 决策

1. **位置操作统一原生 HWND**：`position_window` 在位置变化时优先用 `SetWindowPos(SWP_ASYNCWINDOWPOS)` 移动（复用 drag.rs 的 `move_window_raw`），不再触发 WebView2 `SetBounds`；**尺寸变化仍走 `set_position+set_size`**（低频、必须让 WebView2 重排）。
2. **不做网格/吸附/持久化协议改动**；原生移动后 `outer_position` 读取真实位置，finalize/持久化逻辑不变。
3. **重叠卡死取证**：开发侧在 monitor 下受控复现（拖动重叠、连续恢复多窗口）；若 HUNG 保留 minidump+STILL_HUNG 时间线，用线程栈确认是否仍为 WebView2 COM 等待；若根因不同（如透明窗口重叠合成压力），保留证据进入下一轮专项。

## 风险与回退

- 原生移动后 WebView2 内容短暂滞后（浏览器端异步跟随）——手测「窗口跟手/内容不错位」判定；若明显，回退为原 set_position 并仅保留拖动路径原生化。
- 尺寸变化仍走 WebView2 SetBounds（低频），不消除该 COM 调用本身。

## 后果

- drag.rs：`move_window_raw` 提升为 pub(crate) 供 lib.rs 复用。
- lib.rs：`position_window` 位置变化走原生移动；workflow 默认 4×3。
- settings.rs：v1.10.2 布局迁移（workflow 2×2→4×3、music 3×2→3×3、stats 4×3→5×4，一次性）。