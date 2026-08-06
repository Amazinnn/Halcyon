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


## v1.4.1 增补（配置单一事实源 · 独立置顶状态胶囊 · 应用列表选单 · 进度环/滚动条）

2026-08-05，修复 v1.4「修改不生效」类问题并落地顶条置顶、应用选择器与视觉细节。

### 配置单一事实源（修 bug）
- 根因：`focusMinutes/restMinutes/soundEnabled/showTopbar` 在 settings store（持久化）与 ui store（渲染）各存一份且从不互相同步——设置改时长/顶条三态/提示音不生效。
- 修复：ui store 删除这四个重复 state，改为 getter 实时委托 `useSettingsStore()`；`startFocus/startRest` 经 getter 读最新时长（"下一轮生效"真实落地）；`onPhaseChime` 与 `App.vue` 的 `supervision:alert` 提示音 gate 只读 settings store。`applyConfig` 仅保留 `focusSubtitle`。

### 独立置顶状态胶囊（新窗口 `topbar`）
- 新增 `topbar` 窗口（约 500×44，主屏顶部居中）：`transparent + always_on_top + skip_taskbar + decorations(false)` + `set_ignore_cursor_events(true)`（纯展示、点击穿透，绝不挡应用操作）。不参与 12×8 网格、不参与折叠。
- 新视图 `views/topbar/TopbarView.vue`：任务名 + Agent 状态点 + 「专注/休息 · mm:ss」+ 监督状态点；监听新事件 `focus:tick`（桌面 ui store 每秒广播 `{state, focusRemainingSec, restRemainingSec, paused, phaseDone}`）、`supervision:status`、`agent:event`（全局）。
- Rust 控制显隐：`apply_topbar_visibility`（`visible = mode=="on" || (mode=="auto" && state!="idle")`）在 `focus:state_changed` 监听与 `set_show_topbar` 命令内调用；启动时应用一次 + 1.2s 防御性复应用（防窗口注册竞态）。
- 桌面内联胶囊删除（DesktopView 不再渲染顶条）；capsule 信息由悬浮窗口独占。
- capabilities：windows 列表加 `"topbar"`，新增 `core:window:allow-set-ignore-cursor-events`。

### 黑/白名单应用选择器
- 新命令 `list_running_apps() -> Vec<String>`（`src-tauri/src/apps.rs`）：`EnumWindows` 收集可见顶层窗口 → 跳过本进程 pid → `QueryFullProcessImageNameW` 取 exe 名 → 去重（大小写不敏感）、排序、上限 100；复用既有 Cargo features，无新依赖。
- SettingsPopover 监督分区新增「运行中的应用」折叠列表：每行 = 进程名 + [黑] [白] 按钮，点击追加并立即持久化；*通配* 文本域保留。

### 进度环与滚动条
- 计时环：viewBox 120→360、`cx=cy=180, r=150`、stroke-width 4、`.ring` 300→360px、`ringCirc = 2π×150`——内径 ~295px 足够容纳 8 字符倒计时，数字不再与环重叠（截图验证：环心=屏心，文字在环内）。
- 全局 `::-webkit-scrollbar`：10px、透明轨道、`rgba(163,230,53,.18)` 圆角滑块（hover 提亮），替换默认白底灰条。

### 验证
- `cargo test --lib` 32 项全绿（新增 `list_running_apps` 冒烟/去重排序、`topbar_visible` 模式矩阵）；`npm run build` 与 `packages/event-schema npm test` 绿。
- 实机：idle 顶条隐藏、点「开始专注」后顶条出现且胶囊显示「专注中 · 0:39」实时倒计时；PrintWindow 捕获桌面窗口 hero 环居中；`v1.4.1-*` 截图存 `docs/architecture/evidence/visual-v1/`。

### v1.4.1 增补二（Logos 折叠胶囊启动竞态修复）
- 现象：用户截图中 Logos 窗口显示空态「—」，但 settings.json 已有 3 个折叠视图（对话/统计/音乐）。
- 根因：`apply_initial_layout` → `update_logos` 在 setup 阶段 emit `logos:update`，此时 Logos webview 尚未挂载监听，事件丢失；窗口被显示但内容停留在默认空数组 → 空态「—」（与顶条同类"启动期事件竞态"）。
- 修复：`LogosView.vue` 挂载时先经 `get_bootstrap` 读取持久化 `collapsed` 初始化胶囊列表，再监听 `logos:update` 处理运行时折叠/恢复。
- 验证：重启后 Logos 显示 3 个胶囊（对话/音乐/统计，截图 `v1.4.1-logos.png`）。

## v1.5.x 增补（网格光晕：整线连续渐变 · 1.5 格衰减 · 以实际悬浮位置为中心）

2026-08-06，按用户要求把浮窗拖拽时的网格预览从「每格一段统一颜色」改为「整条网格线沿长度方向的连续渐变」，并将渐变跨度从 2 格收窄为 1.5 格（需求原话见 `docs/requirements-verbatim.md` 第 9 条）。纯前端改动，无 Rust/协议/DB 变更。

### 渲染方式（`GridOverlayView.vue` 重写线条层）
- 删除 96 个 per-cell `.line`（每格一段统一颜色 + 相邻格双色边伪影）；改为 13 根整长竖线 + 9 根整长横线（1px，按 `i/12`、`j/8` 定位），每根线用 CSS `linear-gradient` 背景沿自身长度渐变（竖线 `to bottom`、横线 `to right`）。
- 渐变 stops 每 0.25 格一个（竖线 33 / 横线 49）；每个 stop 的 alpha = `max(0, 1 − max(dx, dy)/1.5)`，dx/dy 为该点在网格单位下到窗口**实际悬浮矩形**（Rust `grid:preview` 载荷 `floatRect`，物理像素 → 逻辑像素 → 网格单位）的 x/y 边缘距离；距离 ≥1.5 格 alpha=0（完全透明）。衰减线性、距离用切比雪夫（与既有光晕形状一致）。
- 每次 `grid:preview` 重算 22 根线的 gradient 字符串（~15ms 拖拽轮询自带平滑；渐变背景不可 CSS 插值，故不加 transition）。
- 保留 `.grid-marks` 层：被占红格 `.occ` + 吸附目标高亮 `.tgt` 不变；无 `floatRect` 时整线透明。
- 常量：`GRID_FALL_OFF = 1.5`、`GRID_STOP_STEP = 0.25`。

### 验证
- `npm run build`（vue-tsc）绿；`cargo test --lib` 无 Rust 改动保持绿；Node 逻辑单测：亮度随距离单调、1.5 格处=0、>1.5 全透明、0.25 格 stops 最大跳变 0.167（理论值）、随 floatRect 移动连续变化。
- release 重建（`npm run tauri build -- --no-bundle`），`launch-focus.cmd` 实机验收：拖拽时整条线从窗口附近亮到 1.5 格外透明、无阶梯、无短线条双色伪影、被占格标红与吸附高亮正常、光晕实时跟随。
