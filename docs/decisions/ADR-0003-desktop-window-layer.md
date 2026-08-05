# ADR-0003：DesktopWindow 层级 —— 普通全屏（Overlay）

- 状态：已接受（2026-08-05，任务 C）
- 关联：设计稿 v0.2 §2.4、§23 问题 1；`workerw_probe` 探针

## 决策

- DesktopWindow 采用**普通全屏、无边框 WebView 窗口**作为稳定模式（Desktop Overlay），不替换 Shell，不把应用嵌入桌面层。
- **放弃 WorkerW 桌面层路线**。

## 依据（本机实测）

- `workerw_probe` 枚举到 15 个 WorkerW，均无 `SHELLDLL_DefView` 子窗口（图标层不在 WorkerW 下）。
- 将测试窗口 `SetParent` 到 WorkerW 后，`GetParent` 立即报"句柄无效"——测试窗口已被系统销毁。这是 Windows 已知行为：**跨进程 SetParent 会销毁子窗口**；Tauri WebView 窗口与 Explorer 属不同进程，故该路线不成立。
- 普通全屏窗口实测覆盖全屏（1536×960）且稳定。

## 后果

- 不引入 WorkerW/桌面层复杂度；后续如需"壁纸层"效果，只能走非跨进程方案（不适用于 WebView2），故不做。
- Windows 虚拟桌面仍按计划推迟到 M6（见可行性报告 #11）。