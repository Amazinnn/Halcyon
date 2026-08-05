# ADR-0005：v1.2 前端视觉与窗口管理（设计语言 · 12×8 网格 · 毛玻璃 · 壁纸）

- 状态：已接受（2026-08-05，实现轮 v1.2）
- 关联：设计稿 v0.2 §2.4/§4/§5/§6；ADR-0002（事件总线）、ADR-0003（窗口层级）

## 决策

1. **设计语言 token**：`src/styles.css` 统一 CSS 变量——深墨绿底（#070b09→#16231c）、亮叶绿阳光高光（#a3e635）、纯绿系 + 双语义色（琥珀=等待/警告、珊瑚红=错误）、系统字体栈、CSS 动效（120/200/320ms + 计时呼吸 2.4s，`prefers-reduced-motion` 全禁用）。
2. **12×8 内容优先网格**：浮窗（对话/统计/音乐/桌宠）按逻辑屏幕 12 列×8 行放置；默认右侧栏（对话 4×4、统计 4×3、音乐 3×1、桌宠 1×1）；禁止重叠（被占格标红、释放回弹）；文本窗最小宽度护栏（≥3 列）；位置/置顶/折叠/logo 停靠持久化到 `settings.json`。
3. **层级与折叠**：展开浮窗默认置顶 + 每窗置顶开关；隐藏 → 毛玻璃 logo 胶囊吸附屏幕边缘（可沿四边拖动）→ 点击原位恢复；桌宠不参与折叠。
4. **毛玻璃**：Pet/Music/对话/统计/logo 窗用 `window-vibrancy::apply_acrylic` 真毛玻璃（半透明、不完全遮挡）；Panel 类不透明窗内卡片用 CSS `backdrop-filter`；Acrylic 不可用时降级半透明假玻璃。
5. **壁纸**：Dock「壁纸」按钮（`tauri-plugin-dialog`）+ 拖入图片双入口；复制到 `app_data_dir/wallpapers/`、路径写 `settings.json`、经 asset 协议（scope `$APPDATA/**`）加载；渲染=cover + 边缘模糊层 + 渐晕层衔接主题。壁纸为用户本地内容，不随发布版分发。

## 相对 v1.1 的调整

- **对话/统计拆分为两个独立窗口**（chat/stats），各自在网格中占格、可独立折叠——与用户"所有显示窗口都在网格"的模型一致。
- 事件：保留 agent 协议事件（agent:event / pet:state_changed / bubble:requested / music:playback_tick / probe:recorded）与既有事件名；UI 层移除 `panel:mode_changed`/`ui:toggle_panel`，新增 `window:visibility`、`logos:update`、`grid:preview`、`ui:toggle_chat`、`grid:drag_start/move/end`（均为冒号命名，符合 Tauri 约束）。
- 命令：新增 `get_bootstrap`、`get_grid_metrics`、`place_window`、`set_topmost`、`collapse`、`restore`、`dock_logos`、`get_wallpaper`、`persist_wallpaper`、`reset_wallpaper`、`quit_app`。

## 后果

- 新依赖：`window-vibrancy`、`tauri-plugin-dialog`、`@tauri-apps/plugin-dialog`（均 MIT/Apache，记入 THIRD_PARTY_NOTICES）。
- 网格坐标为逻辑像素（DPI 无关），多屏基于窗口当前所在屏（本机单屏验证，多屏后置）。

## v1.2.1 增补（拖拽修复 · 毛玻璃去灰 · 卡死修复）

2026-08-05，修复浮窗拖拽与毛玻璃视觉，并修复一次应用卡死（AppHangB1）。

### 拖拽改为 Rust 光标轮询
- 根因：原"webview 指针事件 + 每帧 IPC setPosition + 坐标换算 + 全屏 Overlay"架构不稳——轨迹振荡、窗口残留非网格位、释放落 (0,0)、实时位置与 settings.json 分叉。
- 实现：新增 `drag_start(label)`/`drag_end(label)` 命令与 `src-tauri/src/drag.rs`。`drag_start` 在主线程记录抓取偏移（`GetCursorPos − 窗口物理原点`），显示输入穿透的 Overlay，并启动 ~15ms 轮询线程：`GetCursorPos`（物理坐标）→ 光标未变则跳过 → `set_position`（异步投递，安全）→ 格变化 `emit grid:preview`；`GetAsyncKeyState(VK_LBUTTON)` 检测松开。
- 定位一律按屏幕边界钳制（物理级保险），杜绝 (0,0)/屏外落点；释放后复用 `place_window` 的占用回弹/越界钳制并持久化到 `settings.json`。
- 前端 `useGridDrag.ts` 瘦身为仅"按下/抬起"两个信号（`drag_start`/`drag_end`），删除全部 setPosition/rAF/坐标换算逻辑。

