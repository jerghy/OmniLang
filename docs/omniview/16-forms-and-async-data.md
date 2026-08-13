# 16 — 表单与异步数据

本页覆盖两个**功能设计级语义域**（spec 2026-08-08 §4.1 实施收口）：**表单语义（`@form`）**与**数据三态（`@load`）**。编译器理解「这是一个表单」「这是一个异步数据源」的意图，并针对编译目标（React / Flutter 生产级，OmniView 调试级）生成地道生产代码——源码表达意图，选路是 codegen 职责。

> **语义域登记**：字段校验规则（`validate:` 值域闭集 6 名）已登记 vocabulary-registry 域 6；`formErrors`/`formValid`/`formSubmitting`（`@form`）与 `loading`/`error`/`data`/`retry`/`reset`（`@load`）是**编译器保留名判定**（保留名冲突 = 硬错误），随机制自动生成、用户不可自定义，不属于语义词汇。

## 表单语义（@form）

意图：「这是一个表单——字段有校验规则、提交时收集值并执行提交、错误显示在字段旁」。

```omni
component App {
  signal name: string = ""
  fn submit(e) {}
  view {
    <form @form @onSubmit={submit}>
      <input bind:value={name} validate:{required: true} />
      @if form.errors.name {
        <span id="err-name">{form.errors.name}</span>
      }
      <button disabled={form.submitting}>提交</button>
    </form>
  }
}
```

编译器生成：字段值信号（`bind:value` 接线）、`form.errors` 错误信号 map、提交时校验执行、a11y（aria-invalid/aria-describedby）、`form.submitting` 禁用提交态信号。

### 字段校验（validate: 规则）

`validate:` 是**字段级属性**，值域为 6 名封闭规则集（`VALIDATE_RULES`——编译器白名单，登记册域 6 双向断言）：

| 规则 | 值域 / 约束 | 内建错误消息 |
|------|------------|-------------|
| required | 无参（`true`） | 此项必填 |
| format | 闭集名：`"email"` / `"url"` | 邮箱格式不正确 / URL 格式不正确 |
| min | 数字 | 长度至少为 N |
| max | 数字 | 长度最多为 N |
| pattern | 正则字符串 | 格式不正确 |
| custom | 组件 fn 名（v0 边界：无字面量自定义消息） | fn 返回消息 |

约束（违规 = 编译期硬错误）：

- **`validate:` 必须与 `bind:value`/`bind:checked` 配对使用**（孤儿规则无从校验）。
- **规则名 ∈ 闭集**：`未知校验规则 "@x"——合法规则: required/format/min/max/pattern/custom`。
- **规则值类型合法**（`validate:{format: "email"}`；闭集外值如 `"tel"` → 硬错误，需 custom）。
- `custom` 规则值为**组件 fn 名**（存在 + 单参 + 返回 String）。

> **行为注记（trim）**：validate 规则作用于字段**原始值**（不做 trim）——`validate:{min: 3}` 对 `"ab "`（含尾空格）按长度 3 判定**通过**；迁移前的 `string_trim` 手写校验会拒绝。需要 trim 语义时在 custom 规则内自行处理（如先 `string_trim` 再判空）。

### 错误显式渲染与禁用提交态

- **错误信号显式渲染**：`form.errors` + `@if form.errors.name` 由用户显式渲染（React / OmniView）。
- **`err-{field}` id 约定（行为注记）**：React / OmniView 端生成的 `aria-describedby` 引用 `err-{field}`——用户显式渲染错误容器时需写 `id="err-{field}"` 与之匹配（例：`<span id="err-name">{form.errors.name}</span>`）；Flutter 端不适用（错误经 `errorText` 原生展示）。
- **Flutter 例外（行为差异注记）**：错误经 `TextFormField.errorText` **自动展示**，语言面 `form.errors` 发射为**常量空 map**——语言面跨目标一致保留，行为差异注记于 semantic-universe 域 8。
- **禁用提交态 v0 形态**：编译器生成 `form.submitting` 信号（v0 恒 false），按钮禁用由用户显式写 `disabled={form.submitting}`——**v0 不做自动按钮接线**。

### 提交（@onSubmit）与保留名

