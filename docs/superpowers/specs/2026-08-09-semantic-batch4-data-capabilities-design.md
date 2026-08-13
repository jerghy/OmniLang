# 批次 4 数据能力 设计 spec（2026-08-09-semantic-batch4-data-capabilities-design.md）

**日期**: 2026-08-09
**状态**: 详细级设计（待 writing-plans）
**上游**: `2026-08-09-semantic-capability-baseline-design.md` §4 批次 4 + §3.1 真人写法标准；车票 `.scratch/semantic-capability-baseline/issues/09-stage-scope-data-pagination.md`（域裁决定案）；研究 `.scratch/semantic-capability-baseline/research/01-data-and-routing.md`
**配套计划**: `D:\space\OmniLang\.superpowers\sdd\semantic-batch-specs-plans\batch4-*.md`（writing-plans 生成）
**报告**: `D:\space\OmniLang\.superpowers\sdd\semantic-batch-specs-plans\batch4-spec-report.md`

---

## 1. 背景与范围

本批次实现基线 §4 批次 4「数据能力（@load 深化）」：@load v0 边界补齐（多例/参数/Reset）+ 分页 + 无限滚动 + 轮询 + 防抖节流 stdlib + DataFetching 迁移 + 乐观更新边界记录。现状基线（实证）：@load 三态已实现（域 9 登记；`LoadIntentIR` 仅 `fn_name`、每组件至多一个、仅 Array 返回、mount 自动装载）；`std/async` 已支持 async fn/await/并行/超时（`pub async fn` 语料实证：`lib/api.omni`）。

| 能力项 | 上游裁决 | 本批次状态 |
|---|---|---|
| @load 多例/参数/Reset | 🔜 进语言（域 9 深化）；DataFetching 迁移定案：@load 扩展后迁移 | 进语言（§2.1） |
| 分页（page 参数 + 页码状态机） | 🔜 进语言；库绑定 vs 自生成批次 spec 定 | 进语言（§2.2）——自生成定案，依据见 §2.2 |
| 无限滚动（装载触发语义） | 🔜 进语言（@load scroll 触发） | 进语言（§2.3） |
| 轮询（interval 参数） | 🔜 进语言（@load interval 参数） | 进语言（§2.4） |
| 防抖节流 | 🔜 stdlib 语言函数（时间语义跨平台一致） | 进语言（§2.5） |
| 乐观更新 | 🔜 进语言候选（远期——写路径语义化后随批次） | 语言边界记录（§6） |
| 缓存策略 | 🏞 配置面留生态（失效意图 retry 已覆盖） | 语言边界记录（§6） |
| DataFetching 迁移 | 本批次随 @load 扩展迁移 | demo 迁移（§8） |
| form.submitting 恒 false / Flutter form.valid 恒 true | 基线 §8：标 v0 桩 + 补齐批次，语言面不动 | 记录（§6），非本批范围 |

**半实现信号注记**：基线 §8 半实现信号三件中，@load v0 边界（仅 Array/单例）由本批次补齐；`form.submitting` 恒 false 与 Flutter `form.valid` 恒 true 属表单域，不随本批次（表单深化在远期批次），语言面不动，仅记录。

---

## 2. 语义意图定义

### 2.1 @load 深化：多例 / 参数 / Reset

**意图一句话**：一个组件可以声明多个独立数据源（多例），每个数据源可以有输入参数（参数绑定），并可以显式清空回未装载态（Reset）——编译器为每个实例生成独立的加载三态信号与重试/重置回调。

**语法形态草案**：

```omni
component Orders {
    // 命名实例：信号 = itemsLoading/itemsError/itemsData + itemsRetry()/itemsReset()
    @load items={fetchOrders}

    // 匿名实例：v0 兼容（保留 loading/error/data/retry/reset 保留名）
    @load={fetchQuick}

    // 参数绑定：装载函数调用形式，参数可为信号/表达式
    @load stats={fetchStats(token)}          // token 为信号——变化时自动重新装载
    @load detail={fetchDetail(id, "fixed")}  // 混合：信号 + 字面量

    async fn fetchOrders() -> Array<Order> { ... }        // 装载函数可为 async fn
    fn fetchQuick() -> Array<Quick> { ... }
    async fn fetchStats(token: String) -> Stats { ... }
    async fn fetchDetail(id: i32, mode: String) -> Detail { ... }

    view {
        @load items={fetchOrders}   // 视图级指令语句（v0 同位置）
        @if itemsLoading { ... }
        @if itemsError != "" { ... }
        @for item in itemsData { ... }
        <button @onPress={itemsRetry}>重试</button>
        <button @onPress={itemsReset}>重置</button>
    }
}
```

**BNF 产生式草案**：

