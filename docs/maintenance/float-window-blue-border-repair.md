# 浮窗拖动蓝白边框维修档案

## 范围

只处理内部浮窗在首次打开正常、开始拖动后出现蓝白边框或白色客户区的
生产回归。范围包括 `chat`、`stats`、`music`、`pet` 和 `workflow`；不混入
桌面锁、Agent、Skill 或工作流功能修改。

## 用户可见症状

- 打开浮窗时可能正常。
- 一旦移动浮窗，窗口边缘出现蓝白条或白边。
- 蓝白边框出现后，窗口隐藏按钮可能无法点击。
- 历经多次修复后仍复发，因此静态样式位、自动截图或脚本移动都不能作为
  视觉修复结论。

## 已知事实

- 当前工作区与用户运行版本均为 `main` 的 release 构建。
- 浮窗使用透明 WebView、原生 `SetWindowPos` 拖动和无激活显示。
- 最近实现额外安装了原生 subclass，并返回 `WM_NCCALCSIZE -> 0` 与
  `WM_ERASEBKGND -> 1`；该路径尚未通过真实鼠标拖动验收。
- 此事故是 `INC-001`，当前状态为 `Open / S2`。

## 维修纪律

1. 先回溯每一代浮窗创建、移动和非客户区处理，找出首次引入复发的变化。
2. 每次只验证一个因果假设，并先写会失败的回归测试。
3. 自动测试只证明代码路径；不承担视觉验收。
4. 需要确认鼠标拖动、边框、隐藏按钮、恢复或重叠时，停止自动操作并请用户在
   当前 release 中手工验证。
5. 用户手工报告通过前，事故保持 `Open`，不提交为视觉修复完成。

## 本轮调查

状态：进行中。

### 2026-08-11 Baseline

- Git 分支为 `main`，但当前 release 是从含未提交浮窗改动的工作区重建。因而它
  与用户打开的应用不是不同分支，却不能再把“已提交版本”与“正在运行版本”混为一谈。
- 相对 `main` 的最近窗口提交 `288f23c`，未提交改动已移除拖动、定位、置顶和显示
  路径中反复执行的 `enforce_float_invariants`。当前候选路径只在隐藏创建时配置一次。
- 这份候选路径仍保留 `WM_NCCALCSIZE -> 0` 和 `WM_ERASEBKGND -> 1` 的 subclass。
  这是本轮唯一待证伪的架构假设：它是否仍会在真实鼠标拖动后造成宿主与 WebView
  的非客户区重绘冲突。
- 用户已在当前 release 基线复现：`chat`、`stats`、`music`、`pet`、`workflow` 初次
  打开正常；拖动过程中和鼠标松开后均出现窗口上端蓝白条。

下一步：回溯拖动开始、原生移动、松开吸附和非客户区消息的实际顺序，为单一假设写
失败测试并实施最小修改。

### 2026-08-11 Candidate Change

- 失败测试已确认旧行为：浮窗会对 `WM_NCCALCSIZE` 返回 `0`。
- 当前候选移除了 `SetWindowSubclass`、`WM_NCCALCSIZE` 与
  `WM_ERASEBKGND` 的处理；之后所有非客户区消息均直接交给 Windows/Tauri。
- 保留原生拖动的 `SetWindowPos(...SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS)`，
  以及隐藏创建时一次性的 `WS_POPUP`/`WS_EX_NOACTIVATE` 配置。
- 已知风险：若没有 subclass 的 `WS_POPUP` 宿主重新出现旧白边，必须作为新的、
  独立假设处理，不得重新把消息拦截塞回拖动路径。

### 2026-08-11 Automated Verification

- `float_hosts_delegate_nonclient_messages_to_native_windowing` 先在旧行为下失败，
  移除拦截后通过。
- `npm run build`、`cargo test --lib`（173 passed）、event-schema 测试和
  `launch-focus.cmd rebuild` 均通过。
- release 已重建。此后停止自动窗口操作，等待用户的真实鼠标拖动验收。

### 2026-08-11 Candidate Result: Failed

- 用户真实验收否决了“完全移除 subclass”候选：浮窗出现上端和左端轮廓，WebView
  左上角还出现系统窗口标题文字。
- 这证明当前 `WS_POPUP` 一次性配置没有单独形成正确的无非客户区宿主；此前的
  subclass 也不能作为答案，因为其路径会在拖动后形成蓝白条。
- 下一阶段不是继续加/删消息拦截，而是回溯曾稳定的窗口创建、样式应用与拖动边界，
  找到两种失败状态之间缺失的原生配置条件。

### 2026-08-11 Candidate Change 2

- 历史比对确认 `157d173` 的直接 `GWLP_WNDPROC` 路径曾在创建期捕获原过程和客户区，
  后续拖动只移动 HWND；`c906df5` 才将它改为 managed subclass 并在生命周期反复重配。
- 当前候选恢复直接窗口过程，但只安装一次：`WM_NCCALCSIZE -> 0`、
  `WM_ERASEBKGND -> 1`，其余消息转发原 Tauri 过程。
