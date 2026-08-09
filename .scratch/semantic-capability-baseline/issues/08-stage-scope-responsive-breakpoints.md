# 08 — 批次 3 域裁决：响应式断点

Type: grilling
Status: open
Blocked by: 04, 05

## Question

**响应式与样式断点**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：
- 断点条件（`@if bp("md")` 条件表达式或变体维度——2026-08-08 §4.4 草案：design 域 `breakpoint md: 768` token + 使用形态）
- 用户级 `@media` 断点样式（现状：仅主题生成 prefers-color-scheme，无用户级媒体查询——属样式域，跨域协调）
- 容器查询/自适应布局（研究 02 的样式生态全景输入）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据）
2. 入语言项的 1-2 句设计方向（breakpoint token + `bp("md")` → React matchMedia/Flutter MediaQuery/OmniView matchMedia）
3. 与样式批次（车票 10）的边界划分（断点条件 vs @media 块样式谁管）
4. 本批次阶段 spec 的范围边界

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。
