# 05-b3-variant-class-noise: B3 variant class 误报噪音

Status: ready-for-human

## 背景

阶段 0 第 5 项（ROADMAP.md 0.5）。`tasks/omniview-syntax-audit-bugs.md` **B3 待批**：variant 参数值 class 误报警告洪——`'Button'`×118、`'sm'`×92、`'primary'`×45，渲染正确、纯噪音。

修复方向已写在审计文档：**class 验证豁免 expand_variant 产物**。

## 需要决策

- [ ] 批准修复方向（class 验证豁免 expand_variant 产物）

> 批准后本 issue 状态翻转为 `ready-for-agent` 并执行修复。

## 验收

- 警告洪消除，渲染结果不变
- B3 在 `tasks/omniview-syntax-audit-bugs.md` 关闭

## 关联

- ROADMAP.md 0.5
- `tasks/omniview-syntax-audit-bugs.md`（B3）

## Comments
