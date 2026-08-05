# Focus Desktop

本地专注桌面与 Agent 桌宠系统（Windows 优先，MIT License）。

**v1.4（当前）**：完整番茄钟计时（双向下倒计时、暂停/继续、跳过、进度环、到点提示音+气泡、时长预设 25/5·50/10·90/15+自定义、今日汇总、`focus_sessions` 写库）+ **监督 V1 软限制**（分心超时/空闲/任务超时三条规则、升级提醒、暂停 30 分钟、`supervision_events` 入库、顶条状态点）+ 应用快捷方式真实 exe 图标 + 顶条胶囊默认隐藏/专注时自动显示。v1.3 基础：专注桌面视觉精修（玻璃胶囊顶条、文件快捷区、三键 Dock、设置弹层）。v1.2.1 基础：Rust 光标轮询拖拽（防卡死）、自实现 SWCA Acrylic 毛玻璃去灰、桌宠身体可拖、拖拽回归探针 `scripts/drag-probe.ps1`。

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
apps/desktop/          Tauri 2 + Vue 3 应用（Desktop / Chat / Stats / Music / Pet / Logos / Grid-Overlay 七窗口）
packages/event-schema/ AgentEvent 协议 v1（JSON Schema + TS 类型 + fixtures，npm test 校验）
docs/                  审计、可行性、调查、ADR、风险与下一阶段
docs/architecture/evidence/visual-v1/   v1.2~v1.4 视觉截图（毛玻璃去灰、浮窗透明、v1.4 计时/监督态）
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
cargo test --lib         # Mock Agent 事件 schema 校验单测 + drag 钳制单测
powershell -File scripts/drag-probe.ps1   # 拖拽回归探针（需先启动 release/desktop）
cargo run --bin workerw_probe   # WorkerW 桌面层探针（手动）
```

## 首轮硬性边界（未做）

真实 Agent 接入、锁机、替换 Shell、私有虚拟桌面 API、真实音乐播放控制、浏览器追踪、云同步。