# 18 — 导航与路由

本页覆盖**功能设计级语义域 11「导航」**（spec 2026-08-09-semantic-batch2-navigation-routing-design §2-§4 实施收口）：**页面声明（`@route`）**、**嵌套布局（`@layout`）**、**查询参数（`@query`）**、**页面跳转（`navigate`）**与**返回（`back`）**。页面导航意图只有一个表达面——源码只声明「去哪」与「带什么」，选路（React Router / GoRouter / hash 路由）与平台差异消化是 codegen 职责。

> **语义域登记**：八个语义集条目已登记 semantic-universe 域 11（`@route`/路径参数注入/`@query`/guard/`@layout`/`navigate`/`back`/`lazy`，全部「支持」）；`navigate`/`back` 是**内建语义函数保留名**（用户不得以同名声明 fn/store/信号/组件成员——V13 硬错误）；页面组件（`@route`）与布局组件（`@layout`）**互斥**（V3）。全工程 `@route` 注册表为编译器 import 图遍历（跨文件收集，支撑 V14/W2）。

## 页面声明（@route）

意图：「`@route "/path"` 声明组件为路由页面——路径 + 可选参数段、守卫、重定向、懒加载，平台路由表由编译器生成」。

```omni
component TodoDetail {
  fn isOwner() -> Bool { return true; }
  @route "/todos/:id" guard: isOwner lazy: true redirect: "/login";
  view { <div>{id}</div> }
}
```

语法：`@route "<path>"` + 可选属性 `guard: <fn>`（`() -> Bool` 签名守卫）、`redirect: "<path>"`（守卫失败跳转目标，**必须与 guard 配对**）、`lazy: <bool>`（按需加载）。属性值 `lazy` 只接受布尔字面量。

约束（违规 = 编译期硬错误，除非注明警告）：

- **组件顶级（V1，解析期）**：`@route` 只能出现在组件顶层（与 `@layout`/`@query` 同层），文件顶层与 view 内均为硬错误。
- **路径形态（V2，解析期）**：路径必须以 `/` 开头且非空——`路由路径必须以 \`/\` 开头且非空：'{path}'`。
- **与 @layout 互斥（V3）**：同组件同时声明 `@route` 与 `@layout` → 互斥硬错误。
- **重复声明（V4）**：同组件两条 `@route` → 重复声明错误，两条路径都点名。
- **参数段（V5/V6）**：参数段 `:name` 必须以字母/下划线开头（`路径参数段 ':1x' 非法`）；同名段重复 → `参数段 \`:id\` 重复`。
- **guard（V9）**：守卫必须是已声明的 `() -> Bool` 函数——`未找到或签名不是`。
- **redirect 配对（V11）**：`redirect:` 无 `guard:` → `必须与 \`guard:\` 配对`。
- **lazy（解析期）**：非布尔值 → `lazy 属性必须是 true/false`；未知属性 → `未知 @route 属性`。
- **跨组件唯一（V14）**：同一路径被多个组件声明 → 跨文件注册表硬错误（点名路径 + 先声明组件）。

## 路径参数注入

意图：「`/todos/:id` 的 `:id` 段注入为组件 String prop——页面内直接引用，URL 即数据源」。

```omni
component TodoDetail {
  @route "/todos/:id";
  view { <div>{id}</div> }        // :id 段注入为 String prop
}
```

约束（违规 = 编译期硬错误）：

- **注入名冲突（V7）**：注入参数名与显式声明的 prop/signal/event/fn 同名且**类型不兼容**（非 String）→ `类型不兼容`；同名 **String prop 兼容放行**（显式声明可补充默认值/校验语义）。

## 查询参数（@query）

意图：「`@query filter: String = "all"` 声明查询参数注入 prop——`?filter=all` 这类 URL 查询由编译器解析注入」。

```omni
component TodoList {
  @route "/todos";
  @query filter: String = "all";
  view { <div>{filter}</div> }
}
```

约束（违规 = 编译期硬错误）：

- **与路径参数同名（V8）**：`@query` 名撞上路径参数段 → `同名冲突`。

## 嵌套布局（@layout）

意图：「`@layout "/admin"` 声明组件为布局页面——子页面渲染进 `<@slot/>` 出口，三端映射嵌套路由（ShellRoute/嵌套路由表）」。

```omni
component AdminLayout {
  @layout "/admin";
  view { @slot default }
}
```

约束（违规 = 编译期硬错误）：

- **与 @route 互斥（V3）**：布局组件不能再声明 `@route`。
- **槽出口（V10）**：布局视图必须包含 `<@slot/>`（默认槽）——`未包含 \`<@slot/>\``。

## 页面跳转（navigate）

