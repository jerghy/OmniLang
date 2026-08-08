# 03-adr072-security-guard: ADR-072 信任边界守卫

Status: ready-for-agent

## 背景

阶段 0 第 3 项（ROADMAP.md 0.3）。**CRITICAL**（08-08 全工作区评审 T2）：调试窗编译产物与 `__TAURI_INTERNALS__` **同 realm**，且 capabilities **不门控自定义命令** → 产物可调用全部 17 个自定义 Tauri 命令 = **全盘读写**。

修复方向（评审建议）：`AppManifest::commands` 门控（~5 行）或回滚 ADR-072。

## 任务

- [ ] capabilities 门控自定义命令（或按评审建议的 ~5 行守卫）
- [ ] 安全评审复核确认
- [ ] 若改动 ADR-072 决策本身 → 更新 ADR 记录

## 验收

- 调试窗产物无法调用自定义命令（验证：从 eval 产物侧发起 invoke 被拒）
- 安全 CRITICAL 关闭

## 关联

- ROADMAP.md 0.3
- `.architecture/reviews/full-workspace-review-2026-08-08.md`（T2 CRITICAL）
- `.architecture/decisions/adrs/ADR-072-debug-window-direct-load.md`

## Comments
