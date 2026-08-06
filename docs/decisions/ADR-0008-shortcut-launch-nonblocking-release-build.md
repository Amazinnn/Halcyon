# ADR-0008：修复 release dev 模式构建与快捷方式启动阻塞

- 状态：已接受（2026-08-06，交互修复包）
- 关联：ADR-0005（窗口管理）；ADR-0006（focus-cli 控制面）；需求记录 #12

## 背景

用户报告三处交互缺陷：① 加号弹出的临时浮窗内文字逐字竖排；② 新建的快捷方式不显示真实图标而显示占位图案；③ 快速多次点击快捷方式后，Agent 对话/统计/音乐等浮窗整体卡死。排查还发现一个更前置的问题：`release/desktop.exe` 实际以 dev 模式运行，WebView 全部加载 `http://localhost:1420/`，没有 Vite 时页面全部 `ERR_CONNECTION_REFUSED`。

根因均已取证：

1. **release 误入 dev 模式**：`tauri-2.11.5/build.rs` 判定 `dev = !custom-protocol`；本项目 `apps/desktop/src-tauri/Cargo.toml` 缺少模板必备的 `[features] custom-protocol = ["tauri/custom-protocol"]`，导致 release 构建不内嵌 `frontendDist` 资产、`generate_context!` 使用 `devUrl`。
2. **加号弹层文字竖排**：`.add-menu` 是绝对定位在 104px 宽的 `.add-slot` 内，shrink-to-fit 被槽位宽度封顶（实测菜单仅 84px、按钮 70px），`white-space: normal` 下中文逐字换行。
3. **新建快捷方式无真实图标**：`stores/shortcuts.ts` 的 `addPath()` 落库后不调用 `loadIcon()`，只有应用启动时 `load()` 才拉图标；运行时实测 `get_shortcut_icon` 对记事本/Obsidian 均可正常返回。
4. **双击卡死**：`launch_shortcut` 是同步 command，`launch.rs` 每次最多阻塞 2s（PID 轮询）+ 6s（新窗口探测）；`wry-0.55.1` 在 WebView2 `WebMessageReceived`（UI 线程）内同步执行 IPC handler，双击等于主线程连续阻塞，所有浮窗无响应。

## 决策

1. **release 构建必须启用 `custom-protocol` 特性**。`apps/desktop/src-tauri/Cargo.toml` 增加 `[features] custom-protocol = ["tauri/custom-protocol"]`；重建前先 `cargo clean -p focus-desktop` 清除旧的 dev-cfg 构建脚本缓存。验收标准：release 启动后 WebView 页面 URL 为 `tauri://localhost` 且不依赖 Vite。
2. **快捷方式启动不得阻塞主线程**。`launch_shortcut` 改为 `async` command：非 `internal` 类在 `tauri::async_runtime::spawn_blocking` 中执行 `launch::launch_shortcut`；`internal` 类（涉及 Focus 窗口 API）经 `app.run_on_main_thread` 派发 `restore`。`AppState` 增加 `launch_lock: tokio::sync::Mutex<()>`，用 `try_lock` 单飞：已有启动进行中时后续请求立即返回「另一个快捷方式正在启动」错误，不排队、不累积阻塞。
3. **前端配合**：`shortcuts.ts` 增加 in-flight 去重（同一快捷方式启动中忽略重复点击），`addPath` 后对 `application` 类立即加载图标；`.add-menu` 使用 `width: max-content` 并按内容单行展示（`white-space: nowrap`）。
4. **文件夹/URL 快捷方式保持 glyph 占位设计**，仅 `application` 类提供真实图标。

## 风险与后果

- `launch.rs` 的窗口探测超时（2s/6s）保持不变，但已移至后台线程；探测期间 Focus UI 保持响应。
- `internal` 快捷方式恢复窗口仍全部落在主线程执行，避免 Tauri 窗口 API 跨线程副作用。
- 单飞语义下，双开不同应用的第二请求会立即收到错误而不是排队；前端在 `console` 记录，不新增 UI 弹层。
- release 验收必须以重建后的二进制为准；旧 release 若仍加载 `localhost:1420`，说明构建脚本缓存未清除或 `custom-protocol` 特性未生效。

## 后续

- 验收通过后回填需求记录 #12 状态，并同步更新 #5 的卡死部分状态。
- 多显示器验收仍为 N/A（本机单显示器）。