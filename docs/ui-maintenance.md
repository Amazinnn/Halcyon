# Focus UI 维护手册 v1

Date: 2026-08-14
Requirements: #126
ADR: ADR-0037（窗口注册表）、ADR-0038（Focus UI Kit）
Status: 随 C1/C4 变更交付；扩展性规划见 docs/architecture/extensibility-plan-v1.md

> 本文档是**维护流程**：新增控件、新增窗口、新增面板、修改 tokens 的步骤与
> 门禁。设计契约（哲学/规范）见 [ui-design.md](./ui-design.md)。

## 1. 新增一个控件（Kit 内）

1. 先看 ui-design.md §5 例外原则——尽量用现有组件组合，不要急着加组件。
2. 在 components/focus/ 新建 FocusXxx.vue：
   - props 按真实调用点设计（不做超前抽象）；
   - 样式只引用 tokens（styles.css :root）；
   - 保持原生语义（button/select/input）与 aria 属性。
3. 在 components/focus/focus-kit.test.ts 添加契约断言（?raw 源码断言，
   与项目既有测试风格一致；梯队 2 将替换为行为测试）。
4. 门禁：npm test -- --run && npm run build；git diff --check。
5. 提交 feat(ui-kit)；Eval 记录影响范围。

## 2. 新增一个窗口

按 ADR-0037 的三处声明 + 测试守护：

1. Rust：apps/desktop/src-tauri/src/window_spec.rs 的 WINDOW_SPECS 加一条
   （label/title/kind/default_rect/birth_rect/标志位——参照最接近的既有条目）。
2. 前端：src/lib/view-registry.ts 的 VIEW_REGISTRY 加一条（label/kind/title/
   icon/inTray/component/transparent），并新建对应视图组件。
3. capabilities：src-tauri/capabilities/default.json 的 windows 数组加 label
   （漏了会红：window_spec 测试断言注册表与 capabilities 精确一致）。
4. 若它是网格浮窗且需要托盘入口：inTray: true；折叠/恢复/吸附/无激活全部
   自动继承，不需要新生命周期代码。
5. 门禁：npm test、npm run build、cargo test --lib、openspec validate
   --specs --strict、git diff --check；rebuild 后手测。

## 3. 新增一个面板（声明 + 拼积木）

面板 = 窗口（按 §2 声明，Float + inTray）+ 只读查询 + 事件订阅 + Kit 拼装。
参考 apps/desktop/src/views/overview/OverviewPanelView.vue（C4 示例）：

1. 查询：优先复用现有只读 invoke（get_today_focus_summary、
   workflow_runs_recent、stats_dashboard …）；不要为面板新建写命令。
2. 订阅：先在 docs/architecture/event-streams-v1.md 查事件名与 Domain，
   只订阅面板真正需要的事件（薄窗口原则）；onBeforeUnmount 解绑。
3. 组装：FocusWindowFrame + FocusCard + Kit 控件；布局样式留在视图内，
   控件样式全部来自 Kit。
4. 手测：打开/关闭/实时更新/重启后布局保持。

## 4. 修改 Design Tokens

1. 在 styles.css :root 修改；先 grep 所有 var(--x) 引用评估影响面。
2. 全局生效的 token（--accent 等）改动 = 全产品视觉变更，必须走完整手测
   清单；新增 token 只影响新引用。
3. 更新 docs/ui-design.md §2 token 表；提交 docs。

## 5. 事件流维护

- 新事件：先在 event_bus.rs 加变体（含 domain() 映射），再更新
  docs/architecture/event-streams-v1.md 订阅矩阵，最后实现发出/监听。
- 新监听者：查矩阵确认事件 Domain；薄窗口（topbar/pet-bubble/overlay）
  不得初始化完整 agent store（App.vue 的 THIN_AGENT_LABELS）。

## 6. 门禁清单（交付前全跑）

- cd apps/desktop && npm test -- --run && npm run build
- cd apps/desktop/src-tauri && cargo test --lib
- cd packages/event-schema && npm test
- openspec validate --specs --strict
- git diff --check
- UI/Rust 改动：停 desktop.exe/watchdog 后 npm run tauri build -- --no-bundle
- 编号手测清单交付用户，验收通过后才 archive OpenSpec / 打 tag

## 7. 文档责任

- ui-design.md = 设计契约（哲学/规范/例外）；ui-maintenance.md = 维护流程。
- 架构决策进 docs/decisions/ADR-XXXX；需求原话进 requirements-verbatim.md；
  验收证据进 docs/evals/；事故进 docs/production-incidents.md。
- 扩展性路线图（C1-C4 与后续）见 docs/architecture/extensibility-plan-v1.md。