- 不使用 `SetWindowSubclass`；显示、拖动、松开、定位、恢复和置顶不会重新安装过程、
  重写样式或发送 `SWP_FRAMECHANGED`。
- 失败测试 `float_host_keeps_a_full_client_rect_without_default_background_erase`
  已在“无过程”候选下失败，恢复创建期过程后通过。

### 2026-08-11 Candidate 2 Result: Partial

- 用户真实验收：左侧原生轮廓已消失，但上端条仍存在。
- 这证明创建期全客户区处理解决了横向以外的部分，不足以解释顶部残留。
- 在确认顶部条究竟属于系统 caption 还是 Focus 自身网页头部前，不再新增 Windows
  消息处理或样式补丁。

### 2026-08-11 Source Identification

- 用户确认顶部残留带有黑色窗口标题文字；它是 Windows 原生 caption，不是 Focus
  网页组件。
- 这将问题收敛到宿主收到激活/非客户区重绘消息后的处理：当前直接窗口过程只拥有
  `WM_NCCALCSIZE` 与 `WM_ERASEBKGND`，其余消息仍交给原 Tauri 过程。
- 下一条待证伪假设：拖动期间的 `WM_NCACTIVATE` 被原过程交给默认非客户区绘制，
  即使 `WM_NCCALCSIZE` 已令 client 覆盖全窗口，仍把 caption 画回顶部。候选只能在
  创建期已经安装的同一个窗口过程中对该消息返回已处理；不重写样式、不发送
  `SWP_FRAMECHANGED`，也不在拖动路径安装或替换过程。

### 2026-08-11 Candidate 3 Prepared for Manual Gate

- 回归测试先以 `WM_NCACTIVATE -> None` 失败，再在仅增加该消息的已处理返回后通过。
- `npm run build`、`cargo test --lib`（173 项）、event-schema 测试与
  `launch-focus.cmd rebuild` 均已通过。
- 自动验证到此为止。必须由用户在最新 release 中真实拖动五类浮窗，才能判定该
  原生 caption 是否已停止重绘。

### 2026-08-11 Candidate 3 Result and New Visual Defect

- 用户已在最新 release 中确认：拖动后的原生标题条与标题文字均不再出现。该部分
  验收通过。
- 同次验收发现独立缺陷：原生 SWCA 毛玻璃合成层仍为矩形，伸出网页圆角之外，形成
  四角直角凸出。HTML/CSS 的 `overflow: hidden` 只能裁切 WebView 内容，不能裁切
  宿主 HWND 的毛玻璃层。
- 新假设将只评估 Windows DWM 的宿主圆角偏好；不恢复历史 `SetWindowRgn` 裁剪，也
  不触碰已经通过验收的窗口过程、样式或拖动路径。

### 2026-08-11 Native Acrylic Corner Candidate

- DWM `DWMWA_WINDOW_CORNER_PREFERENCE = ROUND` 已接入现有 `configure_float_host()`：
  只在五类浮窗隐藏创建时调用一次。
- 失败先行测试验证 attribute `33` 与 `ROUND` 值 `2`；实现后通过。它不新增
  `SetWindowRgn`、不更改 `GWLP_WNDPROC`，也不修改拖动、显示或尺寸路径。
- `npm run build`、`cargo test --lib`（174 项）和 event-schema 测试已通过；release
  重建与用户视觉验收待执行。

### 2026-08-11 CSS Radius Alignment Candidate

- 用户确认 DWM 圆角已去除四角矩形外溢，但与网页外层 `16px` 圆角仍有轻微弧度差。
- 此候选只将五类浮窗与透明 WebView 裁切统一改用
  `--window-host-radius: var(--r-md)`（12px）；内部卡片继续使用原有圆角，
  不触碰已验证的窗口过程、DWM 调用、样式位或拖动代码。
- `float-host-radius.test.ts` 先失败（未定义该 token），再通过；自动验证和 release
  重建完成后，必须由用户检查真实窗口四角与一次拖动后的标题条回归。

### 2026-08-11 Pet-Baseline Parameter Tuning

- 用户确认修复已基本可用，但四类带网页描边的浮窗仍与桌宠的原生曲线有极小不重合；
  桌宠本身没有该视觉问题。
- 桌宠没有带描边的网页外壳，因此以它的原生曲线为视觉基准：共享
  `--window-host-radius` 从 12px 微调至 10px。内部卡片、DWM、窗口过程与拖动路径不变。
- 用户明确要求本次只重建，不另做自动或人工视觉验证。

## 2026-08-11: Grid glow center regression (#102)

After the accepted caption repair, the first drag after opening or restoring a
float could place the grid glow about two cells left of the visible client area.
The release had two coordinate sources: positioning used a live client/outer
frame conversion, while drag preview and snap carried separately sampled
origin and dimensions. v1.12.10 introduces one ClientGeometry snapshot from
GetWindowRect, GetClientRect, and ClientToScreen; preview, snap, and
final placement derive from it. Manual mouse-drag verification remains required.