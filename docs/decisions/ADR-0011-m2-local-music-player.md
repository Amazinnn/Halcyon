# ADR-0011：M2 本地音乐播放器（替代 SMTC 方案）

- 日期：2026-08-07
- 状态：已接受
- 关联：需求 #22；取代此前未实现的 SMTC 系统媒体控制方向

## 背景
M2 第二步原计划接入 Windows 系统级媒体会话（GlobalSystemMediaTransportControlsSessionManager，SMTC）。用户随后明确需求：在一个特定文件夹存放 MP3 并播放，「就像真正的音乐播放器一样」。本地文件夹播放器优先于系统媒体控制。

## 决策
1. **播放引擎**：HTML5 `<audio>` + `convertFileSrc`（Tauri asset 协议）。已验证 tauri 2.11.5 asset 协议支持 HTTP Range（206 / Accept-Ranges），因此进度拖动（seek）可用。
2. **文件访问范围**：asset 协议默认 scope 仅 `$APPDATA/**`；用户选定音乐文件夹后，运行时 `asset_protocol_scope().allow_directory(dir, true)` 放行，并在应用启动时重新放行 + 重扫（scope 不跨进程持久）。
3. **元数据**：lofty（0.24，纯 Rust）读取标题/歌手/专辑/内嵌封面；解析失败回退文件名（去扩展名）+ 渐变封面。封面惰性读取（仅当前曲目），以 `data:` URI 传给前端。
4. **格式**：MP3 + FLAC + M4A（Chromium 原生可播）。
5. **播放模式**：单曲循环（重复当前曲）/ 列表循环（到尾回第一首）/ 列表顺序（到尾停止），控制条一个按钮三态循环切换。
6. **删除假播放器**：不再保留 FAKE_PLAYLIST 与前端自走 ticker；未选文件夹时音乐窗口为引导态。

## 影响
- 新增 Rust 依赖 lofty；新增 `settings.music_folder`；新增 `music_set_folder` / `music_list` / `music_cover` 命令。
- 不引入系统级媒体控制、不引入快捷键（需求 #19）、不做随机/音量/倍速。