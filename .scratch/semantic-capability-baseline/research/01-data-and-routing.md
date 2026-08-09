# 01 — 能力全景实证：数据/异步/路由

**日期**: 2026-08-09
**车票**: `issues/01-capability-landscape-data-and-routing.md`
**用途**: 边界裁决（三判据：跨目标一致表达 + 编译器理解意图 + 真实语料需求）的事实依据

## 来源说明（诚实声明）

本环境网络被完全封锁（WebFetch/WebSearch 均被拦截，含 MDN/tanstack.com/swr.vercel.app/reactrouter.com/riverpod.dev），无法现场抓取官方文档。本文件来源分两层：

1. **本地一手来源（仓库内，可复验）**：
   - `test/projects/learn-omniview/build/flutter-test-app/lib/LoadDemo.dart`（Riverpod 3.x 生成代码）+ `pubspec.yaml`（flutter_riverpod ^3.0.0）
   - `test/projects/learn-omniview/build/react-test-app/components/LoadDemo.tsx`（React @load 生成代码）
   - `docs/superpowers/specs/semantic-universe.md` 域 8/9（@form/@load 登记，2026-08-09）
   - `docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md` §4.3 域 C（导航意图契约草案）
   - `docs/omniview/16-forms-and-async-data.md`、`13-project-structure.md`
   - 仓库 node_modules 全量搜索：**未安装** react-router / @tanstack/react-query / swr / vue-router（可复验：`poiesis/node_modules/.pnpm` 无相关包）
2. **外部官方文档（既有精确知识，网络恢复后可复验）**：以下各节引用均为官方文档名（TanStack Query v5 Docs、SWR v2 Docs、React Router v7 Docs、Vue Router 4 Docs、SvelteKit Docs、Flutter API Docs、Riverpod 3.x Docs、MDN Web API）。所有 API 均为各库长期稳定接口，未编造；不确定的版本号一律不写。

---

## 数据（6 项）

### 1. 分页（服务端分页 / 游标分页）

**① 主流框架的标准形态（事实标准库）**
- React 端事实标准：**TanStack Query v5 `useInfiniteQuery`**（官方 guide《Infinite Queries》：`getNextPageParam` + `fetchNextPage` + `isFetchingNextPage`，官方示例同时给出**游标式**（projects/cursor）与**偏移式**（offset）两种服务端契约；`maxPages` 限制保留页数）。次标准：**SWR `useSWRInfinite`**（官方 docs《Pagination》：`getKey(pageIndex, previousPageData)` 逐页构造 URL，返回页面数组）。
- Flutter 端：无官方原语。事实标准为社区包 **infinite_scroll_pagination**（pub.dev，与 ListView.builder 组合）；Riverpod 官方 docs 无分页章节（社区结合 `FutureProvider`/`AsyncNotifier` 手写页码状态）。
- Vue 端：无专门库，VueUse `useInfiniteScroll` 只管触发不管取数；Svelte 端走 `@tanstack/svelte-query`（同 API）。

**② 生态化程度**：高（React 端双事实标准；Flutter 端社区标准，非官方）。

**③ 跨目标一致性**：**部分**。「按游标/偏移取下一页」的意图本身可抽象对齐（page/cursor 是纯数据概念，两端都能表达）；但服务端响应契约（分页信封形状）是后端职责非前端 DSL 职责；触发与渲染形态（React 库 API vs Flutter 包）差异大。

**④ 对 OmniLang 的启示**：**倾向留生态/暂不进语言**。三判据不满足：真实语料（learn-omniview 102 个 .omni）无任何分页用法；「取数据」意图已由 @load 覆盖（域 9），分页是数据源形态的变体，且游标状态机（hasMore/loadingMore）跨目标实现差异大——若语料出现可考虑 @load 扩展（分页参数），但当前 YAGNI。

### 2. 无限滚动

**① 主流框架的标准形态**
- 触发机制是 **web 平台标准 API：IntersectionObserver**（MDN：rootMargin/threshold 观察哨兵元素进入视口）——React/Vue/Svelte 均无框架级标准，库只组合观察器与取数（TanStack Query 官方 docs 明确无限查询「适用于无限滚动 UI」，但观察器需自写；VueUse `useInfiniteScroll` 封装）。
- Flutter 端：官方模式为 `ScrollController` + 接近底部阈值判断（`position.pixels >= maxScrollExtent - threshold`），或 NotificationListener 滚动通知；无官方原语。

