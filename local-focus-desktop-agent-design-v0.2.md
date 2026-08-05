# 本地专注桌面与 Agent 桌宠系统：完整设计方案

> 文档版本：v0.2  
> 状态：讨论成果固化稿 / 可交付本地 Codex 继续细化  
> 目标平台：Windows 11 优先  
> 项目许可证目标：MIT  
> 核心技术方向：Tauri 2 + Vue 3 + TypeScript + Rust + SQLite  
> 更新时间：2026-08-05

---

# 1. 项目定位

本项目不是普通的番茄钟、桌宠、AI 聊天客户端或应用启动器，而是一个本地运行的“专注桌面操作层”。

它在 Windows 上提供一套自定义桌面环境，包含：

1. 可配置的桌面快捷方式与应用启动入口。
2. 可切换的专注模式与应用限制。
3. 软件使用时间、专注时间和任务工时统计。
4. 月度专注热力图、单日时间分布等统计视图。
5. 可接入不同本地 Agent 运行时的统一对话面板。
6. 专门维护日记、任务、复盘和个人数据的生产力 Agent。
7. 可读取、修改用户指定 Markdown 文件与目录的受控工作区。
8. 与 Agent 状态联动的桌面宠物。
9. 简洁的音乐播放浮窗，以及音乐类型、播放时段和专注关联统计。
10. 定时检查任务进度、发出声音和短提示的监督系统。
11. 本地优先的数据存储、权限控制和可审计操作。

一句话定义：

> 一个以“专注工作”为核心、由桌面宠物承载 Agent 状态、同时整合时间追踪、任务文件和应用入口的本地桌面软件。

---

# 2. 已确定的核心原则

## 2.1 本地优先

- 软件本体在本机运行。
- UI 不依赖远程网页。
- 不要求启动 localhost 网站供用户访问。
- 数据默认保存在本地 SQLite 和用户指定的 Markdown 工作区。
- 不上传应用使用记录、文件路径、任务内容或 Agent 日志。
- 第三方模型请求只通过用户主动配置的 Agent 或 Provider 发出。

## 2.2 MIT 优先

以下内容必须满足：

- 主仓库采用 MIT License。
- 直接 fork、复制或修改的项目必须明确允许 fork、修改和再分发。
- 优先采用 MIT 依赖。
- 双许可证依赖必须明确选择 MIT 条款使用。
- 无 LICENSE、Source Available、自定义模糊许可证或限制衍生开发的项目不得进入代码库。
- 保留所有必要版权声明和许可证文本。
- 建立 `THIRD_PARTY_NOTICES.md`。
- 外部 Agent 可作为用户自行安装的程序调用，不代表其自身许可证并入本项目。

## 2.3 Agent 无关

界面和核心协议不得写死 Claude Code。

统一设计为：

- OpenCode Adapter
- Codex Adapter
- Claude Code Adapter
- Custom CLI Adapter
- Future Agent Adapter

首发优先支持 OpenCode，因为其开源且 MIT；其他 Agent 后续以外部程序方式适配。

## 2.4 不依赖不稳定能力完成基础功能

Windows 真实虚拟桌面管理存在版本兼容风险。

因此产品必须提供两种模式：

### 稳定模式：Desktop Overlay

- 创建全屏、无边框的本地窗口作为专注桌面画布。
- 它覆盖原桌面，但不替换 Windows Shell。
- 启动的应用仍是正常 Windows 窗口。
- 这是默认模式，必须稳定可用。

### 实验模式：Windows Virtual Desktop

- 创建或切换到指定 Windows 虚拟桌面。
- 将启动的应用移动到该桌面。
- 使用独立兼容层处理 Windows 版本差异。
- 此模式失败时自动退回 Overlay，而不是导致程序不可用。

---

# 3. 产品界面总览

系统由六类窗口组成。

```text
DesktopWindow     全屏专注桌面
PetWindow         透明置顶桌宠
PanelWindow       Agent 对话 / 时间统计复用面板
LockWindow        锁机或限制遮罩
MusicWindow       极简音乐控制与播放状态浮窗
SettingsWindow    设置、权限、Agent、规则与资源管理
```

另外还有：

```text
TrayController    系统托盘
BackgroundCore    后台计时、进程监测、调度与数据服务
AgentHost         Agent 适配器与会话管理
```

---

# 4. DesktopWindow：专注桌面

## 4.1 视觉结构

```text
┌─────────────────────────────────────────────────────────────┐
│  当前任务：实现统计模块     专注 00:42:18     休息 12:00    │
│                                                             │
│   [VS Code]   [Obsidian]   [浏览器]   [文件夹]              │
│                                                             │
│   [课程项目]  [日记]       [任务]     [统计]                │
│                                                             │
│                      可自由布置的桌面区域                     │
│                                                             │
│                                  ┌─────────────┐            │
│                                  │ Agent 桌宠   │            │
│                                  └─────────────┘            │
│                                                             │
│  Dock：运行中应用 | 开始专注 | 当前 Agent | 设置 | 退出     │
└─────────────────────────────────────────────────────────────┘
```

## 4.2 桌面图标能力

每个图标对应 `DesktopShortcut`：

- 启动 `.exe`。
- 启动 `.lnk`。
- 打开文件或目录。
- 打开 URL。
- 启动带参数的命令。
- 启动项目工作区。
- 调用内部页面，如统计、任务、日记。
- 调用 Agent Skill 或预设任务。

可配置字段：

```ts
interface DesktopShortcut {
  id: string
  name: string
  iconPath?: string
  type: "application" | "file" | "folder" | "url" | "internal" | "agent-action"
  target: string
  arguments?: string[]
  workingDirectory?: string
  runAsAdmin: boolean
  groupId?: string
  position: { x: number; y: number }
  allowedInFocusMode: boolean
}
```

交互能力：

- 拖拽排列。
- 网格吸附。
- 框选。
- 文件夹或分组。
- 自定义图标。
- 右键菜单。
- 搜索应用。
- 显示运行状态。
- 启动后自动归入专注桌面。

## 4.3 桌面不是应用容器

VS Code、Obsidian、浏览器等不会嵌入 DesktopWindow。

正确机制是：

