# README 与设置说明 Eval：面向学生与专注工作者

- 日期：2026-08-11
- 范围：需求 #100；README 用户指南、设置页静态说明与版本展示。不改运行时架构，不新增 ADR。
- 当前版本：v1.12.8

## 改动契约

1. README 面向普通 Windows 使用者：说明定位、启动、首次专注、核心入口、桌宠导入契约与动画质量边界，不再承载开发编年史。
2. 设置页的锁定模式、壁纸、外观、计时、任务、监督、宠物、Agent、音乐、运行记录各有一条常驻简洁说明。
3. 宠物仅手动导入外部文件夹：`pet.json` 与透明 `spritesheet.webp` 或 `.png`；图集为 1536×1872、8×9 格、每格 192×208。
4. 截图只可在用户明确回复“演示状态已准备好”后获取；仅限 Focus 窗口与干净演示数据，不读取、修改或展示个人数据。

## 自动化与文档检查

| 检查 | 状态 | 证据 |
| --- | --- | --- |
| 设置文案先失败 | Pass | 先在 `SettingsPopover.test.ts` 断言 v1.12.8、壁纸格式、监督边界、宠物格式、音乐格式和 CLI 登录归属；旧 `v0.1.0` 上预期失败。 |
| 设置聚焦测试 | Pass | `apps/desktop npm test -- SettingsPopover.test.ts`：4 passed、0 failed。 |
| 完整前端测试 | Pass | `apps/desktop npm test`：13 files、49 tests passed（2026-08-11）。 |
| 前端构建 | Pass | `apps/desktop npm run build`：`vue-tsc --noEmit` 与 Vite production build 成功；仅有既有 bundle 体积提示。 |
| Rust lib 测试 | Pass | `apps/desktop/src-tauri cargo test --lib`：175 passed、0 failed；仅有既有编译警告。 |
| event-schema 测试 | Pass | `packages/event-schema npm test`：11 valid、4 invalid fixtures checked，TypeScript 检查通过。 |
| release 重建 | Blocked | `launch-focus.cmd rebuild` 已执行；链接阶段无法删除 `target/release/desktop.exe`（Windows 拒绝访问，os error 5）。须先由用户关闭正在使用的 Focus 后重试。 |
| README 链接、图片路径与替代文本 | Pending | 等用户准备干净演示状态并补入四张真实 Focus 截图后检查。 |
| `git diff --check` | Pass | 2026-08-11 已通过；仅出现 Git 的 LF→CRLF 工作树提示，无空白错误。 |

## 人工验收

1. Pending：用户按 README 从源码目录启动，并完成一次开始专注、打开聊天和工作流。
2. Pending：用户确认设置页说明简洁、不喧宾夺主，且桌宠、音乐、监督和 Agent 的边界清楚。
3. Pending：用户准备演示状态后，截取主界面与专注、对话与桌宠、工作流、设置与已导入桌宠四张 Focus 专属截图。

## 交接

在截图门槛完成前，本轮不得把 README 的截图要求标记为通过；自动构建与重建同样不得以历史 v1.12.8 证据替代。