```
load_stmt     := '@load' load_spec
load_spec     := '{' fn_ref '}'                 (* 匿名实例：v0 兼容 *)
               | ident '=' '{' fn_ref '}'       (* 命名实例 *)
fn_ref        := identifier                     (* 引用形式：无参装载 *)
               | call_expr                      (* 调用形式：参数绑定 *)
```

**保留名与命名规则（核心设计决策）**：

| 实例形态 | 生成信号/函数保留名 |
|---|---|
| 匿名 `@load={fn}` | `loading` / `error` / `data` / `retry` / `reset`（v0 四名 + 新增 `reset`） |
| 命名 `@load name={fn}` | `{name}Loading` / `{name}Error` / `{name}Data` / `{name}Retry` / `{name}Reset`（camelCase 拼接） |

- `data` 信号可空化：`null` = 未装载（mount 前）/已 Reset。React 生成初始值 `[]` → `null`（渲染处已有 `?? []` 兜底先例）；Flutter `asData?.value ?? const []` 天然 null 安全；OmniView JS 槽位初值 `[]`（view 侧 `{name}Data` 初值 null 兜底渲染）。空闲态渲染 = `@if data == null && !loading`——与 DataFetching 迁移的 idle 态对齐（§8 对照表）。
- `Reset` 语义 = 清空回未装载态（data→null、error→null、loading→false），**不自动重载**——与 DataFetching `fn reset()` 行为对齐；`retry` 保持 v0 语义（不清数据重新装载）。
- 参数绑定语义：参数值变化 → 自动重新装载（React 依赖数组 / Riverpod `ref.watch` 的真人写法先例）；参数为字面量时仅装载一次。暂停自动重载不在 v0（见 §9 P7 未决）。
- 装载函数可为普通 `fn` 或 `async fn`（`pub async fn` 已存在，api.omni 实证）——真实 HTTP 装载路径（DataFetching 迁移依赖此点）。

**与既有语法的关系/冲突检查**：

| 冲突面 | 检查结果 |
|---|---|
| 保留名 `loading/error/data/retry`（域 9 已登记） | 匿名实例维持不变；新增 `reset` 与二者并列登记（§5.1 域 9 扩展） |
| 命名实例信号 `{name}Loading` 等与组件既有信号/函数同名 | 硬错误 V2（§3） |
| 实例名取保留名（`@load loading={fn}`） | 硬错误 V2——保留名专供匿名实例 |
| 匿名实例出现多次 | 硬错误 V6（v0 V-F6 升级：多例必须命名，匿名每组件至多一个） |
| 与 @form 的关系 | 独立维度，无词汇交集；V-F6「组件内 @form/@load 至多一个」升级为「@form 至多一个 + 匿名 @load 至多一个 + 命名 @load 数量不限」 |
| `reset` 与组件内自定义 `fn reset()` | 匿名实例时冲突（V2）——DataFetching 迁移后原 `fn reset()` 删除，由 `{name}Reset()` 取代（§8） |

**IR 形态扩展方向**（编译器职责面）：`LoadIntentIR { fn_name: String }` → `LoadIntentIR { name: Option<String>, loader: FnRef（引用或调用表达式）, mode: LoadMode }`；`ComponentIR.load_intent: Option<LoadIntentIR>` → `Vec<LoadIntentIR>`（组件收集处 `component.rs` 扫描/上报同步扩展，V-F6 诊断改写）。

### 2.2 分页（页码状态机）

**意图一句话**：数据源按页返回，用户触发装载下一页（「加载更多」按钮或滚动触发），编译器管理页码状态机（当前页/是否还有下一页/是否正在装载下一页）。

**语法形态草案**：

```omni
component History {
    // 分页装载：page 参数由编译器注入（首页 = 1）；返回 Page<T> 信封
    @load history={fetchHistory} paginated

    fn fetchHistory(page: i32) -> Page<HistoryItem> {
        return fetchPage(page);   // 服务端契约由 loader 内部拼装
    }

    view {
        @load history={fetchHistory} paginated
        @for item in historyData { ... }
        <button @onPress={historyLoadMore} @if historyHasMore>加载更多</button>
        @if historyLoadingMore { <p>装载下一页…</p> }
        <button @onPress={historyReset}>重置</button>
    }
}
```

**BNF 产生式草案**（并入 2.1 的 load_spec）：

```
load_spec := '{' fn_ref '}' | ident '=' '{' fn_ref '}'
           | load_spec 'paginated'                (* 分页：必须命名实例 *)
```

