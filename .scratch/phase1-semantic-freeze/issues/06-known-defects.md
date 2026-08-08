# 06-known-defects: 已知缺陷收官（检查点 1F）

Status: ready-for-agent

## 背景

冻结前需处置的已知缺陷（详见 08-08 全工作区评审）：

1. **IR type_ann 恒 None**——React/Dart typed 分派死代码（记忆：ir-type-ann-attachment-defect）
2. **跨文件验证空洞**——`validate_required_props` / `validate_effect_deps` 仅单文件（`ir/lower/mod.rs:345`），导入的组件/effects 静默漏诊
3. **cache 性能**——`MAX_CACHE_ENTRIES=50` < 102 文件项目 → 击键 15.6× 回归（`crates/omnilang-core/src/analyse.rs:238`）；LRU 逐出毁反向依赖失效边（stale-IR 合并正确性风险）
4. **E0006 suggestion 缺陷**（含 4 个自身硬错误名）——闭包计划明确归阶段 4 → 按计划 defer 或修复

## 任务

- [ ] IR type_ann 修复
- [ ] 跨文件验证补齐（组件实例不再漏诊）
- [ ] cache 逐出策略修复（容量 ≥ 项目规模 or 保留失效边）
- [ ] E0006 处置（修复或按闭包计划 defer 记录）

## 验收

- 基线回归绿；击键延迟恢复（15.6× 消失）
- 跨文件漏诊消除（新增测试）

## 关联

- ROADMAP.md 1F
- `.architecture/reviews/full-workspace-review-2026-08-08.md`（T3 性能 / T6 跨文件验证）
- `.architecture/deferrals.md`

## Comments
