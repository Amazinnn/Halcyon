# Focus Desktop 当前状态（压缩交接页）

> 更新：2026-08-07（v1.7.2）。新对话请先读本页，再按需查阅 next-phase / requirements / ADR。
> 远程：github.com/Amazinnn/Halcyon（private，main）；本地 D:/Projects/Focus。

## 项目一句话

本地专注桌面 + Agent 桌宠系统（Windows 优先，MIT）。技术栈：Tauri 2 + Vue 3 + TypeScript + Rust + SQLite（apps/desktop）；AgentEvent 协议 v1（packages/event-schema）。

## 当前版本与已验证清单（v1.7.2）

- v1.7.2 交互修复（需求 #15/#16/#17）：
  - 文件夹快捷方式经 explorer.exe 直开：本机 Downloads 的 ShellExecuteW open 会解析到未注册的 shell GUID（52205fd8-…）并弹「找不到应用程序」；已实测打开正常「下载 - 文件资源管理器」窗口。
  - 视图按钮改点击式展开 + 外部点击关闭（原 mouseleave 悬停逻辑导致移向菜单即消失、无法点击）。
  - 桌宠缩放回归修复：overlay 创建时即设 WS_EX_NOACTIVATE（不抢激活、不丢指针捕获）；前端处理 pointercancel / lostpointercapture 防缩放句柄卡死；pet_resize_preview 隐藏不再要求 cols/rows。
  - 网格亮部中心与窗口对齐（启动布局正常时）。
- 先前已验证：#12（菜单单行/真实图标/单飞去重/后台线程/release 生产协议）
- v1.9.1（音乐窗口尺寸化）：右下手柄 3×1~3×4 离散缩放（网格预览/冲突回弹/持久化）；chat/stats/music/pet 禁用原生拉伸（尺寸唯一由网格控制，准则 #23）；行数≥3 才显示播放列表。
- v1.9（M2 本地音乐播放器）：选定文件夹扫描 MP3/FLAC/M4A，HTML5 audio + asset 协议（Range seek、运行时 scope 扩展），lofty 标签/封面（回退文件名/渐变），列表+控制条，单曲循环/列表循环/列表顺序三模式（ADR-0011）。
- v1.8.2（专注落库口径）：按墙钟经过时间记录（跳过也记录；分心/空闲时段计入专注；消除 2s 心跳粒度损失）。
- v1.8.1（统计链路加固）：强制单实例（CreateMutexW，重复启动秒退）+ SQLite busy_timeout(5s) + 会话记录失败打点；修复偶发「专注完成但统计全 0」的环境性丢记录。
- v1.8（M2 统计真实化）：统计窗口接真实 SQLite 数据（30 天热力图 / 今日 24h 分布 / 连续天数 / 今日汇总）；新增 focus-cli stats dashboard（白名单+审计）；分心/空闲/音乐类型暂为「暂无数据」占位。、#13（M1 Pet Pack：hatch-pet 契约播放器/导入/校验/四尺寸）、#14（桌宠 UX：纯精灵图/hover 对话/缩放网格预览/拖拽移动/透明背景校验+淡化开关/pet-builder skill）。
- 已知小问题（用户接受暂不处理）：#17 启动初期浮窗短暂堆叠后自动归位；极少数启动卡死（与 Agent 运行时探测相关）。

## 架构速览

- 7 窗口：desktop（全屏主界面：居中 2×5 快捷区 + 临时视图托盘 + 计时 hero + 三键 Dock）、chat/stats/music/pet（12×8 网格浮窗）、grid-overlay（拖拽/缩放预览，输入穿透）、topbar（置顶状态胶囊，点击穿透）。
- 拖拽：Rust 光标轮询（drag.rs，15ms，GetCursorPos + 屏幕钳制）；前端只发 drag_start/drag_end；网格光晕 = 整条网格线沿长度连续渐变（1.5 格衰减，floatRect 驱动）。
- 事件：Rust 事件总线（event_bus.rs）→ Tauri listen/emit；focus:tick、supervision:alert/status、grid:preview 等。
- 控制面：focus-cli（named pipe；timer/stats/desktop/apps/debug windows；ADR-0006）；M3 Agent 调用走白名单 + 审计（ADR-0007）。
- DB：SQLite（schema_migrations 迁移；app_shortcuts、ui_layouts、focus_sessions、supervision_events、spike_probes）。
- App data：%APPDATA%/com.focusdesktop.app/（settings.json + pet-packs/<id>/ + spike.db）。
- 桌宠素材：pet-packs/focus-demo-pet（透明背景已重建，validate 通过）；生成通道 ~/.codex/skills/pet-builder（SenseNova）。

## 命令

- 启动/重建：launch-focus.cmd；launch-focus.cmd rebuild 强制重建 release。
- 测试：cd apps/desktop && npm run build；cd apps/desktop/src-tauri && cargo test --lib；cd packages/event-schema && npm test。
- 控制面：apps/desktop/src-tauri/target/debug/focus-cli.exe timer status（需应用运行）。

## 已知问题与环境注意

- #17 启动堆叠（用户接受，暂不处理）。
- CDP 调试：启动前设 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222，偶发对新实例不生效（已多方式验证，非产品缺陷）。
- git push：直连易 reset；Clash 开启时走代理，否则 git -c http.proxy= -c https.proxy= push。
- cargo 拉新依赖：清 HTTP(S)_PROXY / ALL_PROXY，设 NO_PROXY=crates.io,index.crates.io,static.crates.io,github.com,*.crates.io。
- PowerShell 管道传中文会变 ?；写中文文件用 .NET WriteAllText（UTF-8 无 BOM）。
- 本机单显示器；多显示器 N/A。

## 下一步候选

- M1 剩余：系统托盘（可做）；全局快捷键绑定**暂缓**（需求 #19，开发完成前不绑）。Agent 状态模拟器完善。
- M2：统计真实化（v1.8）已实现；本地音乐播放器（v1.9）已实现。
- M3 剩余：plan mode / Diff / 终端面板 / Claude Code 接入。
- 悬而未决：图标区是否恢复拖动（当前为居中 2×5 固定）；多屏验证；毛玻璃在部分驱动下透明性（FOCUS_NO_ACRYLIC=1 降级开关）。

## 文档索引

- 需求原话：docs/requirements-verbatim.md（#1–#17，只追加、不改历史原话）。
- ADR：docs/decisions/ADR-0001~0010（0007=M3 Codex CLI；0008=release/启动非阻塞；0009=Pet Pack；0010=桌宠 UX+pet-builder）。
- 设计稿：local-focus-desktop-agent-design-v0.2.md（权威，保持原样、不移动、不改章节编号）。
- 其它：README.md（版本摘要）、docs/next-phase.md（路线）、docs/architecture/（spike/风险/可行性）。