**约定契约**：
- **分页实例必须命名**（`paginated` 与匿名组合 → 硬错误 V7）——分页信号（`{name}Page`/`{name}HasMore`/`{name}LoadingMore`）需归属。
- 装载函数签名约定：`fn {name}(page: i32) -> Page<T>`；`Page<T>` 为语言内建信封类型 `{ items: Array<T>, hasMore: Bool }`（见 §5 登记）。
- 编译器生成：`{name}Data`（items 解包后，`Array<T>`）、`{name}Page`（当前页码，首载后 = 1）、`{name}HasMore`、`{name}LoadingMore`、`{name}LoadMore()` 回调（page+1 装载追加）、`{name}Reset()`、`{name}Retry()`。
- 装载失败：追加页失败进入 `{name}Error`（保留已装载数据），`{name}LoadMore` 可重试。
- **翻页语义跨目标一致**：`{name}Retry` / `{name}Reset` 均回到第 1 页（页码重置后装载）；OmniView 端 retry 装载结果 `data = r.items`（首页覆盖，非追加——追加仅 `{name}LoadMore` 路径）。

**库绑定 vs 编译器自生成——定案：v0 编译器自生成**。依据：
1. 与 @load v0 先例一致——React 端三态自生成（`useState` + `useCallback`，零依赖），Flutter 端 Riverpod 基座内自生成；分页状态机同属「编译器选路职责」（ADR-071 模式）。
2. 避免双事实标准二选一的绑定风险：React 端 TanStack Query 与 SWR 并列（研究 01 ①），选任一即把生态竞争压给用户；自生成零依赖、与既有生成形态同构。
3. 库绑定（`useInfiniteQuery` / `infinite_scroll_pagination`）登记为后续增强路径（§6 触发条件）——触发器语义（scroll/按钮）独立于取数实现，未来可替换。

**冲突检查**：`paginated`/`page`/`hasMore` 均为 @load 局部词汇（指令参数面），不与顶层指令/事件/保留名冲突；`Page<T>` 与 `Array<T>` 并列登记进类型语义集（§5.1 域 13）。

### 2.3 无限滚动（装载触发语义）

**意图一句话**：列表滚动接近底部时自动装载下一页——无需用户点击「加载更多」，装载触发由编译器在滚动容器上安装观察器。

**语法形态草案**：

```omni
component Feed {
    @load feed={fetchFeed} paginated scroll   // scroll = 滚动接近底部自动 LoadMore

    fn fetchFeed(page: i32) -> Page<FeedItem> { ... }

    view {
        <div @load feed={fetchFeed} paginated scroll>   // 容器须可滚动（V10 warning）
            @for item in feedData { ... }
        </div>
    }
}
```

**BNF**：`load_spec := ... | load_spec 'scroll'`（约束：scroll 必须与 paginated 组合——硬错误 V8）。

**语义定义**：编译器在 @load 所在滚动容器的列表尾部安装哨兵（web/OmniView：`IntersectionObserver` 观察最后一个列表项进入视口；Flutter：`ScrollController` 滚动位置 ≥ maxScrollExtent - 阈值）。触发时调用 `{name}LoadMore()`（受 `{name}LoadingMore` 与 `{name}HasMore` 双重防护，不重复触发）。卸载时断开观察器。

**与既有语法关系**：`@in-view` 是 motion 触发器（动画语义，@enter/@in-view/@scroll 同族），不复用为装载触发——装载触发是数据语义（ADR-071 编译器责任），独立参数 `scroll` 表达；与 `@onScroll` 事件（`{x, y, progress}` 位置/进度语义）互不冲突（不同词汇面）。

**冲突检查**：`scroll` 为 @load 局部词汇；@in-view/@onScroll 均为既有词汇，无交集。

### 2.4 轮询（interval 参数）

**意图一句话**：数据源按固定间隔自动重新装载（组件可见期间）——「每 N 毫秒刷新一次」纯意图，间隔是配置不是逻辑。

**语法形态草案**：

```omni
component Prices {
    @load prices={fetchPrices} interval: 5000   // 每 5 秒自动重新装载

    async fn fetchPrices() -> Array<Price> { ... }

    view {
        @load prices={fetchPrices} interval: 5000
        @for p in pricesData { ... }
    }
}
```

**BNF**：`load_spec := ... | load_spec 'interval' ':' INTEGER`（毫秒正整数——硬错误 V5）。

**语义定义**：interval 期间组件卸载 → 定时器清理（React `clearInterval` cleanup / Flutter `dispose` 取消 `Timer.periodic`）；每次轮询 = 重新装载（复用装载路径，等价 retry 语义）；轮询与分页组合 v0 不支持（§9 P3 未决——interval 仅作用于普通装载实例）。

**与既有语法关系**：冒号参数形态与 motion `over: 400ms` 同族（kebab 指令 + 冒号参数语言风格）；`interval` 数值单位 = 毫秒整数（与 std/async `async_sleep(ms)` 单位一致）。

**冲突检查**：`interval` 为 @load 局部参数词，无既有词汇冲突。

### 2.5 防抖节流（stdlib 语言函数）

