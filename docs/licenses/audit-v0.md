# 许可证审计报告 v0（任务 A）

> 日期：2026-08-05
> 范围：设计稿 §27「已核验参考仓库」8 个仓库的定向许可证审计。
> 方法：浅克隆到系统临时目录（`%TEMP%\focus-audit-*`），固定审计当日 HEAD commit；根许可证以仓库内 LICENSE/COPYING 文件为准，GitHub API 元数据交叉核对。首轮 Spike 不复制任何第三方代码或素材，因此只做定向核对，深审留到真正复用时刻。

## 审计矩阵

| 仓库 | 固定 HEAD (审计当日) | SPDX 许可证 | 版权行 | 目标目录/用途 | 首轮复用决策 |
|---|---|---|---|---|---|
| tauri-apps/tauri | `29c87c3d3f5bbcf5a7ae9de01af7e6bb738c1d01` | MIT 与 Apache-2.0 双许可（本项目选择 MIT） | 见 LICENSE-MIT | 作为依赖与 API 消费，不复制源码 | 可复用（依赖，列入 THIRD_PARTY_NOTICES） |
| ayangweb/BongoCat | `44f44bcf2b17b8e16463ad479a477a949d01cc9a` | MIT | Copyright (c) 2025 ayangweb | 透明宠物窗口工程结构（仅参考） | 可参考；**其宠物格式为 Live2D（.model3.json/.moc3/.exp3.json），按设计稿 §16.2 只观察不复制**；V1 用 Sprite Sheet 自定格式，无需兼容 |
| alvinunreal/openpets | `f0f488685cd750751c4bbc42de6ccda1172c8187` | MIT | Copyright (c) 2026 OpenPets | Agent→宠物事件、气泡过滤、权限思路（Electron 外壳不采用） | 可参考，仅概念迁移；逐文件复制时记录来源 |
| MScholtes/VirtualDesktop | `a725cbd3cdb9e977678eeaf034a7cc96d2e74bc6` | MIT | Copyright (c) 2017 Markus Scholtes | 虚拟桌面控制（C#） | 首轮不复制、不实现；见下方定向检查结论 |
| iandiv/AppGroup | `0797abf5cac5dcb908a6779147d26624520f3dd2` | MIT | Copyright (c) 2025 IanDiv | 应用启动器、图标与启动参数思路（仅参考） | 可参考，Vue UI 自研 |
| anomalyco/opencode（等价 sst/opencode，GitHub 重定向） | `4a57013cf8cb163f58638273fd9da8538cd33cb7` | MIT | Copyright (c) 2025 opencode | 首发 Agent 适配对象（服务/协议接入，不 fork） | 可接入；首轮只定义 Adapter 接口与事件 Schema，不 vendor 其代码 |
| accomplish-ai/coworker | `2cf74d08f22078b8b1fd3f97bff3ec4612262613` | MIT | Copyright (c) 2026 Accomplish Inc | AgentHost、权限、会话设计（Electron/React UI 不采用） | 可参考，仅概念 |
| Splode/pomotroid | `f9f0b266f7a04b895599ca660544219c0d0df054` | MIT | Copyright (c) 2018 Christopher Murphy | 计时器状态机（仅参考） | 可参考，统计数据模型自研 |

## 定向检查结论

### BongoCat 宠物格式 = Live2D（不兼容 V1 Sprite Sheet）
- `src-tauri/assets/models/*` 使用 `cat.model3.json`、`.moc3`、`.exp3.json`、`.motion3.json` 等 Live2D Cubism 文件。
- 结论：BongoCat 可作透明窗口、置顶、拖动等工程参考；其宠物模型格式依赖 Live2D SDK，按设计稿 §16.2「Live2D Cubism SDK 只观察、不复制」，首轮 pet-pack 采用自定 Sprite Sheet 格式（ADR-0004），不做 BongoCat 格式兼容，也不引入任何 BongoCat 素材。

### VirtualDesktop 依赖未公开 COM 接口（高风险，推迟 M6）
- `VirtualDesktop.cs` 通过 `[ComImport]` + 硬编码 CLSID 调用内部接口：`IVirtualDesktopManagerInternal`（CLSID `C5E0CDCA-7B6E-41B2-9FC4-D93975CC467B`）、`IVirtualDesktop`、`IApplicationView`、`IVirtualDesktopPinnedApps` 等。
- README 明示：Windows 11 23H2 起微软再次改动 COM GUID，故仓库按系统版本提供 5 份源码（Win10 / Win11 / Win11-24H2 / Server2016 / Server2022）。
- 结论：与设计稿判断一致——虚拟桌面属不稳定能力。首轮不实现、不用私有 API；M6 若推进，倾向复用该 C# helper（MIT）作为独立兼容层，并保留 Overlay 回退。

### opencode 体量过大，仅核验根许可证
- 浅克隆超时，改用 GitHub API 与 raw LICENSE 核验：MIT，© 2025 opencode。
- 首轮只定义 `AgentAdapter` 接口与 `AgentEvent` Schema，不复制 opencode 代码，故根许可证核验已足够；协议接入细节留待 M3。

## 不可用/暂缓清单（首轮不采用）

- AGPL 桌面环境与 Shell Replacement（§16.2）。
- Live2D Cubism SDK 及其素材。
- 来源不明、许可证不清晰的宠物素材与公开分发包。
- 未公开稳定协议的私有 Agent UI 实现。
- 首轮未导入任何第三方宠物素材（内置占位动画为程序生成），因此无素材许可风险。

## 结论

8 个参考仓库均为 MIT（tauri 为 MIT/Apache 双许可，可选 MIT），与「MIT 优先」原则相容。首轮 Spike 零代码/素材复制，`THIRD_PARTY_NOTICES.md` 仅需登记实际引入的 npm/cargo 依赖与上述参考关系。真正的逐文件深审应在计划复制某目录/素材时执行。