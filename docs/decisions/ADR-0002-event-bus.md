# ADR-0002：Rust 事件总线与多窗口同步

- 状态：已接受（2026-08-05，任务 B）
- 关联：设计稿 v0.2 §13.3；`apps/desktop/src-tauri/src/event_bus.rs`

## 决策

- 核心内使用 `tokio::sync::broadcast::Sender<CoreEvent>`（容量 256）；一个 relay 任务订阅总线并 `app.emit(event_name, payload)` 转发到所有窗口。
- 窗口之间禁止直接互调；前端统一通过 `@tauri-apps/api/event` 的 `listen`/`emit` 与核心通信。
- 最小事件集（Spike）：`agent:event`、`pet:state_changed`、`bubble:requested`、`panel:mode_changed`、`music:playback_tick`、`probe:recorded`，以及前端→核心的 `ui:toggle_panel`、`ui:panel_mode_changed`。

## 发现与细化

- **Tauri 事件名禁止 `.`**（运行时报 `IllegalEventName`）。设计稿中的点分命名（如 `agent.event`）在实现时改为冒号分隔（`agent:event`），冒号是 Tauri 官方命名约定，冒号与下划线均可安全使用。
- 前端→核心的事件（如 `ui:toggle_panel`）由 Rust `listen` 接收后转成 `CoreEvent` 重新广播，保证单向数据流。

## 后果

- 任何新事件名必须满足 Tauri 命名约束（字母/数字/`:`/`_`/`-`），不得使用 `.`。