- `@form` **必须配 `@onSubmit`**（`@form 元素必须绑定 @onSubmit`）；每组件至多一个 `@form`。
- `@onSubmit` 事件数据 schema 仅 `{values}`（FormData 收集，`{string: string}`）；旧 DOM 形状字段（`e.value` 等）→ 无效字段硬错误。
- **保留名冲突 = 硬错误**：`formErrors` / `formValid` / `formSubmitting` 与用户信号声明冲突 → `信号名 "@x" 与 @form 生成的保留名冲突`。`form` 对象（`form.errors`/`form.valid`/`form.submitting`）由编译器生成，用户不可声明同名信号。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 受控组件 + `validateForm` 执行器（校验规则链 → `setFormErrors` map，`formValid = Object.keys(formErrors).length === 0`） |
| Flutter | `Form` + `TextFormField.validator` 链（`_formKey`；错误经 `errorText` 自动展示，`form.errors` 常量空 map） |
| OmniView | HTML5 校验属性（`required`/`minLength`/`maxLength`/`pattern`…）+ `{Comp}_validateForm` 自绘错误 |

## 数据三态（@load）

意图：「这个数据源是异步的——加载/错误/数据三态由编译器管理」。batch-4 深化：命名实例（多例）、参数绑定、Reset、分页、无限滚动、轮询、std/timing 防抖节流（语义登记见 `docs/superpowers/specs/semantic-universe.md` 域 9 / 域 13）。

### 匿名实例（v0 兼容形态）

```omni
component App {
  fn fetchTodos() -> Array<String> {}
  view {
    @load={fetchTodos}
    @if loading {
      <div>loading</div>
    }
    @if error != "" {
      <div>{error}</div>
    }
    @for item in data {
      <div>{item}</div>
    }
  }
}
```

编译器生成 `loading`（Bool）/ `error`（String）/ `data`（Array）三态信号 + `retry` 重试回调（绑定到用户交互，如 `@onPress={retry}`）+ `reset` 重置回调（data 清回 null，不自动重载）。

匿名形态约束（v0 兼容）：

- **视图级指令**：`@load={fn}` 绑定组件 fn，mount 后自动装载；每组件至多一个**匿名** `@load`（V6 硬错误：`组件内至多一个匿名 @load`——当前匿名 N 个；多数据源请用命名实例）。
- fn 必须**存在且无参数**（`@load 引用的函数 "@x" 不存在` / `@load 引用的函数 "@x" 必须无参数`；参数绑定请用命名实例 call-form）。
- **v0 边界：仅 Array 返回**（React 端 `data` 初值 `null`，渲染 `?? []` 兜底；OmniView JS 槽位初值 `[]`）；struct/其他返回 → 硬错误 `@load 仅支持 Array 返回（v0）`。
- **保留名冲突 = 硬错误**：`loading` / `error` / `data` / `retry` / `reset` 与用户信号声明冲突 → `信号名 "@x" 与 @load 生成的保留名冲突`（计算属性同理）。
- **增量封装，不替代既有原语**：`@load` 是既有 `async action + await` 之上的三态封装增量——已有 async/await 代码零改动。

### 命名实例（多例）

`@load name={fn}` 将三态信号以 `{name}` 前缀派生——同一组件内多数据源互不干扰（LoadDemo 匿名单例 v0 活证据 + DataFetching 双例先例）：

| 派生信号 | 语义 |
|---------|------|
| `{name}Loading` | 装载中（初始 true） |
| `{name}Error` | 错误消息（初始 `""`） |
| `{name}Data` | 数据（初始 `null`，`@else if {name}Data == null` 判空态） |
| `{name}Retry` | 重试（重置三态 → 重跑 loader） |
| `{name}Reset` | Reset（data → null，**不**自动重载） |

```omni
component App {
  fn fetchTodos() -> Array<String> {}
  view {
    @load todos={fetchTodos}
    @if todosLoading {
      <div>Loading...</div>
    }
    @else if todosError != "" {
      <div>Error: {todosError}</div>
    }
    @else if todosData == null {
      <div>Idle — nothing loaded yet</div>
    }
    @else {
      @for item in todosData {
        <div>{item}</div>
      }
      <button @onPress={todosRetry}>Retry</button>
      <button @onPress={todosReset}>Reset</button>
    }
  }
}
```

### 参数绑定（call-form）

`@load name={fn(arg)}`——实参即**绑定参数**：参数信号变化自动重新装载（编辑 url → 重载）；loader 形参与调用实参一一对应。

