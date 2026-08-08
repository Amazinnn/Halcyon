# Focus Desktop
> GitHub：https://github.com/Amazinnn/Halcyon.git（private）

本地专注桌面与 Agent 桌宠系统（Windows 优先，MIT License）。

**v1.11（当前）**：工作流退化为 Agent 日程工具 = 空角色合法化（save 不挂回 / 删 repair_orphan / list 空串=全部含未绑定，孤儿测试数据删库清理）、Agent 节点级目标（节点 `characterId` 参数 + 前端目标Agent下拉）、JSON 文档 + focus-cli `workflow read/create/update/delete --payload`（Agent 只经 CLI 增删改查 JSON，JSON=唯一真相、画布=渲染器）、`workflow:changed` 事件广播、白名单放行 workflow 全部子命令；需求 #67 + ADR-0020（推翻 ADR-0019 §4 孤儿挂回）。

**v1.10.5.1**：修复轮 = 存档角色绑定、连线箭头改 Vue Flow 原生 MarkerType.ArrowClosed、隐藏数字框 spinner；需求 #64–#66 + ADR-0019（Agent 概念定稿 + 工作流冻结，M5 待实施）。注意：其中「空角色挂默认 / 孤儿挂回」已被 v1.11 推翻。

**v1.10.5**：工作流画布收敛轮 = 7 类节点（移除气泡/IF）、参数面板词条卡片化 + 零变量、Agent=唯一输出通道 + 输出纪律提示词、自动保存竞态修复、绝对不向后兼容（启动 purge 旧工作流）；需求 #59–#63，ADR-0018。

**v1.10.4**：工作流 v2（8 类节点/Agent 填空槽/分支多路/允许成环/触发头/自动保存/窗口 6×5）+ 白框/亮度修复（#49/#50）+ 随机播放（#58）。v1.10.3.1：回退窗口层回归（SetWindowRgn→WebView2 透明背景、隐藏创建→构建期初始矩形）+ 工作流拖动网格预览（#47/#48）。v1.10.3：体验修复轮 v2 = 浮窗 SetWindowRgn 圆角裁剪消除双层框（#42）、桌宠/音乐缩放改最近角点吸附（#43）、工作流/统计 UI 结构收敛（#44）、内部页打开自动最近空位避让（#45）、启动叠窗闪现修复（#46），ADR-0015/0016。v1.10.2：体验修复轮 = position_window 位置原生 HWND（#35/#36）、根元素圆角裁剪（#37）、音乐三尺寸（#38）、缩放位移滑块（#39）、launch-focus.vbs（#40）、统计平滑曲线/5×4（#41）。v1.10.1：拖动卡死修复（原生 SetWindowPos + 预览节流 + poll 24ms + hang-detector STILL_HUNG）。v1.10：工作流入口/快速开关卡死/宠物 canvas tainted 修复 + monitor。v1.9.1：音乐窗口尺寸化 = 右下手柄 3×1~3×4 离散缩放（网格预览/冲突回弹/持久化）、禁用四个浮窗原生拉伸、行数≥3 才显示播放列表（避免半截列表）。v1.9：本地音乐播放器 = 选定文件夹（记住）扫描 MP3/FLAC/M4A，HTML5 audio 播放（asset 协议 Range 支持 seek），lofty 读标签/内嵌封面（回退文件名/渐变），列表+控制条，单曲循环/列表循环/列表顺序三模式（ADR-0011）。v1.8.2：专注落库按墙钟经过时间（跳过也记录；分心/空闲时段计入专注）。v1.8.1：统计链路加固 = 强制单实例（重复启动秒退）+ SQLite busy_timeout(5s) + 会话记录失败打点；修复偶发「专注已完成但统计全 0」的环境性丢记录。v1.8：M2 统计真实化 = 统计窗口接真实 SQLite 数据（30 天热力图 / 今日 24h 分布 / 连续天数 / 今日汇总），新增 focus-cli `stats dashboard`（并入白名单+审计）；分心/空闲/音乐类型暂为「暂无数据」占位。v1.7.2：交互修复收口 = 文件夹快捷方式经 explorer.exe 直开（绕开本机损坏的 Downloads shell GUID，不再弹「找不到应用程序」）、视图按钮点击式展开（外部点击关闭）、桌宠缩放回归修复（overlay WS_EX_NOACTIVATE + pointercancel/lostpointercapture 兜底）、网格亮部中心对齐。v1.7.1：桌宠交互修复 = 图样内只放精灵图 + 对话按钮悬停触发、缩放网格预览（按下即显/亮度跟随目标矩形/冲突标红/松手落库）、修复四尺寸拖拽移动、透明背景校验 + 背景淡化开关（ADR-0010）；外置 `pet-builder` skill（hatch-pet SenseNova 适配版，完整生成流水线）。v1.7 基础：M1 桌宠 Pet Pack = 吸收 OpenAI hatch-pet 产物（`pet.json` + `spritesheet.webp`，固定 8×9 / 192×208 契约），精灵图帧播放器替换几何占位；文件夹导入 + 校验 + 持久化；1×1/1×2/2×1/2×2 拖拽手柄；ADR-0009。v1.6 基础：M3 Agent 接入 = 嵌入真实 Codex CLI（Claudian 式：新建/恢复/流式/停止对话，focus-cli 白名单+审计 skill，skills 透传，UI 选项；ADR-0007）。v1.5 基础：桌面图标**自由摆放**（12×8 网格吸附、拖拽网格线=整线连续渐变光晕（1.5 格衰减）、禁区保护、DB 持久化）、快捷方式 v2（新增 `url` 与 `internal` 类型，`.exe` 真实图标）、**打开窗口自动嵌入网格**（记住上次格位、冲突找最近空闲、豁免全屏/自有窗口）、新增 **`focus-cli` 本地控制面**（`timer/stats/desktop/apps` 四组命令，经 localhost TCP + token 供 M3 Agent 调用；契约见设计稿 §28 与 ADR-0006）、Dock「开始专注」改为纯触发器（点击即消失，idle/休息期再现）。v1.4.1 基础：配置单一事实源、置顶状态胶囊、黑/白名单应用选择器、进度环/滚动条。v1.4 基础：完整番茄钟 + 监督 V1 + 应用图标。v1.3 基础：专注桌面视觉精修（文件快捷区、三键 Dock、设置弹层）。v1.2.1 基础：Rust 光标轮询拖拽、SWCA Acrylic 毛玻璃去灰、桌宠身体可拖、`scripts/drag-probe.ps1`。

