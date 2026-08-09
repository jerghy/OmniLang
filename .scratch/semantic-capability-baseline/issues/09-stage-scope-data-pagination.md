# 09 — 批次 4 域裁决：数据与异步能力

Type: grilling
Status: open
Blocked by: 04, 05

## Question

**数据与异步**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：
- 分页（服务端分页/游标）、无限滚动
- 乐观更新/撤销
- 防抖节流（现状：无用户级 API——运行时绑定内建防抖是两方绑定实现细节，非用户面）
- 数据缓存与失效、轮询
- 现状基线：@load 三态（v0：仅 Array/单例/mount 自动）、std/async（spawn/all/race/timeout）、DataFetching 迁移去留（未决，V-F6/V-F7）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据；注意研究 01 的数据生态全景——TanStack Query 等事实标准）
2. 入语言项的 1-2 句设计方向（@load 强化 vs 新原语；分页是语法+库组合还是语言原语）
3. **DataFetching @load 迁移去留**拍板（此前未决项）
4. @load v0 边界（仅 Array/单例）在能力基线上的处置（补齐 or 明确边界）
5. 本批次阶段 spec 的范围边界

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。