**② 生态化程度**：中（触发观察器 = 平台 API + 各框架封装函数，无「事实标准库」概念）。

**③ 跨目标一致性**：**一致**。「滚动接近底部 → 装载下一页」意图在两端都可对齐（IntersectionObserver vs ScrollController 阈值），且 OmniLang 已有 `@onScroll` 语义事件（`{x, y, progress}`——scroll 语义为位置/进度，spec 2026-08-08 §3.1）。

**④ 对 OmniLang 的启示**：**倾向留生态/语言边界**。三判据 2（编译器理解意图）成立、判据 1 成立，但判据 3（语料需求）为零且组合表达可期（@onScroll + 装载触发的组合能力）；不构成独立语义域，记语言边界即可。与分页同批观察。

### 3. 乐观更新 / 撤销

**① 主流框架的标准形态**
- React 端事实标准模式：**TanStack Query v5《Optimistic Updates》guide**——`onMutate` 先写缓存（`setQueryData`）→ `onError` 用 `context` 快照回滚 → `onSettled` 失效重取；**撤销** = 该 guide 的 todo 示例：保存旧值、用户点 Undo 时 `setQueryData` 恢复。SWR 端：`useSWRMutation` 的 `populateCache` + `rollbackOnError`。
- Flutter 端：**无官方乐观更新原语**。标准做法是 store（Riverpod Notifier）内手动「先改本地状态 → 失败回滚」；无库级抽象。

**② 生态化程度**：中（React 端是 TanStack Query 的**文档化模式**而非独立库；Flutter 端无库）。

**③ 跨目标一致性**：**一致**。「先渲染结果、失败时回滚、可撤销」意图两端都能表达；但两端都是**手工模式**（无统一原语），且回滚需要「旧值快照」机制（context）。

**④ 对 OmniLang 的启示**：**倾向进语言候选（远期）/当前留边界**。判据 1/2 成立（编译器可理解「乐观提交」意图并选路：React onMutate 模式 / Flutter 手动回滚），但依赖 mutate 语义成熟（@load 只读、写路径尚未语义化），且语料零需求。记录为语言边界 + 触发条件，不立语法。

### 4. 防抖 / 节流

**① 主流框架的标准形态**：**无标准形态**。通用工具函数层：Lodash `debounce/throttle`（社区事实标准，与框架无关）；VueUse `useDebounceFn`/`useThrottleFn`（Vue 组合式封装）；**React 无官方**（官方 docs 甚至给出自定义 debounce hook 示例）；**Flutter 无官方**（Timer 手写）。

**② 生态化程度**：低（纯函数工具，无框架级标准；与 UI/组件无关的通用逻辑）。

**③ 跨目标一致性**：**一致**（时间语义（delay ms、leading/trailing）跨平台完全相同，与目标无关——它根本不是 UI 能力）。

**④ 对 OmniLang 的启示**：**倾向 stdlib/语言函数候选**。与视图意图无关，走既有「工具函数」通道（先例：`array_find` 等 stdlib 函数、string 工具）；若入语言，形态是 `debounce(fn, ms)` 语言函数而非视图域。三判据 2 弱（编译器无需理解意图——函数调用语义自明）。**注意**：现有 stdlib 是否有 debounce 需另行核对 `docs/omniview/12-standard-library.md`（本次未核）。

### 5. 数据缓存与失效

**① 主流框架的标准形态**
- React 端事实标准：**TanStack Query**（queryKey 驱动的缓存层 + `staleTime`/`gcTime` + `queryClient.invalidateQueries()` 失效 + refetchOnWindowFocus 自动重取；SWR 同构：全局缓存 + `mutate`/`revalidate`）。缓存与失效是这两个库的**核心卖点**。
- Flutter 端：**Riverpod 3.x 提供 `ref.invalidate`/`ref.refresh`**（异步状态生命周期管理：失效 = 重建 provider 重跑），但**不做网络响应缓存**（无 staleTime 概念——社区另有 riverpod 缓存插件，非官方）。

**② 生态化程度**：高（React 端库级标准；Flutter 端仅失效原语，无缓存层）。