**意图一句话**：高频事件处理器的时间语义包装——`debounce(fn, ms)` 延迟到事件停止后触发一次，`throttle(fn, ms)` 固定节流窗口内至多触发一次；时间语义跨平台一致（非视图意图，走 stdlib 通道——车票定案）。

**语法形态草案**：

```omni
import { debounce, throttle } from "std/timing";

component Search {
    fn doSearch(q: String) { ... }

    view {
        <input @onInput={debounce(doSearch, 300)}></input>   // 输入停止 300ms 后触发
        <button @onPress={throttle(handleRefresh, 2000)}>刷新</button>
    }
}
```

**签名**：`debounce(fn, ms) -> fn`；`throttle(fn, ms) -> fn`。ms 为正整数（硬错误 V9）；默认 trailing 触发、不暴露 leading/trailing 配置（YAGNI，§9 P6 未决确认）。返回包装函数可直接绑定事件；事件处理器引用身份变化时包装器保持有效（编译器生成稳定引用形态，§4）。

**与既有语法的关系**：新 stdlib 模块 `std/timing`（与 std/async 并列——std/async 为异步原语，timing 为时间节流原语；snake_case 命名与既有 stdlib 一致：`async_sleep`/`array_find` 先例）。debounce/throttle 的返回值是函数，绑定进 `@onInput={...}` 表达式——与既有事件绑定表达式系统无缝衔接。

**冲突检查**：`std/timing` 模块名/`debounce`/`throttle` 均为新词汇，与 std 既有模块（console/http/json/array/string/math/storage/async）无重名；与视图指令/事件名无交集。

---

## 3. Validation 清单

| 编号 | 级别 | 触发 | 错误消息草案（中文） |
|---|---|---|---|
| V1 | 解析错误 | 命名实例名非法（非标识符） | 「@load 实例名称须为合法标识符，当前：`{token}`」 |
| V2 | 语义硬错误 | 实例名与保留名/既有信号/函数/其他实例名冲突 | 「@load 实例名称 `{name}` 与保留名/信号 `{conflict}` 冲突——保留名 loading/error/data/retry/reset 专供匿名实例，请更换实例名」 |
| V3 | 语义硬错误 | 两个命名 @load 同名 | 「组件内 @load 实例名称重复：`{name}`——每个实例须唯一命名」 |
| V4 | 语义硬错误 | 命名实例信号派生名与既有信号冲突 | 「@load 实例 `{name}` 生成的信号 `{derived}` 与组件信号/函数冲突，请更换实例名」 |
| V5 | 语义硬错误 | interval 非正整数 | 「@load interval 参数须为正整数毫秒（如 interval: 5000），当前：`{v}`」 |
| V6 | 语义硬错误 | 匿名 @load 出现多次（v0 V-F6 升级） | 「组件内至多一个匿名 @load——多个数据源请使用命名实例（@load name={fn}），当前匿名 {n} 个」 |
| V7 | 语义硬错误 | 分页实例匿名 | 「分页 @load 必须命名（@load name={fn} paginated）——分页信号 page/hasMore/loadingMore 需要实例归属」 |
| V8 | 语义硬错误 | scroll 未与 paginated 组合 | 「scroll 触发须与 paginated 组合使用（@load name={fn} paginated scroll）」 |
| V9 | 语义硬错误 | debounce/throttle ms 非正整数或 fn 非可调用表达式 | 「std/timing `{fn}` 的 ms 参数须为正整数毫秒；fn 须为可调用函数」 |
| V10 | 警告 | scroll 容器不可滚动（无滚动能力/非列表容器） | 「无限滚动 @load 所在容器可能不可滚动——滚动触发可能永不生效，请确认容器有滚动能力」 |
| V11 | 警告 | 分页装载函数签名无法静态确认（第 1 参数 page: i32 / 返回 Page<T>） | 「分页装载函数签名建议为 `fn {name}(page: i32) -> Page<T>`——当前无法静态确认，运行时以约定信封兜底（诚实降级）」 |
| V12 | 运行时 | 分页响应信封缺 hasMore 字段 | 「分页数据缺少 hasMore 字段——按 hasMore=false 处理（停止加载更多）」（进入 error 态并附此消息，绝不静默） |

**半实现信号注记**（基线 §8）：form.submitting 恒 false、Flutter form.valid 恒 true 不属本批，维持 v0 桩注记，语言面不动；@load v0 边界（仅 Array/单例）由本批次补齐后，域 9 登记更新（§5.1）。

---

## 4. 三目标映射表