```omni
component App {
  signal url: String = "https://api.example.com/todos"
  fn fetchTodos(url: String) -> Array<String> {}
  view {
    @load todos={fetchTodos(url)}
    @if todosLoading {
      <div>Loading...</div>
    }
    @else if todosError != "" {
      <div>Error: {todosError}</div>
    }
    @else if todosData == null {
      <div>Idle — edit the URL, then load</div>
    }
    @else {
      @for item in todosData {
        <div>{item}</div>
      }
      <input bind:value={url}></input>
      <button @onPress={todosRetry}>Load</button>
    }
  }
}
```

### Reset 语义

`{name}Reset` 将数据清回**未装载态**（`data = null`）且**不触发自动重载**——与 `{name}Retry` 不同（retry 重置三态后立即重跑 loader）。语义化即切换（ADR-075 原则 8）：手写 reset action + 手写状态复位一律改由 Reset 表达（DataFetching 迁移先例：旧 `status/resultText/errorMsg` 手写状态机已删除）。

### 分页（paginated）

`@load name={fn} paginated` 声明**分页页码状态机**：loader 签名 `fn name(page: i32) -> Page<T>`（`Page<T>` 内建信封 `{ items, hasMore }`），codegen 管理页码/加载态——`Load More` 按钮与无限滚动共用同一状态机：

- **必须命名实例**（V7 硬错误：`分页 @load 必须命名（@load name={fn} paginated）——分页信号 page/hasMore/loadingMore 需要实例归属`）。
- 派生信号：`{name}Page`（当前页，初始 1）/ `{name}HasMore`（初始 true）/ `{name}LoadingMore` / `{name}LoadMore`（加载下一页回调）+ 基础五件套（`{name}Loading/Error/Data/Retry/Reset`）。
- **签名静态确认（V11 warning）**：`分页装载函数签名建议为 fn (page: i32) -> Page<T>`——当前无法静态确认，运行时以约定信封兜底（诚实降级）。签名恰为约定形态 → 静默。
- **运行时信封校验（V12）**：返回数据缺 `hasMore` 字段 → 错误态 + 控制台消息 `分页数据缺少 hasMore 字段——按 hasMore=false 处理（停止加载更多）`。
- 翻页语义：`{name}LoadMore` 追加数据（page+1）；`{name}Retry`/`{name}Reset` 均回到第 1 页。

```omni
component App {
  fn fetchHistory(page: i32) -> Page<String> {
    if page <= 1 {
      return Page { items: ["History 1", "History 2", "History 3"], hasMore: true }
    }
    return Page { items: ["History 4"], hasMore: false }
  }
  view {
    @load history={fetchHistory} paginated
    @if historyLoading {
      <p>Loading...</p>
    }
    @else if historyError != "" {
      <p>Error: {historyError}</p>
    }
    @else {
      @for item in historyData {
        <div>{item}</div>
      }
      <button @onPress={historyLoadMore} disabled={!historyHasMore}>Load More</button>
      <button @onPress={historyReset}>Reset</button>
    }
  }
}
```

### 无限滚动（scroll）

`@load name={fn} paginated scroll`：滚动容器接近底部时**哨兵自动加载下一页**（IntersectionObserver / 等价机制）。容器须可滚动（`style: { height: '300px', overflow: 'auto' }` 等）；scroll 必须与 paginated 组合（V8 硬错误：`scroll 触发须与 paginated 组合使用（@load name={fn} paginated scroll）`）；容器静态启发式判断无滚动能力 → V10 warning（`无限滚动 @load 所在容器可能不可滚动`——请确认容器有滚动能力，绝不静默）。

```omni
component App {
  fn fetchFeed(page: i32) -> Page<String> {
    if page <= 1 {
      return Page { items: ["Feed 1", "Feed 2", "Feed 3"], hasMore: true }
    }
    return Page { items: ["Feed 4", "Feed 5"], hasMore: false }
  }
  view {
    <div style: { height: '300px', overflow: 'auto' }>
      @load feed={fetchFeed} paginated scroll
      @if feedLoading {
        <p>Loading...</p>
      }
      @else if feedError != "" {
        <p>Error: {feedError}</p>
      }
      @else if feedData == null {
        <p>Scroll this box to load the feed</p>
      }
      @else {
        @for item in feedData {
          <div>{item}</div>
        }
        <p>
          @if feedLoadingMore {
            Loading more...
          }
          @else if !feedHasMore {
            End of feed
          }
          @else {
            Loaded — scroll down for more
          }
        </p>
      }
    </div>
  }
}
```