1. 点击桌面图标。
2. 系统启动真实应用进程。
3. 记录应用启动事件。
4. 实验模式下尝试移动该窗口到 Focus 虚拟桌面。
5. DesktopWindow 保持在最底层，效果类似自定义桌面背景与 Dock。

---

# 5. PetWindow：Agent 桌宠

## 5.1 产品定义

桌宠不是独立娱乐模块，而是 Agent 的最低干扰表现形态。

同一个 Agent 有三种 UI 层级：

```text
宠物图标       最轻量状态表达
短气泡         一两句提醒或反馈
正式对话框     完整会话、文件、工具与 Diff
```

宠物可以：

- 待机。
- 移动。
- 播放状态动画。
- 显示短气泡。
- 显示红点、进度或错误标志。
- 被点击、拖动、右键操作。
- 点击后打开 Agent 面板。
- 根据当前 Agent 切换宠物形象。

## 5.2 Agent 状态映射

统一状态枚举：

```ts
type AgentState =
  | "offline"
  | "idle"
  | "thinking"
  | "reading"
  | "searching"
  | "editing"
  | "running"
  | "testing"
  | "waiting_permission"
  | "waiting_user"
  | "success"
  | "warning"
  | "error"
  | "cancelled"
```

宠物状态：

```ts
interface PetReaction {
  agentId: string
  state: AgentState
  animation: string
  bubble?: {
    text: string
    priority: "low" | "normal" | "high" | "critical"
    durationMs: number
  }
  sound?: string
  badge?: string
  progress?: number
}
```

## 5.3 气泡内容原则

气泡只显示短文本，不直接复制 Agent 输出。

示例：

- “正在读取任务文件。”
- “测试快跑完了。”
- “这里需要你确认。”
- “连续十分钟没有检测到进展。”
- “今天已经专注两小时。”
- “修改完成，去看看 Diff 吧。”

禁止进入气泡的内容：

- API Key。
- 完整文件路径。
- 环境变量。
- 多行代码。
- 命令完整输出。
- 用户隐私内容。
- 未经摘要的 Agent 推理或日志。

## 5.4 宠物资源格式

V1 使用 Sprite Sheet，不采用 Live2D SDK。

```text
pet-pack/
├─ manifest.json
├─ spritesheet.webp
├─ preview.webp
├─ sounds/
│  ├─ notify.wav
│  └─ success.wav
└─ LICENSE
```

Manifest 示例：

```json
{
  "schemaVersion": 1,
  "id": "example.pet",
  "name": "Example Pet",
  "author": "Author",
  "license": "MIT",
  "bubbleAnchor": { "x": 0.5, "y": 0.05 },
  "animations": {
    "idle": { "frames": [0, 1, 2, 3], "fps": 4, "loop": true },
    "thinking": { "frames": [4, 5, 6, 7], "fps": 8, "loop": true },
    "success": { "frames": [8, 9, 10], "fps": 10, "loop": false }
  }
}
```

导入时必须：

- 校验 Manifest。
- 限制资源大小。
- 禁止执行脚本。
- 显示作者和许可证。
- 拒绝许可证不明的公开分发包。
- 用户私人导入素材可以本地使用，但不得自动打包进项目发布版。

---


# 6. MusicWindow：极简音乐浮窗

## 6.1 产品定义

MusicWindow 是一个独立、低干扰的长条浮窗，用于显示当前播放状态并提供最基础的媒体控制。

它不承担完整音乐播放器的职责，而是作为本项目的“专注音乐控制器”和“音乐数据采集入口”。

推荐视觉结构：

```text
┌──────────────────────────────────────────────────────────┐
│ [封面]  曲名 · 艺术家      ─────●────────  02:13 / 04:06 │
│         [上一首] [暂停/播放] [下一首]                    │
└──────────────────────────────────────────────────────────┘
```

可选收缩状态：

```text
[封面]  ▶  02:13
```

V1 必备能力：

- 显示专辑封面。
- 显示曲名、艺术家和专辑。
- 上一首。
- 播放/暂停。
- 下一首。
- 当前进度与总时长。
- 拖动进度条跳转。
- 始终置顶。
- 屏幕边缘吸附。
- 自动隐藏。
- 与 PetWindow、PanelWindow 避让。
- 全屏应用或勿扰模式下自动降级为收缩状态。

## 6.2 音乐来源抽象

不要把播放器写死为某个音乐软件。

统一定义：

```ts
interface MusicSourceAdapter {
  readonly id: string
  readonly displayName: string

  detect(): Promise<boolean>
  getPlaybackState(): Promise<PlaybackState>
  play(): Promise<void>
  pause(): Promise<void>
  next(): Promise<void>
  previous(): Promise<void>
  seek(positionMs: number): Promise<void>
  events(): AsyncIterable<MusicEvent>
}
```

首版建议支持两种来源：

### System Media Adapter

通过 Windows 系统媒体会话获取当前播放器信息和控制能力。

适用于：

- Spotify。
- 系统音乐播放器。
- 支持 Windows 媒体会话的第三方播放器。
- 部分浏览器媒体页面。

优点：

- 不必重新实现完整播放器。
- 能统一控制多个播放器。
- 不要求读取音乐文件本体。

局限：

- 不同播放器暴露的专辑、流派和进度信息完整度不同。
- 某些播放器不支持 Seek。
- 浏览器标签页可能只能识别到粗略媒体信息。

### Local Library Adapter

由本软件播放用户指定的本地音乐目录。

适用于：

- 用户希望完全离线。
- 需要稳定的曲目元数据。
- 需要更准确的音乐类型统计。
- 需要自定义专注歌单。

V1 可先实现 System Media Adapter；Local Library Adapter 可在后续加入。

## 6.3 音乐数据统计

音乐统计不能只记录“播放器打开了多久”，应记录实际播放事件。

核心事件：

```ts
interface MusicPlaybackEvent {
  id: string
  sourceId: string
  trackId?: string
  title: string
  artist?: string
  album?: string
  genre?: string
  startedAt: string
  endedAt: string
  listenedMs: number
  trackDurationMs?: number
  completedRatio?: number
  wasSkipped: boolean
  focusSessionId?: string
  taskId?: string
  appContext?: string
}
```

