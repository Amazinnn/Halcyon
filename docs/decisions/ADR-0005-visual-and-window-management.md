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