**③ 跨目标一致性**：**部分**。「失效重跑」意图对齐（@load 的 retry 在两端分别生成 `load()` 复用 / `ref.invalidate(Provider)`——本地一手证据：LoadDemo.dart 第 31 行 `ref.invalidate(fetchItemsProvider)`）；「缓存策略（有效期/GC）」是库配置，跨目标无法也无需对齐。

**④ 对 OmniLang 的启示**：**倾向留生态 + 语言已覆盖失效意图**。@load 的 retry 已是「失效 = 重跑」语义（域 9 已实现）；缓存策略（staleTime/GC/持久化）属库层配置、无跨目标一致表达价值。语言边界注记：「显式缓存失效」= retry/重新装载意图，无新语法。

### 6. 轮询

**① 主流框架的标准形态**
- React 端事实标准：**TanStack Query `refetchInterval`**（官方 guide《Polling》——组件可见时每 N 毫秒自动重取）；SWR `refreshInterval` 同构。
- Flutter 端：**无官方轮询原语**（Timer.periodic + `ref.invalidate` 手写组合，Riverpod 官方无 polling 章节）。

**② 生态化程度**：中（React 端库内建参数；Flutter 端零）。

**③ 跨目标一致性**：**一致**。「每 N 秒自动刷新」意图可对齐（两端都能表达），且是编译器可理解的纯意图（interval 是配置不是逻辑）。

**④ 对 OmniLang 的启示**：**倾向进语言候选（随 @load 扩展）**。判据 1/2/3 中前两项成立；最自然形态是 @load 的 interval 参数或独立指令（编译器选路：React `refetchInterval` 参数 / Flutter Timer 接线）。优先级低于域 C，语料需求待证（learn-omniview 无轮询用例）。

---

## 异步（3 项）

### 7. 加载三态（loading / error / data）

**① 主流框架的标准形态**
- Flutter 端官方核心概念：**Riverpod 3.x `AsyncValue`**（`AsyncData`/`AsyncError`/`AsyncLoading` 三态 + `.when()` 分派 + `.isLoading`/`.asData`；官方 docs《Loading states》）。本地一手证据（`LoadDemo.dart`）：`FutureProvider<List<String>>` + `isLoading` 判空渲染 + `asData?.value` 取数 + error 经 `error?.toString()`。
- React 端无官方：TanStack Query v5（`isPending`/`isError`/`data`，v5 起 isLoading 语义收窄为 pending+首载）；SWR（`data`/`error`/`isLoading`/`isValidating`）。
- Vue/Svelte：VueUse `useAsyncState`；SvelteKit 内建 `load` + 顶层 await。

**② 生态化程度**：高（三态是**跨生态共同抽象**，各框架都有标准表达；Flutter 端是官方语言级概念）。

**③ 跨目标一致性**：**高度一致**——且 OmniLang 已实证：**@load 的 loading/error/data/retry 保留名在 React 端生成 `useState` 三态（LoadDemo.tsx 8-10 行）、Flutter 端生成 `{fn}Async.isLoading/.error/.asData`（LoadDemo.dart）、OmniView 端自绘**（semantic-universe 域 9 三目标代码锚点）。这是本清单中**唯一已完整进语言**的能力（2026-08-09 登记域 9）。

**④ 对 OmniLang 的启示**：**已进语言（先例，域 9 @load）**。三判据全成立：跨目标一致（AsyncValue 官方概念 ↔ 三态 state）、编译器理解意图（「这是异步数据源」）、真实语料需求（learn-omniview 实证）。后续能力裁决以此为模板。

### 8. 竞态处理

**① 主流框架的标准形态**：**无统一库原语**（多为库内建行为 + 手写模式）：
- web 平台标准：**AbortController / AbortSignal**（MDN）——请求取消的底层机制。
- TanStack Query/SWR：**请求去重内建**（同 key 并发只发一次）+ queryFn 注入 AbortSignal 支持取消 + 过期结果不覆盖新数据（自动丢弃）——竞态保护是库行为。
- React 手写模式：ignore flag / AbortController in useEffect cleanup。
- Flutter：Riverpod `ref.invalidate` 后旧 Future 结果自动丢弃（新装载覆盖旧状态）。

**② 生态化程度**：低-中（React 端库内建；Flutter 端框架内建行为；无「竞态库」概念）。

**③ 跨目标一致性**：**一致**（「丢弃过期响应」意图两端都成立且实现均内建于各自的装载机制——无需用户代码）。