### 卡死（AppHangB1）根因与修复
- 根因：`drag_end`（主线程）`join()` 轮询线程；轮询线程在 `finalize` 中调用 `outer_position`/`outer_size`/`scale_factor`——这些是 `window_getter!` 同步请求，需回主线程等待响应 → 主线程等轮询、轮询等主线程，互锁死（Tauri 事件日志：Application Hang 1002）。
- 修复：主线程**永不 join** 轮询线程；`finalize` 只在主线程执行（经 `drag:released` 事件监听或 `drag_end` 命令）；轮询线程只做 `set_position`（异步）、`app.emit`、锁内读 settings/screen；停止改用 `finished: AtomicBool` + 有界等待（≤250ms）。
- 验证：合成拖拽探针（拖出 + 恢复）后应用存活，无 AppHang；手动多轮拖拽正常。

### 毛玻璃去灰（透明窗 + 不透明"墨水"内容）
- 根因：window-vibrancy 0.8 在 Win11（build ≥22523）走 `DWMSBT_TRANSIENTWINDOW` 分支并**忽略 tint**，浮窗变成系统默认浅灰磨砂（用户反馈"纯浅灰色、更不透明"）。
- 实现：删除 window-vibrancy 依赖，新增 `src-tauri/src/acrylic.rs` 自行动态调用未公开 `SetWindowCompositionAttribute`（SWCA），恒走 `ACCENT_ENABLE_ACRYLICBLURBEHIND` 并带低 alpha 深绿 tint `(14,24,18,56)` → 玻璃=仅模糊背景、不叠灰；`FOCUS_NO_ACRYLIC=1` 可整体跳过（CSS 回退）。
- 浮窗根（chat/stats/music）、WindowHeader、Logos 灰绿 tint 改 `transparent`；`--glass/--glass-strong` 改为不透明深绿 `#0e1612/#101a15`（"墨水"卡片/按钮）；气泡改不透明白；Desktop 壁纸融合 tint 保留。

### 桌宠与探针
- 桌宠身体（叶芽 SVG）可拖：去掉 `data-no-drag`（气泡/按钮仍不可拖）。
- 新增回归探针 `scripts/drag-probe.ps1`：合成"按住→分步移动→松开"，读目标窗口 `GetWindowRect` 轨迹，验证 1:1 跟随（≤8px）、无振荡、落点非 (0,0) 且在屏内、网格吸附、随后拖回原位；修复前复现振荡/落 (0,0)，修复后 PASS。


## v1.3 增补（专注桌面视觉精修 · 文件快捷区 · 专注/休息假心跳）

2026-08-05，把专注桌面从静态展示推进为可布置、有生命感的一版。

### 桌面三段式精修
- 顶条改为悬浮玻璃胶囊（当前任务 + Agent 状态点 + 专注/休息倒计时）；中央大计时保留（呼吸光晕）+ 可配置副题（默认"保持节奏，阳光会照到每一片叶子"，存 `settings.json.focusSubtitle`）；底部 Dock 精简为 **专注 / 设置 / 退出** 三键，"运行中应用"指示移除；Dock 为居中胶囊（`align-self: center`）。

### 文件快捷区（图标区）
- 合并为单一自适应网格（去应用/快捷页语义），初始为空、末尾固定全局「+」玻璃卡；点「+」弹迷你菜单（文件/应用 / 文件夹）→ 系统对话框（可多选）→ 卡片（玻璃卡 + 悬停上浮辉光 + 自绘类型字形 file/folder/app + 文件名）。
- 卡片指针拖拽换序（+ 固定末尾，pointer capture + 实时 splice），悬停右上角 ✕ 删除，点击用 `@tauri-apps/plugin-opener` 的 `openPath` 真实打开（capabilities 新增 `opener:allow-open-path`）。
- 持久化：`settings.json` 新增 `shortcuts: [{ id, name, type: "file"|"folder"|"application", target, order }]`；Rust 新增 `add_shortcut(path)`（按目录/扩展名推断类型、生成 id）、`remove_shortcut(id)`、`reorder_shortcuts(ids)`（`src-tauri/src/shortcuts.rs`，含推断与 renumber 单测）；`get_bootstrap` 返回 shortcuts。

### 设置弹层
- Dock「设置」打开小玻璃弹层：壁纸导入/重置（复用现有命令）+ 毛玻璃开关 + 版本/关于；新增 `set_acrylic(enabled)` 命令（`acrylic::clear` = ACCENT_DISABLED，实时 apply/clear）；`settings.json` 新增 `acrylicEnabled`，启动时按此应用，老配置文件经 serde default 兼容。

