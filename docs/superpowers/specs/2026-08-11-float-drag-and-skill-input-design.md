# 浮窗移动、可见 Skill 输入与 Claude 后台启动

- 状态：已获用户同意，待实现
- 关联：需求 #86/#87，INC-001，ADR-0026

## 背景与证据

内部浮窗在打开时的宿主窗口样式已经是 `WS_POPUP`，没有 caption 或 thick frame；问题只在移动后出现。`c906df5` 新增的浮窗 subclass 会对 `WM_ERASEBKGND` 返回已处理。透明 WebView 宿主在移动时因此不能完成背景擦除，最符合“初始正确、移动后出现蓝白边”的复现时序。现有样式探针未声明 DPI 感知，且不能用其虚拟化坐标代替视觉验收。

目前 Skill 选择会由后端读取 `SKILL.md` 并拼接到 Provider prompt。这与 #86 要求的“作为用户输入的一部分”相冲突。Claude 子进程通过默认 Windows 创建方式启动，因而会出现可见黑色控制台。

## 方案比较

1. 推荐：保留 `WS_POPUP`、无激活和 `WM_NCCALCSIZE`，但让 `WM_ERASEBKGND` 交由默认窗口过程处理；拖拽开始和结束各重新应用一次浮窗不变量。它只移除可归因的错误绘制拦截，不改变高频原生移动模型。
2. 每个拖拽轮询周期都重新设置样式和非客户区。它可能掩盖症状，但会增加 `SetWindowPos(SWP_FRAMECHANGED)` 负担和闪烁风险，因此不采用。
3. 移除整个 subclass。它会重新暴露历史非客户区残留风险，因此不采用。

## 设计

### 浮窗移动

- `WM_NCCALCSIZE` 仍返回 0，使客户区覆盖浮窗全域；`WM_ERASEBKGND` 不再被吞掉。
- 抽出一个只处理宿主窗口样式、subclass 和 `WS_EX_NOACTIVATE` 的浮窗不变量入口；现有创建、显示、恢复、置顶路径复用它。
- `drag_start` 与 `finalize` 调用该入口一次；轮询线程仍只进行异步原生位置变更，不重设样式。
- `window-style-probe.ps1` 增加进程 DPI 感知，并能按同一 HWND 在移动前后对比宿主和子窗口。该探针提供结构证据；用户在可见窗口上拖动五类浮窗的视觉检查仍是最终验收。

### 聊天和 Skill

- 聊天消息作者和窗口标题使用当前 `character.name`，即宠物名称；Provider 徽标仍只表示运行时（Codex 或 Claude）。
- 保持单个一次性 Skill。点击列表项目后，在同一个视觉输入框中显示不可编辑、较大、加粗的 `$skill-name` 块；正文输入区域紧随其后，块两侧通过稳定间距分开。
- `Backspace` 或 `Delete` 在正文为空或光标位于起始边界时，一次移除整个 Skill 块。用户不需要逐字符删除名称。
- 本轮发送的唯一文本为 `$skill-name  正文`；无 Skill 时为正文。该字符串既进入本地可见用户消息，也原样传入 Claude/Codex。
- 前后端删除 `skillName` RPC 字段及 `SKILL.md` 读取/提示词拼接。Skill 目录只用于列出可选项，真实 Provider 依自身原生 Skill 机制解析用户消息。
- 新建 ADR-0028，取代 ADR-0026 第 5 条的隐藏提示词注入语义；不改动当天会话、Provider 选择或工作流边界。

### Claude 后台启动

- 仅 Windows：Claude CLI 子进程使用 `CREATE_NO_WINDOW` 创建标志。
- stdin、stdout 和 stderr 仍为当前管道，常驻 stream-json、取消、会话恢复和错误传播保持不变。
- 不创建 Focus 自己的控制台，不传任何跳过权限的 CLI 参数，也不读取 Claude 凭证。

## 测试和验收

- Rust：非客户区映射不处理 `WM_ERASEBKGND`；浮窗不变量覆盖所有内部页；Skill 文本保持原样且无 `SKILL.md` 注入；Windows Claude 创建参数包含无控制台标志。
- 前端：宠物名作为 Agent 消息作者；选中 Skill 的发送文本为 `$skill-name  正文`；删除键原子清除 Skill；聊天模板渲染紧凑块而非隐藏选择状态。
- 自动验证：`npm test`、`npm run build`、`cargo test --lib`、event-schema 测试、`launch-focus.cmd rebuild`。
- 人工验收：反复打开、关闭、移动、缩放 chat/stats/music/pet/workflow，确认无蓝白条且隐藏按钮可点击；用 Claude 对话时不出现黑色终端；用 `focus-cli` Skill 完成一次只读真实 Provider 请求。
- 文档：追加本轮 Eval，更新 STATUS、INC-001、ADR-0028；在交付前运行 `git diff --check`。
