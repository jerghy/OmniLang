# 04-verification-evidence: 验证放行（真机验证证据补齐）

Status: ready-for-agent

## 背景

阶段 0 第 4 项（ROADMAP.md 0.4）。已实现功能缺真机验证证据：

1. **Groups Task 8**（`docs/superpowers/plans/2026-08-08-codeflowcanvas-groups.md`）：10 项 Victauri 清单——dom_snapshot → Shift+click → 组成分组 → 折叠 → 展开 → Ctrl+S → project.json `codeFlowGroups` → reload → `.omni` 含 `// ── 组名 ──` 注释 → 自动布局 → 截图
2. **Enrichment 验证清单**（`docs/superpowers/plans/2026-08-07-codeflow-enrichment.md` 2961-2964 行）：npm test 绿 + tsc 干净 + 手工真机（Victauri）learn-omniview——10 个任务实现已提交，但**跑验证的证据未找到**

## 任务

- [ ] groups Task 8 真机 10 项（Victauri + 截图存档）
- [ ] enrichment 验证补跑（npm test + tsc + 真机）并留存证据

## 验收

- 两组验证证据存档（截图/断言记录）
- Groups Task 8 关闭

## 关联

- ROADMAP.md 0.4
- `docs/superpowers/plans/2026-08-08-codeflowcanvas-groups.md`（Task 8）
- `docs/superpowers/plans/2026-08-07-codeflow-enrichment.md`（验证清单 2961-2964）

## Comments