建议统计：

1. 每日、每周、每月音乐播放时长。
2. 曲目、歌手、专辑播放排行。
3. 音乐类型分布。
4. 一天中不同时间段的听歌分布。
5. 专注时与非专注时的听歌差异。
6. 每种音乐类型对应的平均专注时长。
7. 每种音乐类型对应的中断次数。
8. 每个任务常搭配的音乐类型。
9. 跳歌率与完整播放率。
10. 专注开始前、进行中、结束后的播放习惯。
11. 音乐音量与专注数据的关系（仅在系统可稳定读取时）。
12. 无音乐、白噪音、纯音乐、人声音乐等条件下的专注表现。

“音乐类型”支持多层来源：

```text
用户手动标签
→ 本地文件元数据
→ 播放器提供的 Genre
→ Agent 辅助归类
```

Agent 自动归类只能生成建议，用户可以修改。

## 6.4 音乐与专注数据关联

音乐事件必须能关联：

- `focus_session_id`
- `task_id`
- `project_id`
- `agent_session_id`
- `activity_context`

这样可以回答：

- “我写代码时听什么最久？”
- “哪类音乐下的专注时段最稳定？”
- “听人声歌曲时，我是否更容易切换窗口？”
- “过去一个月，晚上听纯音乐时效率如何？”
- “哪些歌单经常出现在高质量专注时段？”

必须强调：

> 这些结果只能描述个人历史数据中的相关性，不能自动推断音乐造成了效率变化。

## 6.5 浮窗互斥与避让

MusicWindow 与其他窗口的默认关系：

- PetWindow：同时显示，但不能重叠。
- Chat Panel：MusicWindow 自动吸附到 Panel 上方或下方。
- Statistics Panel：可以保持显示。
- LockWindow：只保留暂停或停止入口，其他控制隐藏。
- 全屏应用：自动收缩。
- 用户播放视频时：可按规则隐藏音乐浮窗。
- 勿扰模式：隐藏曲名，仅保留播放状态图标。


# 7. PanelWindow：统一浮动面板

PanelWindow 是一个可复用的悬浮窗口，而不是建立多个互相冲突的窗口。MusicWindow 保持独立，因为它需要持续显示播放状态和媒体控制。

支持模式：

```text
collapsed      收缩为按钮或窄条
chat           Agent 对话
statistics     时间统计
task           当前任务
permission     工具权限审批
diff           文件修改预览
```

## 6.1 对话模式

```text
┌──────────────────────────────────────┐
│ Agent：OpenCode ▼       会话：课程项目│
├──────────────────────────────────────┤
│ [对话] [文件] [Diff] [工具] [日志]    │
│                                      │
│ Agent：正在读取 tasks/today.md        │
│                                      │
│ 用户：继续处理第三项任务              │
│                                      │
│ Agent：已完成修改，等待确认。          │
│                                      │
├──────────────────────────────────────┤
│  附加文件  /命令      输入消息…… 发送 │
└──────────────────────────────────────┘
```

必须支持：

- 流式文本。
- 中断生成或停止 Agent。
- 新建、恢复和切换会话。
- 当前工作目录。
- 当前 Agent 与模型标识。
- 工具调用卡片。
- 文件读取状态。
- 文件修改 Diff。
- 逐项接受、全部接受、拒绝和撤销。
- 权限申请。
- Agent 日志。
- 长期指令、项目指令和临时指令编辑。

## 6.2 统计模式

```text
┌──────────────────────────────────────┐
│ 专注统计         日 / 周 / 月 / 年    │
├──────────────────────────────────────┤
│ 本月专注热力图                       │
│ ■ □ ■ ■ □ ■ ...                     │
│                                      │
│ 今日时间分布                         │
│ 00 03 06 09 12 15 18 21 24          │
│       ███    ███████   ██            │
│                                      │
│ 专注 3h42m  分心 1h08m  空闲 2h10m   │
│                                      │
│ 应用类别 / 项目工时 / 任务完成度      │
└──────────────────────────────────────┘
```

核心图表：

1. GitHub 风格月度/年度专注热力图。
2. 单日 24 小时专注分布。
3. 应用使用时间排行。
4. 分类占比。
5. 专注与分心时间趋势。
6. 任务预计与实际工时。
7. 连续专注天数。
8. 每次专注时段长度分布。
9. Agent 活跃时间与人工工作时间对照。
10. 锁机、偏离和提醒事件时间线。

---

# 8. 宠物、气泡与面板的互斥规则

状态由单一 `UiCoordinator` 管理。

```ts
interface UiState {
  panelMode: "closed" | "chat" | "statistics" | "task" | "permission" | "diff"
  petVisible: boolean
  speechBubbleVisible: boolean
  doNotDisturb: boolean
  lockActive: boolean
}
```

规则：

## 7.1 Chat 打开

- 宠物继续显示和播放动画。
- 普通气泡关闭。
- Agent 完整文本只进入 Chat。
- 权限、错误等高优先级事件用红点、声音或轻量图标表达。
- 不在宠物旁重复显示同一句话。

## 7.2 Statistics 打开

- 宠物继续显示。
- 低频短气泡可以保留。
- 统计面板不应被气泡遮挡。
- 用户可设置“查看统计时静音”。

## 7.3 面板关闭

- 宠物恢复短气泡。
- Agent 完成、需要权限、监督提醒等事件可以冒泡。
- 气泡必须限流，避免每个工具调用都发言。

## 7.4 锁机状态

- 普通气泡关闭。
- 宠物可显示休息、睡觉或锁定动画。
- 只允许显示剩余时间、解锁要求和紧急入口。

---

# 9. AgentHost：多 Agent 植入层

## 9.0 Agent 的产品角色

本项目虽然复用 OpenCode、Codex、Claude Code 等编程 Agent 的底层能力，但面向用户暴露的不是“通用编程助手”，而是一个专门的个人生产力维护 Agent。

默认职责：

- 维护日记。
- 维护任务。
- 整理项目记录。
- 生成每日和每周复盘。
- 查询专注数据。
- 查询软件使用数据。
- 查询音乐播放数据。
- 分析任务、专注和音乐之间的历史关联。
- 根据用户设定的规则检查进展。
- 在需要时通过桌宠、声音或通知提醒。
- 对 Markdown 文件生成可审批的修改。

