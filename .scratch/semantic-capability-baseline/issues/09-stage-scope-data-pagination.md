# 09 — 批次 4 域裁决：数据与异步能力

Type: grilling
Status: resolved
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

## Answer

**批次 4（数据与异步）域裁决定案**（2026-08-09，校准后标准）。

| 能力项 | 结论 | 三目标映射方向 |
|---|---|---|
| @load v0 边界补齐（多例/参数/Reset） | 🔜 **进语言（域 9 深化）**——**DataFetching 迁移去留定案：@load 扩展后迁移**（此前未决项；按新标准「功能该有」——双取数/Reset 意图经 @load 多例+参数+Reset 语义表达，learn-omniview DataFetching 随批次迁移） | React 三态多源 / Flutter FutureProvider 参数化 / OmniView 多装载 |
| 分页 | 🔜 **进语言**（@load 分页扩展：page/cursor 参数 + 页码状态机） | React TanStack Query 风格或编译器自生成 / Flutter infinite_scroll_pagination 风格或自生成——**库绑定 vs 自生成决策批次 spec 定**（audit：引库是真人主流，自生成是零依赖路线） |
| 无限滚动 | 🔜 **进语言**（装载触发语义——@load scroll 触发 / @for 容器滚动接近底部） | IntersectionObserver（web）/ ScrollController 阈值（Flutter） |
| 轮询 | 🔜 **进语言**（@load interval 参数） | refetchInterval / Timer.periodic + invalidate |
| 乐观更新 | 🔜 **进语言候选**（@load 写路径 mutate——远期：写路径语义化后随批次） | TanStack onMutate 快照回滚模式 / Flutter 手动回滚（编译器生成） |
| 防抖节流 | 🔜 **stdlib 语言函数**（debounce(fn, ms)/throttle(fn, ms)——时间语义跨平台一致，非视图意图） | 语言函数 → 各目标语言原语 |
| 缓存策略 | 🏞 配置面留生态（staleTime/GC 是库配置非功能意图）——失效意图 retry 已覆盖 | — |

**批次范围**：@load 深化（多例/参数/Reset/分页/轮询/无限滚动触发）为主体；乐观更新与写路径语义化（远期）；防抖节流 stdlib。DataFetching 迁移入本批次。
