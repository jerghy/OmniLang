# 01-domain-a-closure: 域 A 收口（检查点 1A）

Status: ready-for-agent

## 背景

阶段 2 域 A（@form/@load）Task 1-4 已提交（parser `9dd6dc8e` / IR `8e5268c3` / semantic `0b2c39bd` / React codegen `400785ce`），**Task 5-8 未做**。

## 任务

- [ ] Task 5：**Flutter** @form/@load codegen（`grep FormIntentIR/LoadIntentIR` 仅命中 `omnilang/src/codegen/react/view.rs`）
- [ ] Task 6：**OmniView** codegen
- [ ] Task 7：语义全集**域 8/9 登记** + BNF/文档 + 注入语料（`semantic-universe.md` 现仅域 1-7，无表单/数据三态条目）
- [ ] Task 8：learn-omniview 演示迁移（当前**零 @form/@load 用例**）+ Playwright

## 验收

- 三目标 @form/@load 可用 + 门禁绿
- 演示项目含 @form/@load 用例

## 关联

- ROADMAP.md 1A
- `docs/superpowers/plans/2026-08-08-semantic-form-load-phase2.md`（计划内 defer 见 line ~480：V6 字段类型检查 warning、@load 非 Array 返回、多表单/跨组件表单、FormData 预填、Flutter 提交异步语义）

## Comments