默认不鼓励：

- 无关的软件开发任务。
- 任意系统修改。
- 未授权目录访问。
- 大规模 Shell 自动化。
- 代替用户执行不可逆操作。

编程 Agent 只是运行时底座；产品层通过系统提示词、工具权限和工作区配置，将其约束为“日记、任务和数据维护 Agent”。

## 9.0.1 预设提示词与路径配置

日记和任务文件不需要为每一种格式开发专门解析器。

设置界面提供：

```ts
interface ProductivityAgentProfile {
  id: string
  name: string
  adapterId: string
  workspaceRoots: string[]
  journalPath?: string
  taskPath?: string
  projectPaths?: string[]
  instructionText: string
  supervisionInstruction?: string
  allowedDataScopes: DataScope[]
}
```

用户可以在设置中填写：

```text
日记文件路径：
D:/Notes/journal/daily.md

任务文件路径：
D:/Notes/tasks/today.md

补充指令：
- 日记按日期二级标题组织。
- 新内容追加到当天标题下。
- 不要删除历史记录。
- 完成任务时把 [ ] 改为 [x]。
```

宿主在启动 Agent 会话时自动注入：

- 授权工作区根目录。
- 日记路径。
- 任务路径。
- 用户预设提示词。
- 可用数据工具。
- 文件修改审批规则。

因此 Agent 可以自行搜索、理解和维护 Markdown，而不要求程序预先理解所有日记模板。

但仍需由宿主保证：

- 路径必须存在或由用户明确允许创建。
- 路径必须位于授权目录。
- Agent 不能只凭提示词绕过文件权限。
- 所有写入仍走 Diff、审批和审计。
- 文件路径变更时及时更新 Agent Profile。


## 8.1 统一接口

```ts
interface AgentAdapter {
  readonly id: string
  readonly displayName: string

  detect(): Promise<AgentDetectionResult>
  startSession(options: StartSessionOptions): Promise<AgentSession>
  resumeSession(sessionId: string): Promise<AgentSession>
  send(sessionId: string, input: AgentInput): Promise<void>
  cancel(sessionId: string): Promise<void>
  approve(requestId: string, decision: ApprovalDecision): Promise<void>
  dispose(sessionId: string): Promise<void>

  events(sessionId: string): AsyncIterable<AgentEvent>
}
```

## 8.2 统一事件

```ts
type AgentEvent =
  | { type: "session.started"; sessionId: string }
  | { type: "message.delta"; text: string }
  | { type: "message.completed"; text: string }
  | { type: "tool.started"; tool: string; inputSummary: string }
  | { type: "tool.completed"; tool: string; resultSummary: string }
  | { type: "file.read"; path: string }
  | { type: "file.changed"; path: string; diffId: string }
  | { type: "permission.requested"; requestId: string; risk: string }
  | { type: "status.changed"; state: AgentState }
  | { type: "session.completed"; outcome: string }
  | { type: "session.error"; message: string }
```

所有 Agent 的原始协议先进入 Adapter，再转换为统一事件。

Panel、Pet、Scheduler、日志和通知层只能消费统一事件，不能直接解析某个 Agent 的私有输出。

## 8.3 首发 Agent

### OpenCode

定位：首个完整适配器。

原因：

- MIT。
- 开源。
- 适合研究和修改。
- 可以作为 AgentHost 的首个真实验证对象。

### Custom CLI

定位：最低通用能力。

用户配置：

```json
{
  "command": "example-agent",
  "args": ["--json"],
  "workingDirectory": "D:/workspace",
  "inputProtocol": "stdin-lines",
  "outputProtocol": "json-lines"
}
```

### Claude Code / Codex

定位：后续外部适配器。

原则：

- 不复制或捆绑其程序。
- 检测用户本机安装。
- 调用其 CLI 或公开接口。
- 适配器代码采用 MIT。
- 会话和权限行为必须按各自公开能力实现。
- 不假定不同 Agent 支持完全一致的功能。

---

# 10. Markdown 工作区与日记 Agent

## 9.1 推荐目录

```text
workspace/
├─ journals/
│  └─ 2026-08-05.md
├─ tasks/
│  ├─ inbox.md
│  └─ today.md
├─ projects/
│  └─ project-name.md
├─ reviews/
│  ├─ daily/
│  └─ weekly/
├─ instructions/
│  ├─ AGENT.md
│  ├─ journal.md
│  └─ task-manager.md
└─ attachments/
```

## 9.2 受控工具

Agent 可获得：

- 列出工作区文件。
- 搜索 Markdown。
- 读取文件。
- 创建文件。
- 追加日记。
- 修改任务复选框。
- 生成补丁。
- 查询专注统计。
- 查询软件使用统计。
- 查询音乐播放统计。
- 查询专注、任务、应用与音乐之间的关联数据。
- 查询当前任务。
- 启动或停止专注计时。
- 请求系统提醒。

默认禁止：

- 任意磁盘访问。
- 任意 Shell。
- 删除工作区外文件。
- 修改锁机核心规则。
- 修改程序自身。
- 静默启动高风险进程。
- 将本地内容自动发送给未授权服务。

## 9.3 修改流程

```text
Agent 生成修改
→ 生成 Diff
→ PanelWindow 展示
→ 用户接受/拒绝
→ Rust 文件服务原子写入
→ 生成审计日志
```

低风险操作可选择自动批准：

- 向当天日记追加文本。
- 勾选任务。
- 写入 Agent 自己的日志目录。

---

# 11. 软件计时与活动追踪

## 10.1 时间类型必须分开

```text
ActivityTime      前台应用真实使用时间
FocusTime         用户主动开启的专注时段
TaskTime          绑定具体任务的工时
AgentTime         Agent 会话运行时间
IdleTime          无输入活动时间
BlockedTime       被限制或锁机的时间
```

不能把“软件开着”直接当成“用户正在工作”。

## 10.2 采集内容

默认采集：

- 进程名。
- 应用名称。
- 前台窗口切换时间。
- 会话开始和结束时间。
- 空闲状态。
- 用户设定的应用分类。
- 绑定的当前任务。