| 语言项 | React | Flutter | OmniView |
|---|---|---|---|
| @load 多例/匿名保留名 | 每实例独立 `useState` 三态 + `useCallback` load + `useEffect` mount（v0 形态 × N 实例）；命名实例信号名 `{name}Loading` 等。地道性：受控组件 + hooks 官方惯用法 | 每实例独立 `FutureProvider` + `ref.watch`（v0 形态 × N）；命名信号 `{name}Async` 映射为 `{name}Async` + `.isLoading/.error/.asData`。地道性：Riverpod 3.x 官方三态 | 每实例独立 store 槽位（`{Comp}_{name}_loading`/`{Comp}_{name}_error`/`{Comp}_{name}_data`/`{Comp}_{name}_retry`/`{Comp}_{name}_reset`——v0 `store.rs` 槽位命名扩展；view 侧派生信号 camelCase `{name}Loading` 等） |
| @load 参数绑定 | loader 调用表达式入 useCallback deps（`[token, id]`）+ `useEffect` 依赖重载。地道性：依赖数组官方惯用法 | 参数信号 `ref.watch` 进 provider body——参数变化自动重跑。地道性：Riverpod 官方响应式数据流 | 参数信号 watch 进装载函数调用，变化触发重载 |
| @load Reset | `reset()` = setLoading(false) + setError("") + setData(null)（data 初始值 v0 `[]` → `null`——可空化，渲染处 `?? []` 兜底保持）；不自动重载。地道性：受控状态重置 | `ref.invalidate` 不等价 Reset——生成 `AsyncNotifier`（官方可写三态原语）`setState(AsyncData(null))`；仅使用 Reset/分页的组件升级 AsyncNotifier，其余维持 FutureProvider（渐进式，不扰动既有生成）。降级注记：无（AsyncNotifier 是官方原语，非降级） | store 槽位置空（`{name}_data` → null），视图 `data == null && !loading` 渲染空闲 |
| 分页（自生成） | 自生成页码状态机：`page/hasMore/loadingMore` 三个 useState + `loadMore` useCallback（page+1、追加、hasMore 更新）+ 按钮/滚动接线。地道性：手写分页 state 是 React 无库场景的真人写法 | AsyncNotifier 内 `state = AsyncData([...old, ...page.items])` + `hasMore/loadingMore` 字段；`loadMore` 方法。地道性：AsyncNotifier 承载分页状态是 Riverpod 社区标准模式（研究 01：社区以 AsyncNotifier 手写页码状态） | store 槽位 + 页码槽位（`{Comp}_{name}_page`/`{Comp}_{name}_has_more`/`{Comp}_{name}_loading_more`）+ `{Comp}_{name}_load_more` 回调（view 侧 camelCase 派生信号 `{name}Page`/`{name}HasMore`/`{name}LoadingMore`/`{name}LoadMore`） |
| 无限滚动 | 容器尾部哨兵元素 + `IntersectionObserver`（threshold 阈值，hasMore/loadingMore 双重防护）；卸载 disconnect。地道性：IntersectionObserver 是 web 平台标准（TanStack 官方 docs 亦指「无限滚动适用于观察器」） | `ScrollController` `position.pixels >= maxScrollExtent - threshold` 监听 + dispose 释放。地道性：Flutter 官方滚动模式（研究 01 ①） | IntersectionObserver 同 React |
| 轮询 | `setInterval` + `useEffect` cleanup `clearInterval`，每 tick 调 load。地道性：React 官方无轮询原语，setInterval+hook 是官方 docs 级标准模式 | `Timer.periodic` + `dispose` cancel + `ref.invalidate`。地道性：Flutter 无官方轮询原语，Timer 是 SDK 标准（研究 01 ①） | JS setInterval 同 React |
| debounce/throttle | 生成稳定引用包装：`useMemo(() => debounce(doSearch, 300), [doSearch])`（或 useRef 持 timer）。地道性：lodash 风格是 React 生态事实标准工具层 | 生成 `State` 字段 timer 包装闭包（`Timer` + cancel on 新触发），卸载 dispose。地道性：Dart Timer 是 SDK 唯一时间原语 | JS 同 React（setTimeout/clearTimeout 闭包） |
| Page\<T\> 信封 | 生成接口 `Page<T> { items: T[]; hasMore: boolean }`（TS interface）。地道性：与 useInfiniteQuery 返回形状同构（`{data, hasNextPage}` 数据形状可平滑演进） | 生成 Dart class `Page<T>`（final fields）。地道性：Dart 数据类惯例 | JS 对象字面量 |

**降级注记汇总**：Flutter Reset 需 AsyncNotifier（官方原语，非降级，但生成形态变化——渐进式，仅 Reset/分页组件）；v0 分页游标模式不支持（§6 边界）；轮询 × 分页组合不支持（§9 P3）。

---

## 5. 登记册扩展

### 5.1 semantic-universe 域 9 扩展（@load 深化）

**语义域定义更新**（域 9 现有定义段末追加）：

