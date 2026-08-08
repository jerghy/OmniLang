# 02-red-gate-closure: 红门清零

Status: ready-for-agent

## 背景

阶段 0 第 2 项（ROADMAP.md 0.2）。ADR-075 事件意图化迁移后两个门禁一直红：

- **verify-docs** 3 个失败块：`docs/omniview/00-cheatsheet.md:175`、`docs/omniview/migration-v1-to-v2.md:185`、BNF spec `:829`（cheatsheet 仍在教已死的 `@onClick` 语法）
- **verify-vocabulary** 1 个问题：regex 在 `legacy_spelling` 分支误报

另：poiesis 事件面板仍显示旧事件名（08-08 评审 8/12 专家指出）。

## 任务

- [ ] 执行闭包计划：`docs/superpowers/plans/2026-08-08-semantic-event-intent-phase1-closure.md`（零 Rust 改动：python 脚本 / docs / 编辑器 TS）
- [ ] 同步 poiesis 事件面板旧事件名

## 验收

- docs-gate + verify-vocabulary 全绿
- cheatsheet 不再教已死 `@onClick` 语法
- poiesis 事件面板与 ADR-075 事件名一致

## 关联

- ROADMAP.md 0.2
- `docs/superpowers/plans/2026-08-08-semantic-event-intent-phase1-closure.md`

## Comments
