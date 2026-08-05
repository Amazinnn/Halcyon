# Third-Party Notices

本项目（Focus Desktop）采用 MIT License（见 `LICENSE`）。本文件记录本项目直接依赖与参考来源的许可证信息，随许可证审计（`docs/licenses/audit-v0.md`，2026-08-05）更新。

## 直接依赖

### npm 依赖（apps/desktop 与 packages/event-schema）

| 组件 | 版本 | 许可证 | 用途 |
|---|---|---|---|
| vue | 3.5.41 | MIT | 前端框架 |
| chart.js | 4.5.1 | MIT | 统计图表 |
| @tauri-apps/api | 2.11.1 | MIT 或 Apache-2.0 | Tauri 前端 API |
| @tauri-apps/cli | 2.11.4 | MIT 或 Apache-2.0 | Tauri 构建 CLI（开发依赖） |
| @tauri-apps/plugin-opener | 2.5.4 | MIT 或 Apache-2.0 | 外部打开（脚手架自带） |
| vite | 6.4.3 | MIT | 构建（开发依赖） |
| typescript | 5.6.3 | Apache-2.0 | 类型检查（开发依赖） |
| vue-tsc | 2.2.12 | MIT | 类型检查（开发依赖） |
| @vitejs/plugin-vue | 5.2.4 | MIT | Vite 插件（开发依赖） |
| ajv | 8.20.0 | MIT | event-schema 校验 |
| ajv-formats | 3.0.1 | MIT | event-schema 校验（date-time 格式） |
| @tauri-apps/plugin-dialog | 2.7.2 | MIT 或 Apache-2.0 | 壁纸选择对话框（v1.2） |

### Rust 依赖（apps/desktop/src-tauri，以 Cargo.toml 为准）

| 组件 | 许可证 |
|---|---|
| tauri / tauri-build | MIT 或 Apache-2.0 |
| tauri-plugin-opener | MIT 或 Apache-2.0 |
| serde / serde_json | MIT 或 Apache-2.0 |
| tokio | MIT |
| rusqlite（bundled） | MIT |
| windows | MIT 或 Apache-2.0 |
| window-vibrancy | 0.8.0 | MIT 或 Apache-2.0 | 浮窗 Acrylic 真毛玻璃（v1.2） |
| tauri-plugin-dialog | 2 | MIT 或 Apache-2.0 | 壁纸选择对话框（v1.2） |

## 参考项目（首轮未复制代码，仅作设计参考）

详见 `docs/licenses/audit-v0.md`。8 个参考仓库均为 MIT（tauri 为 MIT/Apache 双许可）：

| 仓库 | 许可证 | 首轮复用决策 |
|---|---|---|
| tauri-apps/tauri | MIT 与 Apache-2.0 双许可 | 依赖消费 |
| ayangweb/BongoCat | MIT | 仅参考（宠物格式为 Live2D，不复制） |
| alvinunreal/openpets | MIT | 仅参考（概念） |
| MScholtes/VirtualDesktop | MIT | 暂不采用（未公开 COM 接口，推迟 M6） |
| iandiv/AppGroup | MIT | 仅参考（概念） |
| anomalyco/opencode（= sst/opencode） | MIT | 仅定义适配接口，不 fork |
| accomplish-ai/coworker | MIT | 仅参考（概念） |
| Splode/pomotroid | MIT | 仅参考（概念） |

> 注意：如后续从任何参考仓库实际复制/改编代码或素材，必须按审计约定逐文件记录来源（commit SHA）并保留原许可证与版权声明。