- **权威设计文档**：[local-focus-desktop-agent-design-v0.2.md](./local-focus-desktop-agent-design-v0.2.md)
- **技术路线**：Tauri 2 + Vue 3 + TypeScript + Rust + SQLite

## 首轮（v0.2 计划任务 A–D）状态：✅ 完成

本仓库已执行设计稿 §24 的首轮任务，交付五件套：

| 交付物 | 位置 |
|---|---|
| 可运行原型（Spike） | `apps/desktop`（`npm run tauri dev`） |
| 架构调查报告 | `docs/architecture/spike-report-v0.md` |
| 许可证审计 | `docs/licenses/audit-v0.md` |
| 风险清单 | `docs/architecture/risks-v0.md` |
| 下一阶段拆分建议 | `docs/next-phase.md` |
| Windows 可行性报告 | `docs/architecture/windows-feasibility-v0.md` |
| 架构决策记录 | `docs/decisions/ADR-0001~0004` |
| 第三方声明 | `THIRD_PARTY_NOTICES.md` |

## 目录

```text
apps/desktop/          Tauri 2 + Vue 3 应用（Desktop / Chat / Stats / Music / Pet / Grid-Overlay / Topbar 七窗口）
packages/event-schema/ AgentEvent 协议 v1（JSON Schema + TS 类型 + fixtures，npm test 校验）
docs/                  审计、可行性、调查、ADR、风险与下一阶段
docs/architecture/evidence/visual-v1/   v1.2~v1.5 视觉截图（毛玻璃去灰、浮窗透明、计时/监督、置顶胶囊、v1.5 自由布局/渐变网格线/窗口嵌入）
```

## 一键启动

双击根目录 `launch-focus.cmd`（或运行 `launch-focus.cmd`）：
- 已有 release 可执行文件 → 直接秒开；
- 首次或没有 → 自动执行 release 构建（`--no-bundle`，约 1–2 分钟）后启动；
- 强制重新构建：`launch-focus.cmd rebuild`。

## 开发

```powershell
cd apps/desktop
npm install
npm run tauri dev        # 启动 Spike（四窗口）
```

```powershell
cd packages/event-schema
npm test                 # Ajv 校验 11 合法 + 4 非法 fixture；tsc 类型检查
```

```powershell
cd apps/desktop/src-tauri
cargo test --lib         # Mock Agent schema + drag + 布局/DB/CLI 单测
powershell -File scripts/drag-probe.ps1   # 拖拽回归探针（需先启动 release/desktop）
cargo run --bin workerw_probe   # WorkerW 桌面层探针（手动）
src-tauri\target\debug\focus-cli.exe timer status   # 本地控制面（需应用运行）
```

## 首轮硬性边界（未做）

内置独立 Agent（M3 改为嵌入真实 Codex CLI，见 ADR-0007）、plan mode/Diff/终端面板、Claude Code 接入（后续）、锁机、替换 Shell、私有虚拟桌面 API、真实音乐播放控制、浏览器追踪、云同步。

## 需求原话记录

用户每次提出的新需求以原话记录在 `docs/requirements-verbatim.md`（只追加、不改历史条目）。

## 诊断

应用运行时可查看浮窗/顶条实时状态：`focus-cli debug windows`（输出各浮窗 visible/collapsed、顶条可见性、grid 布局、active_drag）。