**④ 对 OmniLang 的启示**：**留语言现状（编译器选路职责，不立语法）**。@load 的装载接线已继承目标端竞态保护（Flutter invalidate 丢弃旧结果）；显式取消（AbortController）若语料需要可经 ref/命令式通道，非意图级能力。记录为编译器责任（ADR-071 模式）。

### 9. 并行 / 超时

**① 主流框架的标准形态**：**语言级原语**（非框架库）：
- JS：`Promise.all` / `Promise.allSettled`（语言级）；超时 `AbortSignal.timeout()`（现代 web 标准）/ `Promise.race` 模式。
- Dart：`Future.wait`（SDK 级）；超时 `Future.timeout()`（SDK 级）。
- TanStack Query 提供 `useQueries`（并行查询组合）但底层仍是语言原语。

**② 生态化程度**：低（纯语言/平台能力，无框架标准库）。

**③ 跨目标一致性**：**一致**（Promise.all ↔ Future.wait、timeout 语义两端同构）——这也是现有 `std/async` 已对照的能力（车票原话「对照现有 std/async」）。

**④ 对 OmniLang 的启示**：**留语言级（stdlib/表达式系统）**。并行 = 表达式层并发原语（非视图意图），超时 = stdlib 函数候选；不构成语义域。若现有 std/async 已有 timeout/并行形态则维持，缺口（若有）随 stdlib 批次处理。

---

## 路由（7 项）

> 总览：域 C 设计草案（2026-08-08-semantic-design-coverage-design.md §4.3）已定意图方向——`@route "/todos/:id"` 页面声明 + `navigate(path, {params})` / `back()` 语义函数 + 参数 prop 注入；三目标 React Router（事实标准）/ Flutter Navigator（官方）/ OmniView hash 路由自绘；**决策点已明示：URL 深链跨目标差异大，行为对等 floor 界定是本域设计核心**。以下逐项实证。

### 10. 页面切换

**① 主流框架的标准形态**
- React：**React Router v7**（事实标准，`<Routes>/<Route path>` 声明式）；Vue：**Vue Router 4**（官方框架路由）；Svelte：**SvelteKit 文件路由**（官方内建，`+page.svelte`）；Solid：Solid Router。
- Flutter：**Navigator（官方，命令式 push/pop）** + **GoRouter（flutter.dev 官方推荐，声明式 path 路由）**。

**② 生态化程度**：极高（路由是每个框架的官方一等公民——除 React 外全是框架自带；React 靠事实标准库补位）。

**③ 跨目标一致性**：**一致**（「页面组件 + 路径声明」意图两端可对齐；React Router 声明式 ↔ GoRouter 声明式天然同构）。

**④ 对 OmniLang 的启示**：**进语言（域 C 方向已定）**。三判据全成立：跨目标一致（声明式路由是两端主流）、编译器理解意图（页面切换）、语料需求（learn-omniview 9 标签页目前靠 PageSwitcher 条件渲染手写模拟——设计文档 §6 已列「PageSwitcherDemo → 导航原语」重构点）。

### 11. 路由参数

**① 主流框架的标准形态**：React Router `useParams` + `:id` 路径段；Vue Router `route.params`；SvelteKit `$page.params`；Flutter GoRouter `pathParameters`——**四端同一 `:segment` 路径模式**。

**② 生态化程度**：极高（全部官方内建，且**语法同构**——`:id` 段是跨生态事实标准）。

**③ 跨目标一致性**：**一致**（`:id` 模式 + 参数注入意图全端对齐；域 C 草案「参数经 prop 注入」与 GoRouter 的 `extra`/参数注入可映射）。

**④ 对 OmniLang 的启示**：**进语言（随域 C）**。`:segment` 路径模式是跨生态最强一致的语法形态——语言面 `@route "/todos/:id"` 可直接采用，编译器按目标发射。

### 12. 返回 / 前进

**① 主流框架的标准形态**：React Router `useNavigate` + `navigate(-1)`（history 栈）；Vue Router `router.back()`；Flutter Navigator `Navigator.pop()`（**栈模型**——push/pop 是 Flutter 路由核心心智）；web 平台底层 history API（back/forward）。

**② 生态化程度**：极高（全部官方内建）。

**③ 跨目标一致性**：**一致**（「返回上一页」意图两端可对齐：web 历史栈 vs Flutter 导航栈；语义一致、实现机制不同——这正是编译器选路职责）。

