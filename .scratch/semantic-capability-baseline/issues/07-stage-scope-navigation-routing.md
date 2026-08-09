# 07 — 批次 2 域裁决：导航与路由

Type: grilling
Status: open
Blocked by: 04, 05

## Question

**导航与路由**能力的边界裁决与设计方向——按 05 定稿的裁决模板执行。

覆盖能力项：页面切换、路由参数、返回/前进、守卫/权限路由、懒加载、深链/URL 同步、嵌套路由
起点：`docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md` §4.3 契约草案（`@route "/todos/:id"` 声明 + `navigate(path, {params})` / `back()` 内建；深链/URL 同步为各目标差异 + 诚实降级注记；floor 界定是设计核心——四域中跨目标差异最大）

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据；注意研究 01 的路由生态全景）
2. 入语言项的 1-2 句设计方向（`@route`/`navigate`/`back` 意图 → React Router/Flutter Navigator 选路）
3. **floor 界定**：跨目标差异最大的域——哪些语义 floor 内对等、哪些诚实降级
4. 本批次阶段 spec 的范围边界

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 本批次即成为路线图一个阶段（后续独立 spec→plan→SDD）。
