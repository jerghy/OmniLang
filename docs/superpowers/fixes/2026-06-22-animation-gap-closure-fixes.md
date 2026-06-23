# Animation Gap Closure 测试与修复

## 修复 1: 重复的 `__omni_motion_` const 声明

**问题**: 两个不同组件定义了同名 motion 时，JS 生成重复 `const __omni_motion_{name}` 导致 `Identifier has already been declared` 错误。

**根因**: Motion 变量名未加组件前缀作用域。

**修复**: 在 `emit_motion_js`, `emit_timeline_js` 及所有 12 处 trigger reference 中将 `__omni_motion_{name}` → `__omni_motion_{comp}_{name}`。

**文件**: `omni-compiler/src/codegen/omniview.rs`

## 修复 2: @animate 触发器未自动播放

**问题**: `@animate={motion}` 创建了 AnimationPlayer 但未调用 `.play()`，导致循环动画不播放。

**根因**: `emit_motion_triggers_for_element` 的 match 中 `TriggerKind::Animate` 落入了 `_ =>` 默认分支（只创建不播放）。

**修复**: 新增 `TriggerKind::Animate =>` 分支，创建 player 后调用 `player.play()`。

**文件**: `omni-compiler/src/codegen/omniview.rs`

## 测试验证

| 测试项 | 结果 |
|--------|------|
| @hover: scale 1.05 应用 | ✅ |
| @press: scale 0.95 应用 | ✅ |
| @animate: 持续循环播放 | ✅ |
| @focus: 创建 player | ✅ |
| @in-view: 创建 player | ✅ |
| @scroll: 创建 player | ✅ |
| @drag: 创建 player | ✅ |
| @enter: 创建并播放 | ✅ |
| @if/@else 条件渲染 | ✅ |
| Computed 函数式写法 | ✅ |
| 测试套件 (463个) | ✅ 全部通过 |
