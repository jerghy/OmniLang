# 06 — 批次 1 域裁决：反馈与模态

Type: grilling
Status: open
Blocked by: 04, 05

## Question

**反馈与模态**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：toast/通知、modal/dialog、tooltip、骨架屏
起点：`docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md` §4.2 契约草案（showToast 内建语义函数 / `<dialog @modal={openSig}>` 元素指令 + @slot 投影）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据）
2. 入语言项的 1-2 句设计方向（语义意图 → 编译器选路 → 地道目标 codegen；React 原生 `<dialog>`/Flutter showDialog 先例方向）
3. 本批次阶段 spec 的范围边界（哪些进、哪些明确排除）
4. 与本批次相关的半实现信号/登记册扩展（semantic-universe 新域 10 反馈模态）

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。
