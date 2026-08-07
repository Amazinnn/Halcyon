# ADR-0010：桌宠 UX 修复与外置 Pet Builder skill（v1.7.1）

- 状态：已接受（2026-08-07）
- 关联：ADR-0009（hatch-pet 契约）、ADR-0004（pet-pack 背景）；需求日志 #14

## 背景

用户报告四个桌宠问题：① 1×1 时宠物图标太小，图样内不应有按键，对话按钮改悬停触发、导入迁到设置；② 缩放缺少参考网格；③ 四种尺寸均无法拖拽移动；④ 精灵图背景色与桌面环境不契合。另要求外置一个优化版 Hatch Pet skill 便于测试。

## 根因（已取证）

1. **无法移动**：`PetView.vue` 的 `.pet-stage` 与 `.pet-bar` 均标 `data-no-drag`，被 `useGridDrag` 跳过，点击宠物本体无法触发 `drag_start`。
2. **背景不透明**：去底时硬编码 `--key-color #FF00FF` 未匹配 SenseNova 实际输出的暗品红（约 195,16,120），帧仍 100% 不透明，atlas 带背景色块。
3. **缩放无网格**：`resize_window` 为即时命令，无 `grid:preview` 预览通道。
4. **1×1 太小**：底部 pet-bar（名字/对话/宠物按钮）挤压精灵图。

## 决策

- **布局**：宠物图样内只放精灵图；对话按钮在鼠标悬停于整个浮窗时出现，置于精灵图旁空白区（不覆盖图样），点击后隐藏；宠物名随 hover 气泡显示；导入/切换宠物包迁移到设置（SettingsPopover「宠物」区块）。
- **临时 UI 准则（软准则）**：临时出现的按钮/浮窗尽量不占用已呈现窗口；本实现中 hover 按钮只出现在桌宠浮窗自身空白区，不新开窗口、不遮挡其他浮窗。
- **缩放预览**：缩放手柄按下即显示 grid-overlay，亮度跟随目标尺寸矩形（floatRect=当前位置+目标尺寸），冲突标红；松手先隐藏预览再 `resize_window` 落库；目标与其他浮窗重叠时**拒绝并回弹**原尺寸（不写 settings、窗口不动）。
- **背景处理**：A) 开发期用 `remove_chroma_key.py --auto-key corners --tolerance 60 --spill-cleanup --edge-feather 2 --soft-matte` 重处理帧成真透明；B) 软件内新增 `pet_bg_fade`（默认开）淡化边缘残留底色，作为可选项兜底。
- **导入边界（C1）**：`pets.rs` 导入校验增加背景透明度检查（读取 PNG/WebP 头部尺寸 + 四角/四边采样 alpha），非透明背景**拒绝导入**并提示「背景必须透明」；Focus 保持「只消费合规素材」边界，不做自动去底。
- **外置 skill**：新建 `~/.codex/skills/pet-builder`（hatch-pet SenseNova 适配版），完整流水线=绕代理 → 生成 9 行 × 2 帧（running-left 用官方镜像脚本派生）→ auto-key 去底 → 合成 1536×1872 → validate → 写 pet.json/spritesheet.webp → 可选导入 Focus；保留官方 8×9 / 192×208 / pet.json 四字段契约。
- **不引入**：软件内不内置图像生成/去底；素材仍由外部通道生成。

## 后果

- 桌宠窗口变纯净（无常驻按键），1×1 下精灵图最大化。
- 拖拽移动恢复；缩放有网格参考与冲突保护。
- 导入坏背景包会被拦截并得到明确提示；淡化开关可进一步改善边缘观感。
- `pet-builder` skill 沉淀 SenseNova 通道参数与常见失败（代理未监听、尺寸白名单、帧数补全），换角色/测试可复用。