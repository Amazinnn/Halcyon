# ADR-0015：浮窗圆角裁剪改原生 SetWindowRgn（v1.10.3，复开 #37/#42）

- 状态：已接受（2026-08-08，v1.10.3 修复轮）
- 关联：需求 #37/#42；ADR-0005（视觉与窗口管理）

## 背景

v1.10.2 为消除「web 自带框 + 绘制框」双层纹路（#37），在 `html.transparent-window, body` 上加了 `border-radius: var(--r-lg) + overflow: hidden`。用户验收后确认该方案无效（#42 原话：「圆角玻璃边没有Web自带框，这个完全没有实现」）。

根因：WebView2 的透明合成层是矩形像素，渲染进程只能把页面画成圆角，但合成层矩形边缘无法被 CSS 裁剪到原生窗口像素；容器圆角之外仍会露出极细的矩形页边/合成边缘，形成第二层框。

## 决策

1. **原生裁剪 HWND**：新增 `window_rgn.rs`，对 chat/stats/music/pet/workflow 五个浮窗调用 `CreateRoundRectRgn(0,0,w,h, r*2, r*2)` + `SetWindowRgn`（r = 16 CSS px × scale factor）。`SetWindowRgn` 接管 region 句柄，不得再 `DeleteObject`。
2. **调用时机**：窗口创建后一次；`position_window` 尺寸变化分支与 `resize_window` 提交后同步调用（尺寸变化会改变区域）。
3. **静默容错**：hwnd/尺寸获取失败直接跳过，不阻塞窗口管理。
4. **回退**：若 SetWindowRgn 与 SWCA 毛玻璃/透明合成冲突（截图确认出现直角或毛玻璃失效），回退保留 CSS 方案并记录证据，进入下一轮专项。

## 风险

- SetWindowRgn 使区域外完全不渲染（预期效果即「区域外全透明」，需目视确认毛玻璃仍正常）。
- 区域随尺寸/DPI 变化，漏同步会导致内容被裁或露出矩形角。

## 后果

- 新增 `apps/desktop/src-tauri/src/window_rgn.rs`；lib.rs 的 `create_windows` / `position_window` / `resize_window` 接入。
- CSS `border-radius + overflow:hidden` 保留（双保险，不冲突）。