意图：「`navigate(path, { params, query })` 是全局可调用的内建语义函数——应用内跳转，三端映射 `useNavigate` / `context.push` / `location.hash`」。

```omni
component App {
  fn goDetail() { navigate("/todos/:id", { params: { id: "42" } }) }   // 参数填路径段
  fn goList()   { navigate("/todos", { query: { filter: "all" } }) }   // 查询参数
  view {
    <button @onPress={goDetail}>详情</button>
    <button @onPress={goList}>列表</button>
  }
}
```

约束（违规 = 编译期硬错误，除非注明警告）：

- **首参字面量 + 应用内路径（V12）**：`navigate("https://x.com")` → `只接受应用内路径`（提示改 `<a href>` 表达外部链接）；非 `/` 开头的字面量（含空串）同判。
- **动态首参（W1，警告）**：`navigate(page)` → `动态表达式` 警告（不阻断编译）——目标端仍跳转，只是失去注册表校验。
- **未注册路径（W2，警告）**：字面量路径不在全工程 `@route` 注册表内 → 警告（目标端空路由兜底，不硬阻断）。
- **保留名（V13）**：`navigate` 是内建语义函数名——组件成员同名（fn/信号/store）→ 保留名硬错误。

## 返回（back）

意图：「`back()` 零参内建函数——返回上一页，三端映射 `useNavigate(-1)` / `context.pop()` / `history.back()`」。

```omni
component TodoDetail {
  @route "/todos/:id";
  fn goBack() { back() }
  view { <button @onPress={goBack}>返回</button> }
}
```

约束（违规 = 编译期硬错误）：

- **保留名（V13）**：`back` 与 `navigate` 同为保留名——成员同名硬错误（`signal back: String` → `保留名`）。

## 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | **react-router v7 data router**：`createBrowserRouter` + `RouterProvider`；守卫 → loader 重定向；懒加载 → 路由级 `lazy` + `Suspense`；`navigate` → `useNavigate()`（调用点生成）；`back` → `navigate(-1)` |
| Flutter | **GoRouter**：`GoRoute` 表 + `redirect`（守卫）+ `ShellRoute`（`@layout`）；`navigate` → `context.push(path)`（栈语义，可 back）；`back` → `context.pop()`（**栈空 no-op** 注记） |
| OmniView | **hash 路由**：`__omni_routes` 嵌套渲染 + `hashchange` 监听；深链可直达可分享；动态 import 懒加载 |

## Floor 差异注记（诚实降级，ADR-054）

- **深链/URL 同步（C4）**：React URL 是天然事实源，web 端对等；**Flutter 移动端需平台侧清单配置**（App Links/Universal Links——AndroidManifest/Info.plist 是平台专属表达通道），编译器只出**编译注记、不生成清单**；OmniView hash 可直达可分享。
- **懒加载（C8）**：React 路由级 `lazy` + Suspense；Flutter **web 端 deferred、移动端单 bundle 无运行时按需加载概念 → no-op + 编译注记（绝不静默）**；OmniView 动态 import。
- **栈空 back（C3）**：Flutter `context.pop()` 在栈空时 **no-op**（与 web 历史栈行为差异，编译注记）；web 两端回退到历史栈根。

## Tab 与路由边界（§6 边界 6）

**tab 面板 ≠ 路由页面**。`@if` 互斥切页与 `event onNavigate` 通知是**合法语法**（`@if` 与组件事件本身不可硬诊断），编译器不做「旧形态」硬诊断；收口 = **文档引导**：页面切换意图写 `@route`/`navigate`，tab 的 UI 态（高亮/保留）留在 tab 组件内，不自动路由化。`event onNavigate: String` 作为合法组件事件名继续可用（与 navigate 不冲突），但新写代码应迁移为 `navigate()` 表达页面切换意图。示例：

```omni
component Tabs {
  signal active: String = "home"   // tab UI 态——不是路由
  view {
    <div @onPress={() => { active = "home" }}>首页</div>
    <div @onPress={() => { active = "about" }}>关于</div>
    @if active == "home" { <HomePanel/> }
    @if active == "about" { <AboutPanel/> }
  }
}
component HomePanel { @route "/home"; view { <div>Home</div> } }
```

## 语言边界（v0 defer）

- **通配符路径 / 404 fallback**：React Router `*`、GoRouter `notFound` 为目标端机制，不进语言；触发条件 = 真实语料。
- **动态守卫目标 / 滚动恢复 / 预取**：v0 定界不进语言（语义集全集 8 条目封闭）。
- **守卫失败默认重定向**：`guard:` 无 `redirect:` 时的默认目标待拍板（草案：默认 `/`；可选：强制必填显式）。
- **`navigate` 动态首参的类型化目标**：W1 警告级放行（动态表达式目标端仍跳转），后续语料驱动收紧。