**④ 对 OmniLang 的启示**：**进语言（随域 C）**。域 C 草案已有 `back()` 语义函数；`navigate(-1)` vs `Navigator.pop()` 是典型「同意图不同实现」——编译器选路的教科书案例。

### 13. 守卫 / 权限路由

**① 主流框架的标准形态**：**机制差异大**：
- Vue Router 4：导航守卫内建（`beforeEach` 命令式钩子）；Flutter GoRouter：`redirect` 回调内建（声明式重定向）；SvelteKit：`load` 内 `redirect()`。
- React Router v7：**无独立守卫原语**——标准模式是 data router 的 loader 内重定向（或 `<Navigate>` 条件渲染）。

**② 生态化程度**：高（每端都有官方机制，但形态各异：命令式钩子 / 声明式重定向 / loader 内抛转）。

**③ 跨目标一致性**：**一致（意图层）**。「未授权 → 重定向到登录页」意图两端都成立；实现机制差异大（Vue 钩子 vs GoRouter redirect vs React loader），编译器选路空间大。

**④ 对 OmniLang 的启示**：**进语言候选（随域 C 二期）**。判据 1/2 成立；判据 3（语料需求）弱（learn-omniview 无权限场景）。域 C v0 可不含守卫，记录为扩展点（@route 的守卫属性或独立指令），语料出现再立。

### 14. 懒加载

**① 主流框架的标准形态**：React `React.lazy` + `<Suspense>`（官方），React Router v7 路由级 `lazy` 属性；Vue Router 动态 import（`() => import()` 官方推荐）；SvelteKit 文件路由天然 code-split（官方内建）；**Flutter：无页面级懒加载概念**（单 bundle，编译期 tree-shaking 决定首载，运行时无按需加载语义）。

**② 生态化程度**：中（web 三端官方内建；**Flutter 端缺位**）。

**③ 跨目标一致性**：**不一致**——Flutter 无对应概念，这是本清单少数判据 1 不成立的能力（「按需加载页面」意图在移动端无标准表达）。

**④ 对 OmniLang 的启示**：**留生态 / 不支持（或编译器优化职责）**。三判据 1 不满足 → 不立语法；若未来需要，形态是**编译器选路优化**（React 端 code-split、Flutter 端无操作 + 诚实降级注记），而非语言意图。

### 15. 深链 / URL 同步

**① 主流框架的标准形态**
- web 端：URL 是路由的唯一事实源（React Router/Vue Router/SvelteKit 全部以 URL 为状态中心；hash/history 两种模式）。
- Flutter 移动端：**无 URL 概念**（Navigator 纯内存栈；GoRouter 用路径字符串模拟 URL 但非浏览器地址栏）；Flutter web 端才有真实 URL 同步。
- 平台层：web URL API（MDN：location/history/pushState）。

**② 生态化程度**：高（web 端全员内建；Flutter 移动端本质缺位，GoRouter 是桥接）。

**③ 跨目标一致性**：**不一致（本清单跨目标差异最大项）**——「URL 反映页面状态、可分享可直达」意图在移动端无对应物。设计文档 §4.3 已明示：floor 界定 = 页面切换/参数/返回在 floor 内，**深链/URL 同步为各目标差异 + 诚实降级注记**。

**④ 对 OmniLang 的启示**：**进语言但声明降级边界（域 C 核心决策点，方向已定）**。意图可表达（@route 路径声明隐含 URL 语义），但跨目标行为不对等——语言面保留意图、目标面诚实降级（OmniView hash 路由 / Flutter 无 URL 注记）；这同时是「floor 界定」的设计样板。

### 16. 嵌套路由

**① 主流框架的标准形态**：React Router `<Outlet/>`（布局插槽）；Vue Router 嵌套 `<router-view>`；SvelteKit 目录结构 + `+layout.svelte`（布局路由内建）；Flutter：**无原生嵌套路由**（Navigator 平栈模型；GoRouter `StatefulShellRoute`/`ShellRoute` 是较新模拟，官方文档有但非一等公民）。

**② 生态化程度**：高（web 三端官方内建；Flutter 端弱）。

**③ 跨目标一致性**：**部分**——「布局内嵌子页面」意图可对齐（web 三端一致），Flutter 端 ShellRoute 可映射但语义（子路由栈管理）更复杂。

