# ADR-0004：pet-pack 格式 —— 自定 Sprite Sheet Manifest（v1）

- 状态：已接受（2026-08-05，任务 B）
- 关联：设计稿 v0.2 §5.4、§16.2；`apps/desktop/src/lib/petPack.ts`

## 决策

- V1 宠物包使用**自定 Sprite Sheet + Manifest** 格式（`schemaVersion: 1`），字段沿用设计稿 §5.4：`id/name/author/license/bubbleAnchor/animations{frames,fps,loop}`。
- 首轮不兼容 BongoCat 格式、不引入 Live2D、不导入任何第三方素材。
- 内置 `focus.builtin.placeholder` 占位宠物（程序生成动画，MIT）。

## 依据

- 许可证审计（`docs/licenses/audit-v0.md`）：BongoCat 宠物格式为 Live2D（`.model3.json/.moc3/.exp3.json`），按设计稿 §16.2 只观察不复制。
- Live2D Cubism SDK 不在 MIT 依赖范围内。

## 后果

- 导入第三方宠物包时执行 `validatePetManifest`（校验 schemaVersion、动画字段、作者/许可证），拒绝许可证不明素材；用户私人素材仅本地使用、不打包进发布版。