可选采集：

- 窗口标题。
- 浏览器域名。
- 项目路径。

默认不采集：

- 键盘输入内容。
- 剪贴板内容。
- 屏幕截图。
- 文档正文。
- 密码字段。
- 浏览器完整 URL。
- 私密窗口标题。

## 10.3 采样策略

- 使用 Windows 前台窗口事件为主。
- 低频心跳用于修正丢失事件。
- 空闲阈值默认 5 分钟，可配置。
- 睡眠、锁屏、休眠后停止计算活跃时间。
- 进程退出时补齐事件终点。
- 所有时间统一存 UTC，UI 转本地时区。

## 10.4 应用分类

```text
productive
neutral
distracting
blocked
unknown
```

分类规则支持：

- 进程名。
- 可执行文件路径。
- 窗口标题正则。
- 浏览器域名。
- 当前任务覆盖规则。
- 时间段规则。

---

# 12. 专注监督与定时提醒

## 11.1 Scheduler 不是 Agent 提示词

“每十分钟检查一次进度”必须由本地调度器实现。

```text
Scheduler
→ 读取当前任务
→ 查询最近活动
→ 查询文件修改时间
→ 判断是否偏离
→ 必要时调用 Agent 做语义判断
→ 播放声音
→ 宠物发出短气泡
→ 写入监督事件
```

## 11.2 两级判断

### 一级：本地规则

无需调用模型：

- 连续使用分心应用超过 N 分钟。
- 当前任务绑定的应用未使用。
- 工作区文件长时间无修改。
- 专注计时仍在运行但电脑已空闲。
- 任务已超出预计时长。
- 到达休息时间。

### 二级：Agent 判断

仅在必要时调用：

- 当前行为是否可能属于任务的一部分。
- 文件改动是否代表实质性进展。
- 应该提醒、鼓励还是保持安静。
- 将提醒压缩成一两句。

## 11.3 提醒策略

- 冷却时间。
- 每小时最大次数。
- 严格模式和温和模式。
- 勿扰时段。
- 会议/全屏应用自动静音。
- 失败后不重复轰炸。
- 用户可以一键暂停监督。

---

# 13. 锁机与限制系统

分三阶段实现。

## 12.1 V1：软限制

- 提醒。
- 宠物警告。
- Panel 显示偏离。
- 打开指定应用时弹出确认。
- 专注桌面只展示允许的应用。

## 12.2 V2：覆盖限制

- 多显示器全屏遮罩。
- 倒计时。
- 密码或延迟解锁。
- 允许紧急退出。
- 重启后恢复限制状态。

## 12.3 V3：系统级强化

- 后台守护进程。
- 防止普通关闭绕过。
- 应用启动拦截。
- 专门的 Windows Service。
- 单独的“严格模式”安全说明。

硬限制不能由 Agent 直接启动或永久修改，必须经过用户预设或明确确认。

---

# 14. 技术架构

## 13.1 推荐技术栈

```text
Frontend
- Vue 3
- TypeScript
- Pinia
- Vue Router
- Chart.js
- Calendar Heatmap Component

Desktop Runtime
- Tauri 2
- Rust
- WebView2（Windows 系统组件）

Storage
- SQLite
- SQL migration
- JSON settings
- Markdown workspace

Windows Integration
- windows-rs
- Optional VirtualDesktop Helper
- Process and foreground-window monitoring
- System tray
- Notifications
- Global shortcuts
- Audio output
```

Vue 并不“太简单”。

它只负责：

- UI。
- 状态展示。
- 图表。
- 桌面图标布局。
- 对话渲染。
- Diff 视图。
- 宠物动画。

真正的系统能力全部放在 Rust 后端：

- 进程。
- 窗口。
- 文件。
- SQLite。
- Agent 子进程。
- 定时器。
- 统计聚合。
- 权限。
- 锁机。
- 虚拟桌面。

## 13.2 进程结构

```text
focus-desktop.exe
├─ Rust Core
│  ├─ Window Manager
│  ├─ Activity Tracker
│  ├─ Focus Engine
│  ├─ Scheduler
│  ├─ File Service
│  ├─ SQLite Repository
│  ├─ Permission Broker
│  └─ Agent Supervisor
│
├─ WebView: DesktopWindow
├─ WebView: PetWindow
├─ WebView: PanelWindow
└─ Optional Sidecars
   ├─ opencode
   ├─ custom-agent
   └─ virtual-desktop-helper.exe
```

## 13.3 多窗口同步

禁止窗口之间直接互调。

统一使用 Rust Event Bus：

```text
Agent Adapter ─┐
Tracker ───────┤
Scheduler ─────┼→ Core Event Bus → Desktop / Pet / Panel / Tray
Focus Engine ──┤
File Service ──┘
```

---

# 15. 数据库设计

## 14.1 核心表

```sql
app_shortcuts
app_rules
activity_events
focus_sessions
task_time_entries
tasks
agent_profiles
agent_sessions
agent_events
permission_requests
pet_profiles
pet_events
music_sources
music_tracks
music_playback_events
music_tags
supervision_rules
supervision_events
ui_layouts
settings
schema_migrations
```

## 14.2 关键实体

### activity_events

```text
id
started_at
ended_at
process_name
app_name
window_title
category
is_idle
task_id
source
```

### focus_sessions

```text
id
started_at
ended_at
planned_minutes
actual_active_minutes
task_id
mode
status
interruption_count
```

### agent_sessions

```text
id
adapter_id
external_session_id
workspace_path
started_at
ended_at
status
title
```

### agent_events

```text
id
session_id
timestamp
event_type
summary
payload_json
sensitivity
```


### music_playback_events

```text
id
source_id
track_id
title
artist
album
genre
started_at
ended_at
listened_ms
track_duration_ms
completed_ratio
was_skipped
focus_session_id
task_id
agent_session_id
```

### music_tracks

```text
id
source_track_id
title
artist
album
duration_ms
genre
user_tags_json
cover_cache_path
first_seen_at
last_seen_at
```

### pet_profiles

```text
id
name
package_path
license
author
selected_agent_id
enabled
```

