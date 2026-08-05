# ADR-0006：Agent CLI 本地控制面（focus-cli）

- 状态：已接受（2026-08-05，v1.5）
- 关联：设计稿 §28（Agent 本地控制面）；§9 AgentHost、§10 受控工具、§17 权限模型

## 决策

1. **载体**：新增同 crate 第二二进制 `focus-cli`，通过 **localhost TCP（127.0.0.1 + 临时端口 + 每运行 token）** 与主程序通信；`{port, token}` 写入 `app_data_dir/cli.json`。消息为 JSON，4 字节小端长度前缀成帧。
   - 原计划 named pipe：当前工具链（Rust 1.97）`std::os::windows::named_pipe` 不可用；windows crate 管道需新增 `Win32_System_Pipes`/`Win32_Storage_FileSystem` feature 与大量 unsafe。TCP + token 纯 std、零新依赖、仅本机同用户可连，作为等价替代。
2. **命令集**（契约写入设计稿 §28）：`timer start|pause|skip|status`、`stats today|week|sessions`、`desktop layout`、`apps now|visible`、`ping`。
3. **timer 路由**：Rust 服务线程经 `cli:timer {id, action}` 事件驱动桌面 webview 的番茄钟状态机；webview 执行后回 `cli:timer-done {id, ...状态}`，服务线程按 id 匹配 oneshot 等待（≤3s）并回给 CLI。
4. **安全**：仅 127.0.0.1 + 用户级 token（`cli.json` 存于 `app_data_dir`）；M3 接 Agent 时由宿主加"授权动作白名单 + 审计"；token 轮换/加密记为可选强化。

## 后果

- 主进程新增 `cli.rs`（TCP 服务）、`src/bin/focus-cli.rs`（客户端）；`launch-focus.cmd` 构建产物含 `focus-cli.exe`。
- `AppState` 新增 `cli_pending`（id → oneshot）、`cli_next_id`、`cli_token`。
- Agent（M3）通过调用 `focus-cli` 即可查询状态/驱动计时，无需触碰 DB 或 UI。
- 风险：TCP 服务若被本机恶意进程读到 token 可被控制——仅同用户可读 `app_data_dir`，且当前为本地开发阶段；发布前应加授权白名单与审计。
