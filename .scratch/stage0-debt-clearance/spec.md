# Stage 0 — 清债

路线图阶段 0（ROADMAP.md）：**先清再开工**——开工编译器大活前把红债清零。

## 范围

| # | Issue | 内容 |
|---|---|---|
| 01 | commit-and-push | 收尾提交 + push 82 commits |
| 02 | red-gate-closure | 红门清零（docs-gate + verify-vocabulary） |
| 03 | adr072-security-guard | ADR-072 信任边界守卫（CRITICAL） |
| 04 | verification-evidence | Groups Task 8 + Enrichment 真机验证证据补齐 |
| 05 | b3-variant-class-noise | B3 variant class 误报噪音（待批） |

## 关联

- ROADMAP.md 阶段 0
- `docs/superpowers/plans/2026-08-08-semantic-event-intent-phase1-closure.md`（红门清零计划）
- `.architecture/reviews/full-workspace-review-2026-08-08.md`（ADR-072 / cache / 死面 / 82 commits）
- `tasks/omniview-syntax-audit-bugs.md`（B3）
- `docs/superpowers/plans/2026-08-08-codeflowcanvas-groups.md`（Task 7-8）
- `docs/superpowers/plans/2026-08-07-codeflow-enrichment.md`（真机验证清单）

## 验收

CI 全绿 + 全部推送 + 验证证据存档（各 issue 见验收细则）。