> 深化（2026-08-09 批次 4）：@load 支持多例（命名实例 `@load name={fn}`，信号派生 `{name}Loading/{name}Error/{name}Data/{name}Retry/{name}Reset`）、参数绑定（调用形式 `@load name={fn(arg)}`，参数变化自动重载）、Reset（清空回未装载态，data=null）；v0 边界（仅 Array/每组件至多一个/仅 mount 自动）补齐——装载函数可为普通 fn 或 async fn。分页/无限滚动/轮询扩展归域 13。

**语义集条目表**（域 9 现有 5 行追加）：

| 语义集条目 | 实现状态 | 诊断行为 |
|---|---|---|
| @load 命名实例（多例） | 部分（本批补齐） | 名称冲突/匿名重复 → 硬错误 V2/V6 |
| {name}Loading/{name}Error/{name}Data/{name}Retry/{name}Reset | 部分（本批补齐） | 派生名冲突 → 硬错误 V4 |
| reset（匿名） | 部分（本批补齐） | 与既有信号/函数冲突 → 硬错误 V2 |

**已实现白名单锚点账本**（追加锚点）：`@load` 多例/参数/Reset → `LoadIntentIR` 扩展（`ir/types.rs`）+ 组件收集 `ir/lower/component.rs`（V-F6 改写）；`{name}Reset` → React `component/mod.rs` / Flutter `AsyncNotifier` 形态 / OmniView `store.rs` 槽位——机制级条目走存在性断言（域 9 先例：「无词法级白名单常量——以编译器代码锚点存在性断言」）。

### 5.2 新域 13 数据深化登记（照域 9 格式）

**语义域定义**：

> 数据能力深化语义域——分页/无限滚动/轮询/时间节流。与域 9（@load 三态装载）同簇：装载触发与装载状态的扩展语义（分页页码状态机、滚动接近底部装载触发、定时重新装载、高频事件时间包装）。全集封闭按本表；v0 边界：分页仅页码模式（游标模式留边界）、轮询与分页不组合、无限滚动触发仅列表容器（scroll 参数）。

**语义集条目表**：

| 语义集条目 | 实现状态 | 诊断行为 |
|---|---|---|
| @load paginated（分页页码状态机） | 未支持（本批实现后更新） | 匿名分页 → 硬错误 V7；签名无法确认 → warning V11 |
| Page\<T\>（内建信封 `{items, hasMore}`） | 未支持 | 信封缺字段 → 运行时错误 V12（诚实降级） |
| {name}Page / {name}HasMore / {name}LoadingMore / {name}LoadMore | 未支持 | 派生名冲突 → 硬错误 V4 |
| @load scroll（无限滚动触发） | 未支持 | 未与 paginated 组合 → 硬错误 V8；容器不可滚动 → warning V10 |
| @load interval: ms（轮询） | 未支持 | 非正整数 → 硬错误 V5 |
| debounce / throttle（std/timing） | 未支持 | ms 非正整数 → 硬错误 V9 |

**已实现白名单锚点方案**：域 13 同域 9 先例——无词法级白名单常量，以编译器代码锚点存在性断言：`paginated/scroll/interval` 参数解析锚点 `parser/view/element.rs`（@load 指令参数面）；`Page<T>` 类型锚点类型检查器；`std/timing` 模块锚点 stdlib 注册表。verify 脚本扩展方案见 §5.3。

### 5.3 vocabulary-registry 新词汇（照四字段格式）

| 语义词汇 | 来源域 | 目标映射 | 说明 |
|---|---|---|---|
| debounce | std/timing（stdlib 语言函数） | setTimeout/clearTimeout 闭包（web）/ Dart Timer 闭包（Flutter） | 时间节流原语——非视图意图，stdlib 通道（async_sleep 先例） |
| throttle | std/timing（stdlib 语言函数） | 同上 | 同上 |
| paginated | 域 13 数据深化（@load 参数面） | 页码状态机生成 | @load 局部参数词——机制级条目（存在性断言） |
| scroll（@load 参数） | 域 13 数据深化 | IntersectionObserver / ScrollController 阈值 | 装载触发参数——与 @in-view/@onScroll 既有词汇区分（不同词汇面） |
| interval（@load 参数） | 域 13 数据深化 | setInterval / Timer.periodic | 冒号参数形态（motion over: 先例） |
| Page\<T\> | 域 13 数据深化（内建类型） | TS interface / Dart class | 分页信封——与 Array\<T\> 并列 |

**排除词汇注记**：`refetchInterval`（TanStack 参数名）/`refreshInterval`（SWR 参数名）/`infinite_scroll_pagination` 包名——平台/库形状，非语义词汇（禁止借词规则，registry §24 行既有规则）。既有 `retry`（域 9）维持。

### 5.4 verify 脚本锚点方案