---

# 16. 项目目录建议

```text
focus-desktop/
├─ apps/
│  └─ desktop/
│     ├─ src/
│     │  ├─ windows/
│     │  │  ├─ desktop/
│     │  │  ├─ panel/
│     │  │  ├─ pet/
│     │  │  └─ settings/
│     │  ├─ components/
│     │  ├─ stores/
│     │  ├─ router/
│     │  ├─ charts/
│     │  └─ types/
│     └─ src-tauri/
│        ├─ src/
│        │  ├─ activity/
│        │  ├─ agents/
│        │  │  ├─ mod.rs
│        │  │  ├─ opencode.rs
│        │  │  ├─ custom_cli.rs
│        │  │  └─ protocol.rs
│        │  ├─ desktop/
│        │  ├─ focus/
│        │  ├─ files/
│        │  ├─ permissions/
│        │  ├─ pets/
│        │  ├─ music/
│        │  ├─ scheduler/
│        │  ├─ storage/
│        │  ├─ windows/
│        │  └─ main.rs
│        └─ migrations/
├─ packages/
│  ├─ agent-protocol/
│  ├─ pet-format/
│  ├─ event-schema/
│  └─ shared-types/
├─ assets/
├─ docs/
│  ├─ architecture/
│  ├─ decisions/
│  ├─ licenses/
│  └─ threat-model/
├─ LICENSE
├─ THIRD_PARTY_NOTICES.md
├─ CONTRIBUTING.md
└─ README.md
```

---

# 17. 参考项目与复用边界

## 16.1 可作为直接参考或 fork 来源

### Tauri

用途：

- 多窗口本地桌面应用。
- Rust 后端。
- 系统托盘、窗口和插件生态。

许可：

- 项目同时提供 MIT 与 Apache-2.0 许可文件。
- 本项目明确选择 MIT 路径。

### BongoCat

用途：

- Tauri + Vue 桌宠工程结构。
- 透明宠物窗口。
- 跨平台窗口行为。
- 自定义模型导入。
- 离线桌宠逻辑。

策略：

- 优先研究并抽取桌宠窗口、动画和模型格式。
- 不保留与本产品无关的键鼠展示功能。
- 保留原作者许可证和版权说明。

### OpenPets

用途：

- Agent → 宠物状态事件。
- `react` / `say` 类型交互。
- 插件权限。
- 气泡敏感信息过滤。
- Agent Hook 与 MCP 集成思路。
- 宠物包和插件 SDK 思路。

策略：

- 不采用 Electron 外壳。
- 只迁移或重新实现协议、安全和事件层。
- 复用代码时逐文件记录来源。

### MScholtes/VirtualDesktop

用途：

- Windows 虚拟桌面控制思路。
- Windows 版本兼容参考。

策略：

- 先做独立兼容适配器。
- 不让核心产品依赖其成功运行。
- 每次 Windows 大版本升级执行兼容测试。

### AppGroup

用途：

- Windows 应用启动器。
- 快捷方式与分组交互。
- 图标提取和启动参数思路。

策略：

- 参考 Windows 应用发现、图标和启动逻辑。
- Vue UI 重新设计。

### OpenCode

用途：

- 首发 Agent。
- 完整开源 Agent 实现和协议验证。

策略：

- 优先采用服务或公开协议接入。
- 不修改其核心也能完成初版时，保持松耦合。
- 用户可以替换为其他 Agent。

### Coworker

用途：

- 本地 Agent 桌面宿主。
- 文件夹权限。
- 多 Provider。
- Skills 和操作审批产品交互。

策略：

- 研究 AgentHost、权限和会话设计。
- 不采用其 Electron/React UI。
- 不直接把整个项目作为主基座。

### Pomotroid

用途：

- 计时器状态机。
- Tauri/Rust 计时工程。
- 专注统计交互参考。

策略：

- 研究计时、托盘和通知。
- 统计数据模型需按本项目重新设计。

## 16.2 只观察、不复制

- AGPL 桌面环境。
- 非 MIT 的 Shell Replacement。
- Live2D Cubism SDK。
- 来源不明的宠物素材。
- 许可证不明确的 Codex 宠物包。
- 未公开稳定协议的私有 Agent UI 实现。

---

# 18. 安全与权限模型

## 17.1 原则

- 最小权限。
- 明确授权。
- 操作可见。
- 修改可撤销。
- 路径范围可配置。
- Agent 与系统控制解耦。

## 17.2 风险等级

```text
LOW
读取授权目录、查询统计、追加日志

MEDIUM
修改 Markdown、启动允许应用、创建文件

HIGH
删除文件、执行 Shell、访问工作区外目录、联网发送文件

CRITICAL
修改锁机规则、安装系统服务、管理员权限、注册表和系统启动项
```

High 和 Critical 默认必须人工确认。

## 17.3 审计

记录：

- 哪个 Agent。
- 哪个会话。
- 调用了什么工具。
- 修改了哪个文件。
- 用户是否批准。
- 修改前后摘要。
- 是否回滚。
- 是否触发宠物气泡或提醒。

---

# 19. 非功能需求

## 18.1 性能目标

- 后台空闲 CPU 接近 0%。
- PetWindow 动画可降帧。
- 普通状态内存目标低于 Electron 同类产品。
- Activity Tracker 不使用高频轮询。
- 图表只按需加载。
- Agent 日志分页，不一次渲染全部。
- SQLite 定期压缩和归档。

## 18.2 稳定性目标

- Agent 崩溃不导致桌面主程序退出。
- PetWindow 崩溃可单独重建。
- 虚拟桌面失败自动回退。
- 锁机必须保留安全退出路径。
- 数据写入使用事务。
- Markdown 写入使用临时文件 + 原子替换。
- 每次升级自动备份数据库。

## 18.3 可测试性

必须建立：

- Agent Adapter Mock。
- 虚拟时钟。
- Scheduler 单元测试。
- 时间跨午夜测试。
- 睡眠/休眠恢复测试。
- 多显示器测试。
- 虚拟桌面失败测试。
- Pet Manifest 校验测试。
- 权限审批测试。
- Diff 回滚测试。

---

# 20. 开发里程碑

## M0：技术验证

