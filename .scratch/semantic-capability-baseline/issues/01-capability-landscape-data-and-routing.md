# 01 — 能力全景实证：数据/异步/路由

Type: research
Status: resolved
Blocked by:

## Question

主流前端框架与生态（React/Vue/Svelte/Solid/Flutter/web 平台）在**数据与异步、导航路由**两簇能力的全景形态是什么？为边界裁决（三判据）提供事实依据。

覆盖能力项（基线清单草稿）：
- 数据：分页（服务端/游标）、无限滚动、乐观更新/撤销、防抖节流、数据缓存与失效、轮询
- 异步：加载三态、竞态处理、并行/超时（对照现有 std/async）
- 路由：页面切换、参数、返回/前进、守卫/权限路由、懒加载、深链/URL 同步、嵌套路由

## 产出要求

高可信一手来源（官方文档优先）——不写二手博客。产出 `.scratch/semantic-capability-baseline/research/01-data-and-routing.md`，每项能力记录：
1. 主流框架的标准形态（谁是事实标准库：React Router/TanStack Query/SWR/Vue Router/SvelteKit/Flutter Navigator…）
2. 生态化程度（有标准库=留生态候选；无标准形态=进语言候选）
3. 跨目标一致性（同一意图在 React/Flutter 两端能否对齐——进语言判据的输入）
4. 对 OmniLang 的启示（1-2 句：进语言/留生态/不支持的初步倾向 + 依据）

## 解析即

研究完成 → 车票加 `## Answer`（要点 + 产出文件链接）+ `Status: resolved` + 地图 Decisions-so-far 追加一行。

## Answer

研究完成，产出 `research/01-data-and-routing.md`（16 项能力全实证 + 遗漏补充 8 项 + 裁决速查表）。要点：
- **已进语言**：加载三态（域 9 @load 先例，React useState 三态 / Flutter AsyncValue 本地代码锚点实证）——本清单唯一已完整进语言的能力，后续裁决模板。
- **进语言（域 C 方向已定）**：路由页面切换/参数/返回（`:id` 路径段跨生态语法同构，判据 1 最强项）；深链/URL 同步进语言但声明降级边界（跨目标差异最大，Flutter 移动端无 URL 概念——域 C floor 界定核心决策点）。
- **进语言候选（低优先级）**：守卫（域 C 二期扩展点）、嵌套路由（Flutter ShellRoute 弱，可由组件嵌套组合）、轮询（@load interval 参数扩展，语料需求待证）、乐观更新（远期，依赖 mutate 语义成熟）。
- **留生态/不立语法**：分页/无限滚动（语料零需求，YAGNI）、缓存策略（retry 已覆盖失效意图）、竞态/请求去重（编译器选路职责）、并行/超时（留语言级 std/async）、防抖节流（stdlib 函数候选）、懒加载（Flutter 缺位，判据 1 不成立）。
- **网络限制声明**：环境网络封锁无法抓取官方文档；以仓库内一手证据（LoadDemo.dart/tsx、pubspec riverpod ^3.0.0）+ 对稳定库官方 API 的既有精确知识为据，版本号仅本地实证者引用。