- `verify-semantic-universe.py`：域 13 新增断言块（语义集条目 → 编译器锚点存在性断言：`parser/view/element.rs` @load 参数解析 + `std/timing` 注册表 + `Page<T>` 类型锚点）；域 9 扩展条目并入既有 data 断言块（v0 边界列更新）。
- `verify-vocabulary.py`：新词汇四字段条目断言（§5.3 表）。
- `verify-demo-coverage.py`：DataFetching 迁移后页面仍在 + 新 demo 页接线断言（§8）。
- 注入语料：855 元组扩展点（§7.6 清单）。

---

## 6. 语言边界与触发条件（本批次未进语言项）

| 能力项 | 处置 | 触发条件 |
|---|---|---|
| 乐观更新 | 🔜 远期候选，不立语法（@load 写路径 mutate 未语义化；v0 @load 只读） | 写路径语义化后随批次评估；映射方向：TanStack onMutate 快照回滚 / Flutter 手动回滚（编译器生成） |
| 缓存策略（staleTime/GC/持久化） | 🏞 配置面留生态（库配置非功能意图；失效意图 retry/Reset 已覆盖） | 无（不重评） |
| 分页库绑定（TanStack useInfiniteQuery / infinite_scroll_pagination） | 🏞 编译器职责·后续增强路径（v0 自生成定案 §2.2） | 自生成形态出现不可控复杂（如多列表联合分页）或用户明确要求引库时重评 |
| 游标分页（cursor 模式） | 🏞 语言边界（v0 仅页码模式） | 语料需求出现（游标状态机跨目标差异大，研究 01 ③） |
| 轮询 × 分页组合 | 🏞 v0 不支持（interval 仅普通装载） | §9 P3 用户拍板后放开 |
| 预取 prefetch | 🏞 编译器优化职责候选（研究 01 遗漏补充） | 语料需求出现 |
| 请求去重/竞态 | 🏞 编译器职责（继承目标端装载机制内建行为——Riverpod invalidate 丢弃旧结果先例） | 无（不立语法） |
| 滚动恢复 scroll restoration | 🏞 语言边界（研究 01 遗漏补充） | 语料需求出现 |
| 半实现信号（form.submitting 恒 false / Flutter form.valid 恒 true） | v0 桩注记维持，语言面不动（基线 §8） | 表单深化批次（远期） |

---

## 7. 门禁与验证（五层门禁）

### 7.1 tsc（React 目标）

react-test-app 编译通过：多例/参数/Reset 生成（命名信号 + data 可空化 + useMemo debounce 稳定引用）、分页自生成状态机、无限滚动 IntersectionObserver、轮询 setInterval cleanup——`npx tsc --noEmit` 绿。

### 7.2 dart analyze（Flutter 目标）

flutter-test-app 通过：AsyncNotifier 升级形态（Reset/分页组件）、FutureProvider 维持（普通实例）、ScrollController 阈值监听、Timer.periodic dispose、`Page<T>` Dart class——`dart analyze` 零警告。

### 7.3 Playwright（浏览器行为断言清单）

| 断言 | 行为 |
|---|---|
| 多例独立 | 双 @load 组件：一个实例 error、另一实例正常出数据（互不污染） |
| 参数重载 | 修改 url 输入框 → 自动重新装载（网络请求新 URL） |
| Reset | 点击重置 → 列表清空 + 空闲提示（data == null）；再点装载 → 数据重现 |
| 分页 | 点击「加载更多」→ 追加第 2 页；hasMore=false → 按钮隐藏 |
| 无限滚动 | 滚动列表到底 → 自动装载第 2 页（零点击）；卸载组件无观察器泄漏（console 断言） |
| 轮询 | interval 5s：等待 → 数据自动刷新（拦截网络请求计数） |
| debounce | 快速输入 3 次 → 300ms 后仅 1 次触发（请求计数 = 1） |
| 空闲态 | Reset 后 `data == null && !loading` 分支渲染正确 |

### 7.4 docs-gate

verify-docs.py + verify-semantic-universe.py（域 9 扩展 + 域 13 新登记 + v0 边界列更新）+ verify-vocabulary.py（5.3 新词汇）+ verify-demo-coverage.py（§8 demo 接线）全绿。

### 7.5 注入语料（855 元组扩展）

新增元组草案（随 writing-plans 定稿落位）：

- @load 命名实例 × 信号派生名（5 信号）——5 元组
- @load 参数绑定（引用/调用/信号参数/字面量参数）——4 元组
- @load Reset（匿名/命名）——2 元组
- @load paginated（页码状态机信号 4 名 + LoadMore）——5 元组
- @load paginated scroll（触发 + 容器约束）——2 元组
- @load interval（轮询 + cleanup）——2 元组
- std/timing debounce/throttle（签名 + 事件绑定）——4 元组
- 错误路径（V2/V6/V7/V8 各 1）——4 元组

---

## 8. demo 迁移清单（learn-omniview 用点）

