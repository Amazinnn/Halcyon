# Focus UI 设计规范 v1（说明文档）

Date: 2026-08-14
Requirements: #126
ADR: ADR-0038（Focus UI Kit）
Status: 随 C1 变更交付；新控件/新面板以此为准

> 本文档是 Focus 桌面的**设计契约**：设计哲学、设计语言、控件规范与窗口
> 规范。写代码前先读这里；与实现冲突时以本文档为准并修正实现（或先改文档
> 再动手）。维护流程见 [ui-maintenance.md](./ui-maintenance.md)。

## 1. 设计哲学

1. **玻璃墨面，阳光绿**。窗口是透明的（毛玻璃透出桌面），内容承载在"墨色"
   半透明表面（--glass 系列）上；强调色是"阳光"石灰绿 --accent: #a3e635。
   深绿基底 + 石灰绿强调是全局身份，任何新控件不得引入第三种主色。
2. **简洁至上**。设置以外的地方尽可能简洁（需求 #76）；参数以词条卡片而非
   文本框承载（需求 #60）；不为"未来可能"预置 UI（YAGNI）。
3. **窗口尺寸影响一切**（准则 #23）。可缩放窗口（桌宠/音乐/工作流）的任何
   布局变化都要同时评估网格吸附、拖拽/缩放光晕、布局与字体。
4. **声明式组件优先**。控件只从 Focus UI Kit 拼装，不手写重复样式；新窗口
   视觉自动一致（需求 #126）。
5. **原生对齐**。浮窗宿主圆角与 Windows DWM 原生圆角对齐
   （--window-host-radius: 10px）；网页层不得再画一层"框"。

## 2. 设计语言（Design Tokens）

所有视觉值只存在于 apps/desktop/src/styles.css 的 :root。**组件与视图
不得内联硬编码颜色/圆角/尺寸**。

| Token 组 | 变量 | 说明 |
| --- | --- | --- |
| 基底 | --bg-0..3 | 深绿层级背景 |
| 玻璃 | --glass / --glass-strong / --glass-border | 墨色表面与描边 |
| 强调 | --accent / --accent-bright / --accent-wash / --accent-glow | 阳光绿与淡染 |
| 语义 | --warn / --err | 琥珀警告、珊瑚错误 |
| 文本 | --text-hi / --text-mid / --text-low | 三级文本 |
| 间距 | --sp-1..7（4→48px） | 间距阶梯 |
| 圆角 | --r-sm 8px / --r-md 12px / --r-lg 16px / --r-pill / --window-host-radius 10px | 圆角阶梯 |
| 动效 | --t-fast 120ms / --t-base 200ms / --t-slow 320ms / --ease-out | 时长与缓动 |
| 字号 | --fs-xs 10px / --fs-sm 11px / --fs-md 12px / --fs-lg 13px | 控件字号阶梯 |
| 阴影 | --shadow-pop / --shadow-float | 浮层阴影 |
| 层级 | --z-tray 12 / --z-backdrop 20 / --z-popover 30 | z-index 阶梯 |

## 3. 控件规范（Focus UI Kit）

组件目录 apps/desktop/src/components/focus/。所有组件：
- 只消费 tokens；scoped 样式内不出现硬编码色值。
- 保留原生的 aria-* 语义（按钮 aria-pressed、分组 role="group"）。
- 禁用态可见（opacity 0.45 + cursor: default）。
- 键盘可达：原生元素（button/select/input）优先，不做自定义键盘模拟。

| 组件 | 用途 | 变体/属性 |
| --- | --- | --- |
| FocusButton | 按钮 | variant: default（透明+悬停描边）/ glass（墨底）/ ghost（无边框）/ accent（实底）/ danger（红色）；size: tight/xs/sm/md/lg/icon（历史尺寸阶梯）；off 半透明 |
| FocusToggle | 胶囊开关 | modelValue + label；on 态 accent 实底 |
| FocusSegmented | 分段选择 | options[{label,value}] + modelValue；variant: soft（淡染选中）/ solid（实底选中）/ pill（胶囊容器+亮底选中） |
| FocusInput | 文本/数字输入 | type: text|number、min/max、placeholder；number spinner 已全局隐藏 |
| FocusSlider | 滑条 | modelValue、min/max/step、disabled；accent 拇指 |
| FocusSelect | 原生下拉 | options[{label,value}] + modelValue + disabled |
| FocusCard | 玻璃卡片 | title/note 可选 + 默认插槽 |
| FocusWindowFrame | 浮窗标题栏 | title + collapsible；含拖动（useGridDrag）、置顶、折叠（150ms 防抖） |

## 4. 窗口规范

- 窗口声明见 Rust WINDOW_SPECS（window_spec.rs，ADR-0037）：label/kind/
  默认格位/标志位一处声明；前端视图映射见 view-registry.ts。
- 浮窗宿主统一：隐藏创建、无激活、透明背景、--window-host-radius 圆角
  （configure_float_host）；网页层透明窗口类 transparent-window。
- 网格浮窗（Float）才拥有折叠/恢复/吸附/缩放生命周期；气泡（Bubble）与
  顶条（Topbar）是点击穿透的信息层，不参与网格。
- 托盘条目 = view-registry.ts 中 kind: "float" 且 inTray: true 的窗口。

## 5. 布局与文本宽度准则（需求 #130）

1. **文本项最小宽度**：承载多行文本的 flex 项必须给最小宽度
   （`--text-min-row: 120px`）或 ellipsis 保护；禁止裸 `min-width: 0` 让
   名字/说明被压成竖排窄列。
2. **输入与下拉最小宽度**：FocusInput/FocusSelect 强制
   `--ctrl-min-input: 96px` / `--ctrl-min-select: 88px`，窄容器里也不得
   被压扁。
3. **行容器换行**：内容可能溢出的 flex 行（按钮组、管理行）必须
   `flex-wrap: wrap` 并留 `row-gap`；按钮 `white-space: nowrap` 的溢出
   由容器换行兜底。
4. **长文本**：单行元数据用 ellipsis；正文/说明文字要占满可用宽度
   （flex: 1 + 最小宽度），不强行限宽。
5. **设置弹窗（300px）**：Agent 命名输入独占一行；管理行按钮允许换行。

## 6. 动态尺寸与溢出准则（需求 #131）

1. **内容自适应**：短内容输入用 `autosize`（field-sizing: content），
   宽度随内容在 `--ctrl-min-input-auto`（40px）与容器 100% 之间变化。
2. **硬约束**：任何动态控件 `max-width: 100%`——绝不突破外层窗口边框；
   网格窗口定尺寸保证不挤小其他窗口。
3. **溢出处理三态**：
   - 输入框内文字超出 = 原生隐藏（光标处可见），不破框；
   - 显示型文本超出 = 右缘渐隐（`.fade-x`/mask 渐变）替代硬截断；
   - 多行输入区超出 `--ctrl-max-input-h`（约 4 行）= 内部滚动。
4. **有限纵向增长**：多行输入允许随内容长高，但设上限，超出内部滚动。
5. **同类控件同高**：同一输入行内的下拉/按钮与输入框同高对齐。

## 7. 例外原则

领域专用控件允许不入 Kit：MusicView 播放进度条、PetView 对话按钮、
TopbarView 胶囊。例外必须记录在本节；新领域控件先尝试 Kit 组合，确实无法
组合时才写专用样式。

## 7. 验收标准

- 新控件/新窗口必须消费 tokens（grep 检查无硬编码色值）。
- 前端门禁：npm test -- --run、npm run build（vue-tsc 强制类型）。
- 视觉回归：手测清单对照既有窗口；可缩放窗口按准则 #23 检查。