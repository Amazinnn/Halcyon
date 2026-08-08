# ADR-0016：内部页打开自动避让最近空位（v1.10.3，需求 #45）

- 状态：已接受（2026-08-08，v1.10.3 修复轮）
- 关联：需求 #45；ADR-0005（网格/窗口管理）；grid.rs

## 背景

用户发现：点击视图托盘打开「尚未出现的内部页」时，新窗口可能直接落在当前可见窗口之上（例：无统计但有工作流时点统计，统计叠在工作流上）。虽然暂未卡死，但用户担心再次触发 #35 类进程级卡死。

根因：`restore_window` 直接取 `settings.grid` 中保存的旧矩形并 `position_window`，**不检查 occupied**；若该矩形与当前可见窗口重叠（例如窗口曾被移走/折叠后旧坐标被其他窗口占用），新窗口即重叠打开。拖放路径 `place()` 有重叠校验，但 restore 路径没有。

## 决策

1. **restore 前避让**：`restore_window` 计算目标矩形后，若与 `occupied_rects(settings, Some(label))` 任一矩形重叠，则调用新增 `grid::find_free_slot` 找**离原位置最近的空位**，找到则更新 `settings.grid` 并持久化；找不到空位（实际不可能，网格 12×8 仅 5 个窗口）回退原矩形。
2. **find_free_slot 语义**：在合法范围内（含 TEXT_WINDOWS 最小宽度钳制）按候选顶角到 desired 顶角的距离升序，返回第一个不与 occupied 重叠的矩形；desired 本身空闲时优先返回 desired。
3. **拖放路径不变**：`place_window` 冲突回弹语义保持不变（拖到占用格仍回弹）。
4. **M4 show_window 节点**共用 restore_window，自动获得避让能力。

## 后果

- grid.rs 新增 `find_free_slot` + 单测；lib.rs `restore_window` 接入。
- 所有内部页打开（视图托盘 / show_window 节点 / 快捷键 internal）行为一致：绝不重叠。