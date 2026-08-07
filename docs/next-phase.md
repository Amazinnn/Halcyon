# 下一阶段拆分建议（M0 收尾 → M1）

> 对应设计稿 §25 最终交付项「下一阶段拆分建议」。2026-08-05。

## 首轮已完成（任务 A–D）

- 许可证审计（`docs/licenses/audit-v0.md`）、AgentEvent 协议 v1（`packages/event-schema`）、技术 Spike（`apps/desktop`）、Windows 可行性（`docs/architecture/windows-feasibility-v0.md`）、ADR-0001~0004、风险清单、本建议。

## M0 收尾（人工验收清单）

1. 透明窗口（Pet/Music）目视确认；必要时调整圆角/不透明度。
2. Alt+Tab、Win+D、全屏共存、睡眠/休眠恢复逐项手动验证并记录。
3. 200% DPI 下核对 `inner_size` 逻辑/物理映射，按需改用 `PhysicalSize`。
4. 多显示器：外接或虚拟显示器驱动验证窗口位置与 Desktop 覆盖。
5. 连续运行 2 小时观察内存与事件丢包（M0 验收）。

## M1：桌宠与桌面壳层（2026-08-07 现状）

已完成（v1.5–v1.7.2）：
- 桌面快捷方式与图标自由布局（v1.5）：应用/文件/文件夹/URL/内部页，真实图标提取，12×8 网格吸附 + DB 持久化。
- Pet Pack 导入与精灵图播放（v1.7，ADR-0009）：吸收 hatch-pet 产物（pet.json + spritesheet.webp，8×9 / 192×208），文件夹导入 + 校验（尺寸 + 透明背景）+ 持久化 + 四尺寸。
- 桌宠 UX（v1.7.1，ADR-0010）：纯精灵图布局 + hover 对话按钮、缩放网格预览 + 冲突回弹、修复拖拽移动、淡化开关、外置 pet-builder skill。
- 交互修复（v1.7.2）：文件夹快捷方式经 explorer.exe 直开、视图按钮点击式展开、缩放回归修复（overlay WS_EX_NOACTIVATE）。

剩余（下一迭代候选）：
- **托盘与全局快捷键**：`TrayController` 基础托盘菜单 + 2~3 个全局快捷键（显示/隐藏面板、开始/停止专注）。
- **Agent 状态模拟器完善**：Mock 序列参数化（速度/剧本/气泡），保留 Schema 校验。
- 验收：重启后布局恢复；宠物状态切换流畅。

## 后续里程碑（保持 M2→M7 顺序）

- M2：时间统计 + System Media Adapter（音乐）；M3：Codex CLI 嵌入（Claudian 式；2026-08-06 已实现垂直切片：对话 + focus-cli 白名单审计 + skills 透传，ADR-0007；plan mode/Diff/终端面板/Claude Code 为后续）；M4：Markdown 日记/任务与审批；M5：监督与软限制；M6：实验性虚拟桌面（C# helper）；M7：多 Agent 生态。
- M1 与 M2 模块边界清晰，可部分并行；M3 依赖 M1 的对话面板可用性。