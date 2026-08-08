# ADR-0023：桌面锁（focus-mode lock，后端能力）

- 状态：已接受（2026-08-08）
- 关联：需求 #70；未来「专注模式」的前置后端能力

## 背景

用户希望锁住 Windows 桌面及其下栏（任务栏），「在软件中时无法打开」；具体使用方式留给之后引进的专注模式（#70）。本次只做后端能力（lock/unlock 命令 + 逃生体系），UI 触发不做（#19：开发完成前不绑全局快捷键）。

## 决策

1. **锁的语义**：隐藏任务栏（`Shell_TrayWnd`）+ 隐藏桌面图标（`Progman`）+ 低级键盘钩子拦截 Win 键 / Alt+Tab / Alt+F4 / Ctrl+Esc。Focus 全屏 desktop 窗口本身在桌面层盖住一切。

2. **解锁**：界面内解锁（未来专注模式）+ 未来自动解锁；本次只做 focus-cli `desktop unlock`（手动）。

3. **六层崩溃检测/逃生**（测试阶段绝不锁死）：
   - ① **panic hook**：panic 时先解锁再输出（开发期防御）
   - ② **Drop impl**：正常退出恢复（产品保留）
   - ③ **watchdog 子进程**：`--focus-watchdog <pid>` 模式，监控主进程句柄（`WaitForSingleObject`），主进程任何方式死亡（含 taskkill /F）→ 恢复桌面（开发期防御）
   - ④ **focus-cli desktop unlock**：TCP 通道天然不被键盘钩子拦（产品保留，Agent 也能解锁）
   - ⑤ **逃生文件**：`%TEMP%/focus-lock-escape.tmp` 出现即解锁（开发期防御）
   - ⑥ **explorer 重启**：Ctrl+Alt+Del → 任务管理器 → 重启 explorer.exe（Windows 原生兜底，文档告知，无代码）

4. **模块化（#70 grill 定稿）**：`desktop_lock.rs`（核心：lock/unlock/钩子/Drop，产品保留）+ `desktop_lock_escapes.rs`（开发期防御：panic hook/watchdog/逃生文件，**只调核心公开 API**）。产品期删一个文件即移除，核心零改动。不用 cargo feature flag。

5. **失败兜底**：lock 失败不进入锁定态（恢复已隐藏的窗口，不半锁死）；unlock 尽力恢复所有（单个失败不中断其余）。

6. **安全边界**：Ctrl+Alt+Del 任何软件拦不了（Windows 安全边界，用户已知悉）。

## 影响

- 未来专注模式直接调 `desktop_lock::lock_desktop()/unlock_desktop()`（或 focus-cli desktop lock/unlock，Agent 也能控制）。
- 开发期六层逃生保证测试不锁死；产品期移除 ①③⑤（删除 desktop_lock_escapes.rs + lib.rs 一行调用）。