### 轮询（interval）

`@load name={fn} interval: 5000`：每 5 秒自动重新装载（组件卸载自动清理定时器）。ms 必须为正整数（V5 硬错误：`interval 参数须为正整数毫秒`——当前：N）；**v0 边界：轮询 × 分页不组合**（P3 决策，触发条件 = 真实语料，记录于 semantic-universe 域 13 留生态）。

```omni
component App {
  fn fetchPrices() -> Array<String> {}
  view {
    @load prices={fetchPrices} interval: 5000
    @if pricesLoading {
      <p>Loading prices...</p>
    }
    @else if pricesError != "" {
      <p>Error: {pricesError}</p>
    }
    @else if pricesData == null {
      <p>Waiting for first poll...</p>
    }
    @else {
      @for item in pricesData {
        <pre>{item}</pre>
      }
    }
  }
}
```

### 防抖 / 节流（std/timing）

`debounce` / `throttle` 为纯语义时间函数（模块登记见 12-standard-library.md），时间语义跨端一致，表达式形态可直接作事件处理器：

```omni
import { debounce, throttle } from "std/timing";

component App {
  signal searchQuery: String = ""
  fn doSearch() -> String {}
  view {
    <input bind:value={searchQuery} @onInput={debounce(doSearch, 300)}></input>
    <button @onPress={throttle(doSearch, 2000)}>Refresh</button>
  }
}
```

- `debounce(fn, ms)`：停顿 ms 后执行一次（连续输入只触发最后一次）。
- `throttle(fn, ms)`：窗口内至多执行一次（trailing-only——窗口内以最新参数触发一次）。
- ms 须为正整数、fn 须可调用（V9 硬错误：`std/timing "fn" 的 ms 参数须为正整数毫秒；fn 须为可调用函数`——消息内 fn 为实际函数名）。
- **跨目标注记**：JS 族（React / OmniView）任意事件形态完整生效；Flutter 端完整拦截仅限 void 回调族（`@onPress`/`@onTap` 按钮系），`@onInput` 为透传占位（诚实降级，详见 12-standard-library.md）。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `useEffect` + 三态 state（`data` 初值 `null`，渲染 `?? []` 兜底）；命名实例 `{name}` 前缀 state；call-form 参数信号变化触发重载；分页页码 state + `Load More` 追加；scroll 哨兵 `IntersectionObserver`；`interval` → `setInterval`/`clearInterval`；debounce/throttle → `useMemo` 稳定包装（`(_fn.current ?? _fn)` 调用形态） |
| Flutter | `FutureProvider`（Riverpod 3.0 既有依赖；`asData?.value ?? const []`）；retry → `ref.invalidate`；分页 `AsyncNotifier` 页码状态机；轮询 `Timer.periodic`；debounce/throttle 拦截 void 回调族（onPressed 等），`@onInput` 透传占位 |
| OmniView | 自绘三态；JS 槽位 `{Comp}_{name}_loading/error/data/retry/reset`（view 侧 camelCase 派生信号）；分页 `{Comp}_{name}_page/has_more/loading_more/load_more`；轮询 `setInterval`；防抖/节流 `setTimeout` 包装 |

## 语言边界（v0 defer）

- **字段类型检查（V6）**：`value` 类型与控件匹配 → 仅警告级，v0 未实现。
- **`@load` 非 Array 返回**：v0 硬错误（见上），struct 支持属后续版本。
- **多表单 / 跨组件表单共享**：v0 不支持（每组件至多一个 `@form`）。
- **FormData 预填**：v0 不支持。
- **`form.submitting` 异步语义**：v0 恒 false（Flutter 端同步行为注记：常量发射）。
- **`custom` 自定义消息**：v0 仅组件 fn 名，无字面量自定义消息。
- **游标分页 / 滚动位置恢复 / 请求去重与竞态 / 乐观更新 / 缓存策略 / 预取 / 分页库绑定**：v0 defer——留生态登记（触发条件见 semantic-universe 域 13）。
- **轮询 × 分页组合**：v0 边界不组合（见上「轮询」）。
