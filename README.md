# Focus Desktop
> GitHub：https://github.com/Amazinnn/Halcyon.git（private）

本地专注桌面与 Agent 桌宠系统（Windows 优先，MIT License）。

**v1.7（当前）**：M1 桌宠 Pet Pack = 吸收 OpenAI hatch-pet 产物（`pet.json` + `spritesheet.webp`，固定 8×9 / 192×208 契约），精灵图帧播放器替换几何占位；文件夹导入 + 校验 + 持久化；桌宠 1×1/1×2/2×1/2×2 拖拽手柄、图案居中留边、对话按钮放空白区；ADR-0009。v1.6 基础：M3 Agent 接入 = 嵌入真实 Codex CLI（Claudian 式：新建/恢复/流式/停止对话，focus-cli 白名单+审计 skill，skills 透传，UI 选项；ADR-0007）。v1.5 基础：桌面图标**自由摆放**（12×8 网格吸附、拖拽网格线=整线连续渐变光晕（1.5 格衰减）、禁区保护、DB 持久化）、快捷方式 v2（新增 `url` 与 `internal` 类型，`.exe` 真实图标）、**打开窗口自动嵌入网格**（记住上次格位、冲突找最近空闲、豁免全屏/自有窗口）、新增 **`focus-cli` 本地控制面**（`timer/stats/desktop/apps` 四组命令，经 localhost TCP + token 供 M3 Agent 调用；契约见设计稿 §28 与 ADR-0006）、Dock「开始专注」改为纯触发器（点击即消失，idle/休息期再现）。v1.4.1 基础：配置单一事实源、置顶状态胶囊、黑/白名单应用选择器、进度环/滚动条。v1.4 基础：完整番茄钟 + 监督 V1 + 应用图标。v1.3 基础：专注桌面视觉精修（文件快捷区、三键 Dock、设置弹层）。v1.2.1 基础：Rust 光标轮询拖拽、SWCA Acrylic 毛玻璃去灰、桌宠身体可拖、`scripts/drag-probe.ps1`。

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
