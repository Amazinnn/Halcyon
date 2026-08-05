# 技术 Spike 调查报告 v0（任务 B）

> 日期：2026-08-05
> 交付：`apps/desktop`（Tauri 2 + Vue 3 + TS + Rust + SQLite），`packages/event-schema`（AgentEvent v1）
> 验证：`cargo check` 无错误；`cargo test --lib` 3/3 通过；`npm run build`（vue-tsc + vite）通过；运行时四窗口创建成功、探针写入 SQLite 正常（见 `windows-feasibility-v0.md`）

## 已验证的能力

1. **四窗口**：DesktopWindow（全屏无边框）、PetWindow（透明置顶可拖动）、PanelWindow（Chat/Statistics 路由切换）、MusicWindow（置顶无边框）——单进程创建成功。
2. **事件总线**：`tokio::sync::broadcast` + relay → `app.emit` → 前端 `listen`；事件名按 Tauri 约束改为冒号分隔（ADR-0002）。
3. **Mock Agent**：`agents/mock.rs` 每 2s 按脚本序列（thinking→reading→editing→waiting_permission→success，每 3 轮穿插 error）发出**符合 AgentEvent Schema v1 的 JSON**（`include_str!` 嵌入 schema；轻量 schema 驱动校验测试 3/3）。
4. **UiCoordinator**：Pinia store 管理 `panelMode/petVisible/speechBubbleVisible/doNotDisturb/lockActive`；Chat 打开时宠物气泡被抑制（§7.1）。
5. **假统计**：月度热力图（30 天）、24h 分布、音乐类型分布（Chart.js），固定种子数据保证截图稳定。
6. **pet-pack**：Manifest 校验器 + 内置占位宠物（ADR-0004）。
7. **SQLite**：`schema_migrations` + `spike_probes`；前台探针每 5s 写入一条记录（实测 8 条）。
8. **MusicWindow**：假数据 3 曲歌单、1s 心跳推进进度、上一首/播放暂停/下一首/进度拖动；`music:playback_tick` 走前端→核心→总线路径。

## 关键发现

- **Tauri 事件名不能用 `.`**（运行时报 `IllegalEventName`），统一改冒号命名（ADR-0002）。
- **WorkerW 桌面层对 WebView2 不可行**：跨进程 SetParent 销毁子窗口（ADR-0003）。
- **200% DPI 下尺寸语义需核对**：`inner_size(440,680)` 逻辑尺寸实测外框 453×716（非客户区 + 尺寸换算），M0 需确认物理/逻辑映射。
- **透明窗口**：由 WebView2 合成（未置 `WS_EX_LAYERED`），需目视确认（截图在 `evidence/`）。
- **jsonschema 依赖过重**：Rust 侧不引入完整 JSON Schema 引擎，改用嵌入 schema 驱动的轻量结构校验（枚举与 required 均从 schema 读取）；全量校验保留在 `packages/event-schema`（Ajv）。

## 未验证/待人工项

- 透明视觉、Alt+Tab、Win+D、全屏共存、睡眠恢复、面板聊天渲染与气泡互斥的目视确认；多显示器（本机单屏）。