**④ 对 OmniLang 的启示**：**进语言候选（随域 C）**。OmniLang 组件树天然嵌套（组件内嵌组件已有 @slot 投影）——「嵌套路由 = 组件嵌套 + 布局插槽」可由既有表达组合；是否需要独立语义（layout 声明）待域 C 细化。

---

## 清单对照

基线清单 16 项全数覆盖（分页/无限滚动/乐观/防抖节流/缓存失效/轮询/加载三态/竞态/并行超时/路由切换/参数/返回/守卫/懒加载/深链/嵌套）。

**遗漏补充（基线未列、研究中出现的相邻能力）**：

| 候选能力 | 事实标准形态 | 初步倾向 |
|---|---|---|
| 数据获取/请求本身（fetch 调用） | 各框架由装载库覆盖；web fetch API 平台标准 | 已被 @load 覆盖，无新语义 |
| URL 查询参数（query string） | React Router `useSearchParams`/Vue Router `route.query`/GoRouter `uri.queryParameters` | 随域 C 深链一并裁决 |
| 预取（prefetch） | TanStack Query `prefetchQuery`；SvelteKit link prefetch（官方内建） | 留生态（编译器优化职责候选） |
| 请求去重 | TanStack Query/SWR 内建（同 key 并发合并） | 编译器选路职责，不立语法（同竞态项） |
| 重试策略（backoff） | TanStack Query `retry` 参数；Flutter 无官方 | 留生态（库配置） |
| 缓存持久化（离线） | TanStack Query persistQueryClient 插件；非官方插件 | 留生态（明确不支持） |
| 滚动恢复（scroll restoration） | React Router `<ScrollRestoration/>` 内建；Flutter 无直接对应 | 留生态/语言边界 |
| 分页跳页（page jump） | 各库 page 参数（与游标不同轴）；分页控件 UI 在组件层 | 随分页一并观察 |

**裁决输入汇总（跨目标一致性判据速查）**：

| 能力 | 跨目标一致 | 生态标准形态 | 倾向 |
|---|---|---|---|
| 加载三态 | 高度一致（官方概念） | React 无官方 / Flutter AsyncValue 官方 | **已进语言**（域 9 @load 先例） |
| 路由切换/参数/返回 | 一致 | 全端官方/事实标准 | **进语言**（域 C 方向已定） |
| 深链/URL 同步 | 不一致（移动端缺位） | web 全员 / Flutter 无 | **进语言 + 降级边界**（域 C floor 界定） |
| 守卫 | 意图一致、机制差异大 | 全端官方 | 进语言候选（域 C 二期） |
| 嵌套路由 | 部分（Flutter 弱） | web 三端官方 | 进语言候选（随域 C） |
| 轮询 | 一致 | React 库内建 / Flutter 无 | 进语言候选（@load interval 扩展） |
| 乐观更新 | 一致但均为手工模式 | React 模式化 / Flutter 无 | 语言边界（远期候选） |
| 分页 | 部分（触发/渲染差异大） | React 双标准 / Flutter 社区 | 留生态（语料零需求） |
| 无限滚动 | 一致 | 平台 API + 封装 | 留生态/语言边界 |
| 竞态 | 一致（库内建） | 库/框架内建行为 | 编译器选路职责 |
| 并行/超时 | 一致 | 语言级原语 | 留语言级（stdlib） |
| 防抖节流 | 一致（非 UI 能力） | 无标准（工具函数层） | stdlib/语言函数候选 |
| 缓存与失效 | 部分（失效一致、策略不一致） | React 库核心 / Flutter 仅失效 | 留生态（retry 已覆盖失效意图） |
| 懒加载 | 不一致（Flutter 缺位） | web 三端官方 | 留生态/不支持 |

**总体形态判断**：数据/异步簇的「标准形态」集中在**库层与平台层**（TanStack Query/SWR/Riverpod/AbortController/语言原语），仅加载三态已入语言；路由簇的「标准形态」集中在**框架官方层**（除 React 外全为框架自带），且 `:id` 路径段、返回栈、页面声明跨生态高度同构——路由是比数据更「语言友好」的域，与域 C 优先级排序（表单+数据三态 → 反馈模态 → 导航 → 响应式，导航后置因跨目标差异最大需细究 floor）吻合。