目标：

- Tauri + Vue 创建三个窗口。
- PetWindow 透明、置顶、可拖动。
- PanelWindow 在 Chat/Statistics 间切换。
- DesktopWindow 全屏。
- Rust Event Bus 可同步三个窗口。
- SQLite 可写入。
- Windows 前台进程可被检测。

验收：

- 连续运行两小时无明显泄漏或失控。
- 多显示器下窗口位置正确。
- 面板打开时气泡互斥有效。

## M1：桌宠与桌面壳层

目标：

- 导入 Pet Pack。
- Agent 状态模拟器。
- 桌面快捷方式。
- 图标拖拽和布局保存。
- 托盘和全局快捷键。

验收：

- 可以完全不接 Agent 使用桌宠与启动器。
- 宠物状态切换流畅。
- 重启后布局恢复。

## M2：时间追踪与统计

目标：

- 前台应用记录。
- 空闲检测。
- 专注计时。
- 月度热力图。
- 单日时间分布。
- 应用分类。
- System Media Adapter。
- MusicWindow 基础控制。
- 音乐播放事件记录。
- 音乐类型与专注关联统计基础版。

验收：

- 跨午夜统计正确。
- 休眠时间不计入。
- 用户可以删除某日记录。
- 统计与原始事件可对账。

## M3：OpenCode Agent

目标：

- OpenCode 检测。
- 会话创建和恢复。
- 流式输出。
- 工具事件。
- Agent 状态映射到桌宠。
- 对话打开时禁止重复气泡。

验收：

- Agent 失败不影响 Tracker。
- 用户可以随时停止。
- 文件修改进入 Diff。

## M4：Markdown 日记与任务

目标：

- 选择工作区。
- 文件树。
- Markdown 编辑。
- 任务解析。
- 日记模板。
- Agent 受控读写。
- 修改审批。

验收：

- Agent 无法访问授权目录外文件。
- 修改可撤销。
- 日记追加不会覆盖原内容。

## M5：监督与限制

目标：

- 定时监督规则。
- 进度检测。
- 自定义声音。
- 偏离提醒。
- 软锁机。
- 多显示器遮罩。

验收：

- 不会在全屏会议/视频时频繁打扰。
- 每条规则可查看触发原因。
- 紧急退出可靠。

## M6：实验性虚拟桌面

目标：

- 创建 Focus Desktop。
- 切换桌面。
- 移动启动的窗口。
- Windows 版本适配。
- 失败回退。

验收：

- Windows 更新后失败不会阻止主程序启动。
- 单实例应用行为有明确处理。
- 不丢失用户窗口。

## M7：多 Agent 与插件生态

目标：

- Codex Adapter。
- Claude Code Adapter。
- Custom CLI。
- Agent 插件开发文档。
- Pet Pack SDK。
- 统计插件接口。

---

# 21. V1 建议范围

V1 不应一次完成全部设想。

建议 V1 只包含：

1. Desktop Overlay。
2. 可配置应用图标。
3. PetWindow。
4. PanelWindow Chat/Statistics。
5. 软件使用计时。
6. 专注计时。
7. 月度热力图。
8. 单日时间分布。
9. MusicWindow 与 System Media Adapter。
10. 音乐播放时长、曲目和类型统计。
11. OpenCode Adapter。
12. 生产力 Agent Profile 与路径提示词配置。
13. Markdown 工作区只读 + 审批式写入。
14. 基础 Scheduler。
15. 声音与气泡提醒。

暂缓：

- 真正系统级锁机。
- 替换 Explorer Shell。
- Live2D。
- 多 Agent 同时运行。
- 自动开放 Shell。
- 浏览器详细网页追踪。
- 跨平台。
- 公共宠物商店。
- 完整本地音乐库播放器。
- 在线音乐服务账号深度集成。
- 云同步。

---

# 22. 关键产品状态机

```text
AppState
├─ Normal
├─ FocusPreparing
├─ Focusing
│  ├─ Productive
│  ├─ Idle
│  ├─ Distracted
│  └─ WaitingAgent
├─ Break
├─ Locked
└─ Recovering
```

```text
PanelState
├─ Closed
├─ Chat
├─ Statistics
├─ Task
├─ Diff
└─ Permission
```

```text
PetSpeechPolicy
├─ Enabled
├─ RateLimited
├─ CriticalOnly
└─ Muted
```

建议映射：

| 状态 | 宠物 | 气泡 | 面板 |
|---|---|---|---|
| Panel Closed | 正常 | 允许 | 关闭 |
| Chat Open | 正常动画 | Critical Only | Chat |
| Statistics Open | 正常 | Rate Limited | Statistics |
| Locked | 锁定/休息 | 只显示锁机信息 | 不可自由打开 |
| DND | 正常 | Muted | 可打开 |

---

# 23. 尚未确定的问题

以下内容必须留给后续与 Codex 讨论和原型验证：

1. DesktopWindow 在 Windows 中应设为普通全屏、桌面层窗口还是 WorkerW 子窗口。
2. 是否从一开始支持多显示器独立布局。
3. VirtualDesktop 采用 C# helper 还是 Rust 直接调用。
4. OpenCode 的最佳接入方式是服务、SDK 还是 CLI JSON 流。
5. Markdown 编辑器使用现成 Vue 组件还是自研轻量编辑器。
6. Diff 审批由 Agent 自己生成还是由宿主基于文件快照生成。
7. 统计数据是否保存窗口标题。
8. 浏览器统计是否需要扩展。
9. 严格锁机是否值得安装 Windows Service。
10. 宠物动画包是否兼容 BongoCat 格式，还是定义新格式并写转换器。
11. Agent 会话历史放 SQLite、JSONL 还是保留外部 Agent 原格式。
12. 是否允许多个 Agent 同时存在多个宠物。
13. PanelWindow 是否固定右侧吸附，还是支持任意浮动。
14. 第一版是否只做 Windows。
15. 项目最终名称、图标和视觉语言。
16. System Media Adapter 在目标播放器上的元数据完整度。
17. 是否需要 V2 自带本地音乐库播放器。
18. 音乐类型由用户标签、元数据还是 Agent 建议主导。
19. Agent 是否可以主动生成跨音乐与专注数据的周期性报告。
20. MusicWindow 与桌宠在多显示器上的默认归属屏幕。