### 专注/休息假心跳
- ui store 增 `focusState: "idle"|"focus"|"rest"` 与 1s 心跳：专注向上计时、休息从 10:00 倒数到 0 自动回专注；顶条倒计时联动。
- 桌面根 `.focus-active`（专注态）：整体提亮（brightness/saturate 微升）+ 亮叶绿遮罩与光晕增强（"被注入春天的灵魂"），休息态恢复平静深绿；`prefers-reduced-motion` 下计时照走但无动画。
- 未改动：AgentEvent 协议、事件名、DB 表；浮窗拖拽/毛玻璃/气泡互斥/壁纸边缘虚化保持。


## v1.4 增补（番茄钟计时 · 监督 V1 软限制 · 应用图标 · 顶条开关）

2026-08-05，把计时从假心跳升级为完整番茄钟功能集，叠加 V1 软限制监督，并修复应用图标与顶条开关。

### 计时系统
- 状态机 `idle → focus(向下倒计时) → rest(向下倒计时)`：可暂停/继续（暂停冻结计时、氛围回平静）、跳过当前阶段、休息中可提前切回专注；无轮次（不计数、无长休息）；休息到 0 停在等待，显示「开始下一轮」按钮。
- 时长默认 25/5 分钟，设置内预设 25/5·50/10·90/15 + 自定义，切换下一轮生效。
- 计时区：大号呼吸计时 + 外圈细进度环（专注亮叶绿 / 休息琥珀，按剩余占比）；专注/休息中进度环下方出现「暂停/继续」「跳过」小玻璃按钮。
- 到点反馈：Web Audio 短音 + 宠物气泡（“专注完成/休息结束”），共用提示音开关（`soundEnabled`）。
- 数据：每完成一轮专注写 `focus_sessions`（开始/结束/时长/任务 id）；启动时 `get_today_focus_summary()` 读今日汇总；计时区底部「今日专注 X · 完成 N 轮」。
- 专注静音：专注中低优先级普通宠物气泡静音（监督提醒不受影响），休息恢复。
- 顶条：默认隐藏；`focusState !== idle` 自动显示；设置三态（自动/常显/隐藏）。

### 轻量任务（settings.json）
- `tasks[]`（id/名称/可选预计分钟/可选绑定应用）+ `currentTaskId`；设置弹层内编辑当前任务（名称+预计分钟）与切换；任务累计专注时长用于「任务超时」规则，切任务重置。任务累计专注存内存（重启清零），完整任务专注历史留给统计轮。

### 监督 V1（src-tauri/src/supervision.rs，2s 心跳，复用 activity 探针）
- 规则：①分心超时——黑名单进程名（不区分大小写、支持 `*通配*`），前台命中且不在白名单，持续 2 分钟首提；期间短暂切走 <30 秒不算打断，≥30 秒重置；同段内冷却逐级 5→3→1→0.5 分钟且语气加重。②空闲——前台非黑名单且无输入（`GetLastInputInfo`）≥3 分钟提醒一次，有输入即重置；分心优先。③任务超时——当前任务有预计分钟且本任务累计专注超预计提醒一次（切任务重置）。
- 节流：按规则冷却 5 分钟；滑动 60 分钟窗口 ≤4 次；设置「暂停监督 30 分钟」（`supervisionPauseUntil` 持久化，到期自动恢复）；休息中与暂停期间不提醒。
- 载体：`supervision:alert`（rule/app/level/text）→ 宠物气泡 + 提示音；`supervision:status`（正常/走神中/暂停）→ 顶条状态点。
- 入库：`supervision_events` 表（时间/规则/应用/级别）随迁移追加，每次提醒写一行。

### 应用图标（修复）
- 新命令 `get_shortcut_icon(path)`：`ExtractIconExW` + `DrawIconEx` + `CreateDIBSection` → 32×32 RGBA；前端 canvas → dataURL，按 target 缓存于 shortcuts store；`application` 卡片显示真实图标，提取失败/无图标回退 `app` 字形。

### 命令与事件
- 新命令：`save_task`/`set_current_task`/`set_focus_durations`/`set_distraction_lists`/`set_supervision_paused`/`resume_supervision`/`set_supervision_enabled`/`set_sound_enabled`/`set_show_topbar`/`record_focus_session`/`get_today_focus_summary`/`get_shortcut_icon`。
- 新事件：`focus:state_changed`（前端→Rust）、`supervision:alert`、`supervision:status`（Rust→前端，冒号命名）。
- DB 迁移：`focus_sessions`、`supervision_events` 两表（沿用 `schema_migrations` 机制）。
- Cargo features 追加：`Win32_UI_Shell`、`Win32_System_SystemInformation`（windows crate 既有依赖，无新增第三方依赖）。