### 8.1 DataFetching.omni —— 必须迁移（车票定案：@load 扩展后迁移）

逐项对照迁移表：

| 现有形态（DataFetching.omni） | 迁移后形态 |
|---|---|
| `signal status: String = "idle"` | 删除——三态信号（loading/error/data）替代 |
| `signal resultText: String = ""` | 删除——`{name}Data`（json_stringify 结果）替代 |
| `signal errorMsg: String = ""` | 删除——`{name}Error` 替代 |
| `signal url: String = "https://…/1"` | 保留 url 信号（输入框 bind 不变）——作 @load 参数绑定源 |
| `action loadData() async`（http_get + json_parse + try/catch） | 迁移为 `async fn loadDirect(url: String) -> Array<String>`（逻辑保留：http_get + json_parse + json_stringify；错误路径交给 @load error 信号——`@load direct={loadDirect(url)}`） |
| `action loadViaApi() async`（fetch_demo 跨文件导入） | 迁移为 `async fn loadViaApi(url: String) -> Array<String>`（fetch_demo + error_message 逻辑保留）——第二实例 `@load api={loadViaApi(url)}` |
| `fn reset()` | 删除——`directReset()` / `apiReset()` |
| @if 四态渲染（idle/loading/success/error） | loading → `@if directLoading`；error → `@if directError != ""`；success → data 渲染；idle（@else 提示语）→ `@if directData == null && !directLoading && directError == ""` |
| 三个按钮（Load/API/Reset） | 保留 Load/API/Reset 三个按钮：`@onPress={directRetry}` / `@onPress={apiRetry}` / `@onPress={directReset}`（或 reset 同时重置双实例——见 §9 P8 未决） |

**迁移纪律**（ADR-075 语义化即切换）：迁移后 DataFetching 不再出现手写 status 状态机写法；学习文档（16-forms-and-async-data.md 相应节）同步改写。@load 匿名语法本身不删除（v0 兼容保留——LoadDemo 不迁移）。

### 8.2 新增 demo

- `PaginationDemo.omni`：分页（「加载更多」按钮）+ 无限滚动（scroll 参数）——挂接 UiComponents 页（PageSwitcher 标签 + Playwright 断言：追加页/滚动触发/按钮隐藏）。
- `PollingDemo.omni`：轮询（interval）+ debounce 搜索输入框（std/timing 首用点）——挂接 UiComponents 页 + Playwright 断言（自动刷新/防抖计数）。
- LoadDemo.omni 维持不变（匿名实例 v0 兼容的活证据）。

---

## 9. 未决问题（需用户/主 agent 拍板，spec 不擅自裁决）

- **P1 分页信封契约**：v0 采用约定信封 `Page<T> = { items, hasMore }`（编译器自生成的前提契约）。服务端信封形状是后端职责（研究 01 ③），约定信封把形状约束压给用户 loader——是否接受，还是需要信封字段可配置（配置面）？
- **P2 页码 vs 游标**：v0 仅页码模式（page: i32）。游标模式（nextCursor）是 TanStack 官方同权重形态——语料零需求故后置，确认后置决策。
- **P3 轮询 × 分页组合**：v0 不支持 interval + paginated 同实例（组合状态机复杂度高）。确认后置。
- **P4 Flutter Reset 的 AsyncNotifier 升级**：Reset/分页组件的 Flutter 生成从 `FutureProvider` 升级为 `AsyncNotifier`（官方可写三态原语，非降级）——生成形态变化是否接受（渐进式，仅受影响组件）？
- **P5 data 信号可空化**：`data = null` 表达「未装载/已 Reset」（React 初始值 []→null 形态变化）。确认接受——此设计使「成功但空数组」与「空闲」可区分（DataFetching idle 迁移依赖）。
- **P6 debounce/throttle 配置面**：v0 默认 trailing、不暴露 leading/trailing/立即执行配置。确认后置。
- **P7 参数自动重载**：参数变化 → 自动重新装载（真人写法默认）。是否需要「暂停自动重载」能力（v0 不做）？
- **P8 DataFetching 双实例 Reset**：`directReset()`/`apiReset()` 双重置回调 vs 单 Reset 同重置双实例——迁移后按哪个语义（建议：双实例各自 Reset，与多例独立性一致）？

---

## 附：完成时自查

- 占位符：无 TBD/TODO 残留。
- 节间矛盾：§2 语法形态 ↔ §3 Validation ↔ §4 映射 ↔ §5 登记一致（V 编号与 §5.2 诊断行为列互引）。
- 与基线矛盾：无推翻；分页自生成定案是车票 09 明示「批次 spec 定」的委派裁决，非推翻；其余决策点全部入 §9 未决。
- 命名冲突表：§2 各节冲突检查 + §5.3 排除词汇注记。
