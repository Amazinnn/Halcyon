# 窗口回归检查点（2026-08-10）

## 范围

- 需求 #84/#85；事故 INC-001、INC-015。
- 浮窗隐藏、非客户区样式与恢复布局。
- #83 的两处边界补正：本地日期跨午夜的可见历史轮换，以及每周计划参数验证。

## 自动验证

| 检查 | 结果 | 证据 |
| --- | --- | --- |
| `apps/desktop: npm test` | Pass | 10 files，43 tests；新增无空位提示及跨午夜历史轮换测试。 |
| `apps/desktop: npm run build` | Pass | `vue-tsc --noEmit` 与 Vite production build 通过。 |
| `apps/desktop/src-tauri: cargo test --lib` | Pass | 170 tests；新增满格恢复拒绝与非法每周计划验证。 |
| `packages/event-schema: npm test` | Pass | 11 valid / 4 invalid fixtures，`tsc --noEmit`。 |
| `launch-focus.cmd rebuild` | Pass | release 重建并启动成功。 |

## 原生 Windows 验收

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| 折叠真实隐藏 | Pass | release UI Automation 点击对话的“折叠为 logo”；`desktop layout` 含 `chat`，窗口不再可见。 |
| 满格恢复不重叠 | Pass | chat/stats/pet 已可见时请求打开 music，主界面出现“没有可用位置，请先折叠一个窗口”，music 仍在 `collapsed`。 |
| 释放位置后恢复 | Pass | 折叠 chat 后打开 music；stats/music/pet 三个可见宿主的矩形两两不重叠。 |
| 样式 / 激活 | Pass | `scripts/window-style-probe.ps1`：内部宿主为 `WS_POPUP`，无 caption/thick frame，未为前台窗口。 |
| 淡蓝标题条视觉 | Pending | 原生样式条件已通过；需要用户在真实可见 WebView 上连续打开、关闭、移动、缩放各类型浮窗确认。 |
| 桌面锁手工回归 | Pending | 本次未改；按桌面锁 Eval 项单独执行。 |

## #83 边界补正

| 项目 | 状态 | 证据 |
| --- | --- | --- |
| 跨午夜可见聊天隔离 | Pass | Vitest 固定 23:59→00:01；新消息只写入当天键，前一天消息不被复制。 |
| 非法每周计划拒绝 | Pass | Rust 将 `weeklyDay=7` / `weeklyTime=99:00` 的 scheduled 工作流拒绝在保存前。 |
| 真实 Claude 三轮与重启恢复 | Pending | 仍需按 #83 的真实 Provider 硬门槛执行，不以本地时间单测替代。 |

## 后续手工清单

1. 连续打开、折叠 chat、stats、music、pet、workflow；确认无淡蓝/白色原生边缘。
2. 先打开三个浮窗，再请求打开第四个；确认它保持折叠并显示简短提示，而不是重叠。
3. 折叠任意一个后重新打开刚才被拒绝的窗口；确认它占用空位且不遮挡其他窗口。

关联：[STATUS](../STATUS.md)、[事故台账](../production-incidents.md)、[Eval 规范](README.md)。
