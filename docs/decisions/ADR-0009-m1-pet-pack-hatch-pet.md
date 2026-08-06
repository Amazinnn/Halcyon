# ADR-0009：M1 桌宠 Pet Pack —— 吸收 hatch-pet 产物（v1.7）

- 状态：已接受（2026-08-06，M1 桌宠）
- 关联：ADR-0004（pet-pack 背景，许可证审计结论不变）、设计稿 v0.2 §5.4；需求日志 #13
- 取代：ADR-0004 的「自定义 Sprite Sheet + Manifest」部分（本 ADR 采纳官方 hatch-pet 契约为唯一输入格式）

## 背景

用户要求桌宠为真实动图（非几何占位）、支持用户自定义导入、可 1×1/1×2/2×1/2×2 拉伸且图案居中留边。OpenAI 官方 hatch-pet skill 已实现精灵图生成流程（文字/参考图 → 固定状态透明帧 → 裁剪校验 → 合成 atlas → 打包 `pet.json` + `spritesheet.webp`）。本 ADR 决定 Focus 直接消费该产物，不自造 manifest 变体。

## 决策

- **输入格式 = hatch-pet 产物契约**：
  - Atlas：PNG 或 WebP，固定 `1536x1872`，8 列 × 9 行，每格 `192x208`，透明背景，未用格全透明。
  - 包结构：`${CODEX_HOME:-$HOME/.codex}/pets/<pet-name>/pet.json` + `spritesheet.webp`。
  - `pet.json` 四字段：`id` / `displayName` / `description` / `spritesheetPath`。
  - 行状态/帧时长固定于官方契约 `animation-rows.md`（idle/running-right/running-left/waving/jumping/failed/waiting/running/review；每行固定列数与每帧 ms）。
- **状态映射（官方 9 行 → 应用六态）**：
  - idle→row0 idle；thinking→row7 running；editing→row8 review；waiting→row6 waiting；success→row4 jumping；error→row5 failed。
  - running-right/left（row1/2）、waving（row3）本轮不映射（浮窗无移动语义）。
  - 非循环动画（jumping/failed）播完回 idle。
- **导入流程**：选择文件夹 → 校验 `pet.json` 字段与 spritesheet 存在 → 复制整个目录到应用数据目录 `pet-packs/<id>/` → `settings.pet_pack_id` 持久化 → 立即激活。
- **校验边界**：Rust 只做字段/存在性校验（不引入 image 解码依赖）；atlas 尺寸 `1536x1872` 由前端 Image 加载后校验，不符拒绝。
- **不引入**：Live2D、zip 解包、脚本执行；本轮不做素材生成流程集成（生成由 hatch-pet 负责，Focus 只消费）。
- **回退**：未导入或校验失败时使用内置占位（现有 SVG，MIT，`focus.builtin.placeholder`）。

## 后果

- 播放器按固定几何切帧，无需 manifest 帧配置；事件链路（`pet:state_changed`）与前端 store 动画名保持不变，只改渲染层。
- 用户自备/生成素材须符合官方契约尺寸；不符包被拒绝并提示原因。
- ADR-0004 保留为背景（Live2D 排除与许可证审计结论不变），其自定义 manifest 字段不再作为输入格式。

## ?????2026-08-06?

- ?????`pets.rs` ??/??/?????`settings.pet_pack_id`?`resize_window`?1?1/1?2/2?1/2?2?????????? + ??/?? UI + ?????Rust 52 passed?vue-tsc ???release ?????
- ????????? MiniMax-M3 via Claude Code?hatch-pet skill??**BLOCKED**????Claude Code ??? `image_gen` ???`imagegen` CLI fallback ? `OPENAI_API_KEY`???????? deepseek ????? `POST /v1/images/generations` ?? 404?Codex ???? `OPENAI_API_KEY`???????????????? ADR??????????????
- ??????????? `OPENAI_API_KEY` ?? hatch-pet CLI/?????????????????? Codex ???????????????1536?1872 / 8?9 / 192?208???????
