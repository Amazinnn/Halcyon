# 扩展性总体规划 v1（UI / CLI / 事件流 / 桌宠）

Date: 2026-08-14
Requirements: #125, #126, #127
ADR: ADR-0037（窗口注册表，本规划的哲学基石）
Status: 已批准；C1–C4 按路线图逐个开 OpenSpec 变更实施

> 本文档是把「未来场景 → 扩展点 → 现状缺口 → 推荐改动」画成地图并定义实施
> 路线图的权威文件。每个实施变更（C1–C4）以本文档为指引；实施中发现偏差，
> 先改本文档再动手。本文档不替代 ADR（架构决策）与 Eval（验收证据）。

## 1. 扩展性原则（宽松边界定义）

1. **声明式 > 硬编码**。新增窗口/命令/事件/控件应「声明 + 组件」，而非修改
   既有逻辑。窗口注册表（ADR-0037）已验证：声明表 + 一致性测试守护，新增
   窗口只动三处声明，不碰创建逻辑。同一哲学推广到 CLI、事件、UI 控件。
2. **领域模块边界**。每个领域 = 独立 Rust module + 前端 lib + 事件命名空间；
   不建插件 API（用户：插件"不一定"），模块边界就是未来的插件边界
   （next-phase 扩展方向 5）。
3. **宽松的度**：为**已选定场景**留缝（声明表、注册点、命名空间）；为未选
   场景设 YAGNI 护栏，不预建抽象。判据：一个扩展点必须有「用户选定场景」的
   直接对应，否则砍掉。
4. **行为零变化**是重构轮的红线（对齐冻结基线 v1.12.10-restructure-freeze）；
   每个变更的可见行为必须与实施前逐位一致，由门禁 + 编号手测兜底。

## 2. 用户选定场景与对应扩展点

| 场景（用户选定） | 需要的扩展点 | 优先级 |
| --- | --- | --- |
| ① 新自定义面板/窗口（如「某应用使用情况看板」） | 窗口声明（已就位 #125）+ UI Kit 控件 + 面板查询命令 + 事件订阅 | P0/P1 |
| ② Agent 通过 focus-cli 做更多事（自主调度/第三方集成） | CLI 命令注册表（声明/白名单/审计/帮助一处化） | P0 |
| ③ 多窗口协同/新事件流（按领域分组、薄窗口订阅） | 事件命名空间 + 领域分组 + 订阅最小集 | P1 |
| 桌宠扩展（预留：新状态/新交互/新包类型） | 状态映射声明化、交互扩展点、包适配器边界 | P2 |

## 3. 四领域扩展点地图

### 3.1 UI（C1，P0）

- 现状：`styles.css`（101 行）有 tokens 雏形（色板/间距/圆角/动效），但
  控件样式在 ≥5 个文件重复且不一致（`.switch.on` 两套、`.seg.on` 三套、
  `select` 三个类名、`.btn/.ghost` 三处）；无设计契约文档。
- 缺口：控件不可复用、新面板视觉无法自动一致、无设计规范。
- 推荐改动（C1）：补全控件级 tokens（字号阶梯/阴影/z-index/控件尺寸）；
  新建 8 个组件（FocusButton/FocusToggle/FocusSegmented/FocusInput/
  FocusSlider/FocusSelect/FocusCard/FocusWindowFrame），全部消费 tokens；
  替换 SettingsPopover/DesktopView/WorkflowView/ChatView/WindowHeader 的
  重复样式；产出 docs/ui-design.md（设计哲学/规范）与
  docs/ui-maintenance.md（维护手册）。
- 例外：MusicView 播放进度条、PetView chat-btn、TopbarView 胶囊属领域专用
  控件，不入 Kit（ui-design.md 记录例外原则）。

### 3.2 CLI（C2，P0）

- 现状：新增命令需改 3 处——`cli.rs::handle_request` 大 match（~160 行分发）、
  `agent_whitelisted` 硬编码白名单、`focus-cli` 客户端（help/参数映射）；
  `debug windows` 硬编码 `["chat","stats","music","pet"]`（未吃上窗口注册表）。
- 缺口：命令路由是面条式 match；白名单/审计/帮助与分发不同源，易漏改。
- 推荐改动（C2）：CommandSpec 声明表——一条声明含命令路径/参数/处理器/
  白名单标记/审计；服务端分发、Agent 白名单、客户端帮助全部由声明派生；
  `debug windows` 改用 `window_spec` 的浮窗集合。JSON 协议保留（可加可选
  version 字段，不做 v2）。

### 3.3 事件流（C3，P1）

- 现状：CoreEvent 无领域分组；topbar/pet-bubble 白初始化完整 agent store。
- 缺口：新窗口要订阅事件难以知道该听什么；薄窗口承载过多初始化。
- 推荐改动（C3）：事件命名空间表（focus:/stats:/agent:/workflow:/
  supervision:）+ CoreEvent 按领域分组；薄窗口订阅模式——topbar/pet-bubble
  只订阅最小事件集，保留编译期类型安全（event-schema 已是 TS 类型源）。

### 3.4 桌宠（P2 预留，本轮不实施）

- 现状：pets.rs 已模块化良好（加载/校验/校准/调色板/状态映射/导入，大量测试）。
- 缺口（边界层）：状态集写死（focus/rest/sleep…）、交互方式写死（hover 对话
  按钮）、包格式适配器逐个手写（官方 Hatch/focus-hatch-pet/退役 draft）。
- 预留扩展点（写入设计文档，不实施）：状态→动画映射声明化（新状态不碰引擎）、
  交互扩展点（点击/双击/拖拽之外的新交互注册）、包适配器注册边界。
- 触发条件：用户提出新状态/新交互/新包类型需求时，按扩展点实施。

## 4. 实施路线图（每个 = 独立 OpenSpec 变更）

| 变更 | 内容 | 依赖 | 优先级 |
| --- | --- | --- | --- |
| C1 | Focus UI Kit + docs/ui-design.md + docs/ui-maintenance.md | #126 | P0 |
| C2 | CLI CommandSpec 命令注册表 | #127, ADR-0037 | P0 |
| C3 | 事件流领域分组 + 薄窗口订阅 | C1 的 store 收拢可并行 | P1 |
| C4 | 自定义面板窗口框架（面板 = 窗口声明 + 查询命令 + 事件订阅 + UI Kit） | C1/C2/C3 | P1 |

每个变更沿用既有铁律：需求原话 → OpenSpec propose → apply → 门禁 + 编号
手测 → 用户验收 → sync + archive。

## 5. 非目标（YAGNI 护栏）

- 不建插件 API、不引入热加载、不做跨进程协议改造（focus-cli JSON 协议保留）。
- 不为「新宠物包类型/新交互方式」预建抽象（P2 触发后再做）。
- 梯队 3 架构级（薄窗口模式已由 C3 部分覆盖；bundle 分包）另行单独论证。
- 不推倒重写、不换框架（next-phase 明确的非目标）。

## 6. 验收门槛

- 每个变更：`npm test -- --run`、`npm run build`、`cargo test --lib`、
  `packages/event-schema npm test`、`openspec validate --specs --strict`、
  `git diff --check`、release rebuild、编号手测清单（用户逐项验收）。
- 视觉/行为零变化由「实施前截图/行为对照 + 手测」确认，不以自动化替代。