---

# 24. 给本地 Codex 的第一轮任务

建议先让 Codex执行“调查与架构确认”，不要立刻写完整产品。

## 任务 A：许可证审计

检查所有候选仓库：

- 当前 LICENSE。
- 代码文件中的额外版权头。
- 资源文件是否另有许可证。
- 是否包含第三方非 MIT 代码。
- 是否允许复制宠物素材。
- Tauri 依赖树中的许可证。
- 生成 `docs/licenses/audit-v0.md`。

## 任务 B：技术 Spike

建立最小原型：

```text
Tauri 2 + Vue 3
├─ DesktopWindow
├─ PetWindow
└─ PanelWindow
```

实现：

- 三窗口创建。
- 透明宠物窗口。
- 总在最前。
- 可拖动。
- Chat/Statistics 路由切换。
- Rust Event Bus。
- 面板打开时气泡关闭。
- 假 Agent 每两秒切换状态。
- 假统计数据绘制热力图和 24 小时图。

## 任务 C：Windows 可行性测试

测试：

- 多显示器。
- DPI 缩放。
- Alt+Tab。
- Win+D。
- 全屏应用。
- 窗口置顶。
- DesktopWindow 最底层。
- WorkerW 可行性。
- 虚拟桌面切换。
- 休眠恢复。

## 任务 D：Agent 协议

先定义 `AgentEvent` JSON Schema 和 Mock Adapter。

在 Schema 稳定前，不接真实 Agent。

---

# 25. 推荐的 Codex 开工提示词

```text
你正在为一个 MIT 许可的 Windows 本地生产力软件进行架构调查和技术原型开发。

请先完整阅读仓库中的《本地专注桌面与 Agent 桌宠系统：完整设计方案》。

当前阶段禁止直接实现完整产品。你的任务是：

1. 检查设计中所有技术假设，尤其是 Tauri 多窗口、透明置顶窗口、Windows Desktop Overlay、虚拟桌面兼容、前台应用追踪和 Agent 子进程管理。
2. 对所有计划引用或 fork 的仓库执行许可证审计，确认代码和素材是否都可在 MIT 项目中合法复用。
3. 建立最小 Tauri 2 + Vue 3 + TypeScript + Rust 原型，包含 DesktopWindow、PetWindow 和 PanelWindow。
4. 使用 Rust 事件总线同步三窗口。
5. 用 MockAgentAdapter 模拟 thinking、editing、waiting_permission、success 和 error 状态。
6. 实现规则：Chat 面板打开时宠物保留动画但不显示普通气泡；面板关闭后恢复气泡。
7. 新增 MusicWindow，用假数据展示封面、曲名、进度、上一首、播放暂停和下一首。
8. 在 Statistics 页面使用假数据展示月度专注热力图、单日 24 小时时间分布和音乐类型分布。
9. 建立 ProductivityAgentProfile，支持配置日记路径、任务路径和预设提示词。
10. 不接入真实 Agent，不做锁机，不替换 explorer.exe，不使用 Windows 私有虚拟桌面 API，直到技术调查完成。
9. 每个重要架构决策写入 docs/decisions/ADR-xxxx.md。
10. 任何不确定或可能受 Windows 版本影响的实现都必须写明风险、退路和验证方法，不得假定其稳定。

最终交付：
- 可运行的原型。
- 架构调查报告。
- 许可证审计。
- 风险清单。
- 下一阶段拆分建议。
```

---

# 26. 当前结论

项目具备现实可行性。

最稳妥的实现路线不是重写 Windows Shell，也不是把所有功能塞进单一 Agent 客户端，而是：

```text
Tauri + Vue 本地多窗口外壳
+ Rust 系统能力核心
+ 稳定 Desktop Overlay
+ 可选 Windows 虚拟桌面后端
+ OpenCode 首发 Agent Adapter
+ Markdown 受控工作区
+ 本地活动与音乐播放计时统计
+ Agent 状态驱动的桌宠
+ Chat / Statistics 复用浮动面板
```

V1 的关键不在于功能数量，而在于先证明以下闭环：

```text
用户进入专注桌面
→ 启动任务和计时
→ Agent 在授权目录内工作
→ 宠物显示 Agent 状态
→ 用户需要时打开正式对话
→ 系统记录人工与 Agent 活动
→ 音乐浮窗控制当前播放并记录音乐数据
→ 统计面板呈现时间、应用与音乐分布
→ 生产力 Agent 维护日记、任务并读取全部授权数据
→ 偏离时本地规则发出温和提醒
```

只要这个闭环成立，锁机、多 Agent、真实虚拟桌面和插件生态都可以在后续逐步增加。

---

# 27. 已核验参考仓库

- Tauri: https://github.com/tauri-apps/tauri
- BongoCat: https://github.com/ayangweb/BongoCat
- OpenPets: https://github.com/alvinunreal/openpets
- VirtualDesktop: https://github.com/MScholtes/VirtualDesktop
- AppGroup: https://github.com/iandiv/AppGroup
- OpenCode: https://github.com/anomalyco/opencode
- Coworker: https://github.com/accomplish-ai/coworker
- Pomotroid: https://github.com/Splode/pomotroid

> 在真正复制任何文件前，仍需由 Codex 对具体 commit、目录和素材执行逐项许可证审计。


---

# 27. v0.2 变更记录

本版本相较 v0.1 新增：

1. 独立 MusicWindow。
2. System Media Adapter 与 Local Library Adapter 规划。
3. 音乐播放事件、曲目和标签数据模型。
4. 音乐类型、播放时段与专注数据的关联统计。
5. Agent 对全部授权专注、应用和音乐数据的查询能力。
6. 将 Agent 产品角色明确为“个人生产力维护 Agent”。
7. ProductivityAgentProfile。
8. 通过设置界面配置日记路径、任务路径和预设提示词。
9. 明确“提示词负责告诉 Agent 如何理解文件，宿主权限系统负责限制它可以访问什么”。
10. 更新 V1 范围、里程碑、数据库、目录结构和 Codex 技术原型任务。
