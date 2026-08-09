# 批次 2 域 C 导航路由 设计 spec（2026-08-09-semantic-batch2-navigation-routing-design.md）

**日期**: 2026-08-09
**状态**: 设计定稿（详细级）
**上游**: `2026-08-09-semantic-capability-baseline-design.md`（§4 批次 2 设计方向 + §3.1 真人写法标准 + §8 治理）；`.scratch/semantic-capability-baseline/issues/07-stage-scope-navigation-routing.md`（域裁决定案，全量进 floor）；`.scratch/semantic-capability-baseline/research/01-data-and-routing.md`（路由 7 项全景）；`.scratch/semantic-capability-baseline/research/04-design-direction-audit.md`（审计细化点 2）
**起点**: `2026-08-08-semantic-design-coverage-design.md` §4.3 域 C 契约草案
**配套计划**: 待 writing-plans 生成（批次 2 实施计划，本 spec 为唯一详细级输入）

---

## 1. 背景与范围

本批次将「导航路由」能力域的 8 项裁决进语言（车票 07 全量进 floor，无一排除）。语言面表达页面导航意图（声明 + 调用），编译器为三目标（React/Flutter/OmniView）选路地道映射（React Router / GoRouter / hash 路由），平台形态差异以诚实降级注记呈现（ADR-054）。

| # | 能力项 | 上游裁决 | 设计方向（基线 §4 批次 2） |
|---|---|---|---|
| C1 | 页面切换 | 🔜 进语言（域 C） | `@route "/todos/:id"` 页面组件声明 |
| C2 | 路由参数 | 🔜 进语言 | `:id` 路径段跨生态同构 → 参数 prop 注入 |
| C3 | 返回/前进 | 🔜 进语言 | `back()` 内建语义函数 |
| C4 | 深链/URL 同步 | 🔜 进 floor | React URL / Flutter App Links+web URL / OmniView hash；形态差异诚实降级注记 |
| C5 | 守卫/权限 | 🔜 域 C v0 含守卫 | `guard` 属性 → React loader 重定向 / GoRouter redirect / OmniView 守卫 fn |
| C6 | 嵌套路由 | 🔜 独立语法（用户拍板） | **独立 layout 语法**（`@layout` 指令，非组合折衷） |
| C7 | URL 查询参数 | 🔜 进语言（随域 C） | `@query` 声明 → useSearchParams / uri.queryParameters / URLSearchParams |
| C8 | 懒加载 | 🔜 随域 C（编译器职责优先） | `lazy` 属性 → React.lazy+Suspense / Flutter web deferred（移动端 no-op 注记）/ OmniView 动态加载 |

导航调用侧：`navigate(path, {params})` / `back()` 内建语义函数（基线 §4 定形）。

**审计细化点 2 的决策权**（research 04 登记，本 spec 行使）：React Router 库绑定与依赖注入形态、Flutter GoRouter vs Navigator 取舍——见 §4。

**语料位**（research 01 实证）：`test/projects/learn-omniview/src/pages/Overview.omni`（`event onNavigate: String` + `tabName` prop 通知父级切页模式）、`src/components/PageSwitcherDemo.omni`（手写 `@if/@else` 组件级页面切换模拟——设计文档 §6 已列「PageSwitcherDemo → 导航原语」重构点）、`src/main.omni`（9 标签页 `@if` 链）。全部为迁移目标，见 §8。

---

## 2. 语义意图定义

### 2.1 页面切换 — `@route` 页面组件声明

**意图一句话**：组件声明自己是一个页面，挂在应用路由表中的指定路径；应用内跳转经 `navigate` 语义函数表达，编译器生成目标端路由接线。

**语法形态草案**：

```
component TodoDetailPage {
    @route "/todos/:id";
    ...
}
```

- `@route` 是**组件级指令**（与 `prop`/`signal`/`event`/`fn`/`@store` 同级，出现在组件顶层块），非视图元素指令
- 路径为字符串字面量，必须以 `/` 开头；段可为静态段或参数段 `:name`
- 属性扩展点（车票 07 与基线定形）：`guard: fnName`（C5）、`lazy: true`（C8）
- 参数段注入为**隐式 prop**（C2，见 2.2）——组件体内直接引用，无需显式声明

**示例**：

```
component TodoListPage {
    @route "/todos";
    signal items: [String] = [];

    view {
        <div>
            <h1>全部待办</h1>
            <ul>
                @for item in items {
                    <li @onPress={() => { navigate("/todos/:id", { params: { id: item } }) }}>{item}</li>
                }
            </ul>
        </div>
    }
}

component TodoDetailPage {
    @route "/todos/:id";
    // :id 自动注入为 String prop `id`，组件内直接可用
    view {
        <div>
            <h1>待办 #{id}</h1>
            <button @onPress={() => { back() }}>返回</button>
        </div>
    }
}
```

**BNF 产生式草案**：

```
route_decl    := '@route' STRING route_attr*
route_attr    := 'guard' ':' IDENT
               | 'lazy' ':' BOOL
```

**与既有语法的关系/冲突检查**：

| 既有语法 | 关系 |
|---|---|
| `@if/@for`（视图内指令） | 位置域不同（组件顶层 vs 视图元素），无冲突 |
| `@store`（组件级指令） | 同为组件顶层指令，可共存（页面组件可挂 store） |
| `@slot`（域 5 投影） | **复用**：作为 @layout 布局的子页面出口（2.6） |
| `prop` 声明 | 路径参数注入与显式 prop 同名时按 V7 校验 |
| `event onNavigate: String`（语料既有模式） | 合法组件事件名，不冲突；但页面切换意图应迁移为 `navigate()`（§8 文档引导，不做硬诊断——见 §6 边界 6） |

### 2.2 路由参数 — 路径参数 prop 注入

**意图一句话**：`:id` 路径段参数跨生态同构（React useParams / GoRouter pathParameters / OmniView 解析均同 `:segment` 模式），参数值注入为页面组件的 prop。

**语法形态草案**：`@route "/todos/:id"` 声明后，`:id` 段自动成为组件隐式 prop `id: String`——组件体内直接引用（视图表达式、fn 参数、信号初值均可）。无需显式声明；显式声明同名 prop 时类型必须为 String（V7）。

**BNF**（并入 route_decl）：路径字符串解析出 `path_param ::= ':' IDENT`。

**冲突检查**：注入 prop 名与组件显式 prop/signal/event/fn 同名 → V7 硬错误；参数段重名 → V6。

### 2.3 导航调用 — `navigate(path, {params})` / `back()` 内建语义函数

**意图一句话**：应用内页面跳转与返回，编译器选路目标端导航 API（React useNavigate / GoRouter push / OmniView hash 更新），「同意图不同实现」的教科书案例（research 01 项 12）。

**语法形态草案**：

```
navigate("/todos")                          // 静态路径，字面量透传
navigate("/todos/:id", { params: { id: item } })   // 模板路径 + 参数替换
navigate("/todos", { query: { filter: "all" } })   // 查询参数（C7）
back()                                       // 返回上一页
```

- 内建语义函数（与 `showToast`（批次 1）/stdlib 函数同一函数调用通道），任意组件体内可调（编译器在目标端注入导航上下文，如 React `useNavigate` 取自组件作用域）
- options 对象形态 `{ params, query }` 仿批次 1 `showToast(msg, {kind, duration})` 风格（冒号键值对）
- `navigate` 只接受应用内路径（`/` 开头，V12）；外部链接走既有 `<a href>` 语法
- 命名空间冲突：`navigate`/`back` 为**保留名**，组件不可声明同名 fn/event/prop/signal（V13）

**BNF**：

```
navigate_call := 'navigate' '(' expr (',' '{' navigate_opts '}')? ')'
navigate_opts := (('params'|'query') ':' object_expr) (',' ...)?
back_call     := 'back' '(' ')'
```

**冲突检查**：与 stdlib 函数（`array_find` 等工具函数）命名空间分离——内建语义函数 vs stdlib 工具函数，无重名；`back` 与任何既有词汇无冲突。

### 2.4 URL 查询参数 — `@query` 声明

**意图一句话**：URL 查询参数（`?filter=all`）显式声明为页面 prop，编译器按目标端读取（useSearchParams / uri.queryParameters / URLSearchParams）。

**语法形态草案**：

```
component TodoListPage {
    @route "/todos";
    @query filter: String = "";
    // filter 注入为 String prop（默认 ""），视图直接引用
}
```

- `@query name: Type = default`，与 `prop` 声明同形态（类型 + 可选默认值），**仅声明了的查询参数才注入**（未声明的查询参数被目标端忽略，不报错）
- 读取侧：页面首次挂载与 URL 变化时注入 prop 值；写入侧经 `navigate(..., { query })` 更新 URL

**BNF**：

```
query_decl := '@query' IDENT ':' TYPE ('=' expr)?
```

**冲突检查**：`@query` 名与路径参数同名 → V8；与显式 prop/signal/event/fn 同名 → 同 V7 规则。

### 2.5 守卫/权限 — `guard` 属性 + 重定向目标

**意图一句话**：「未授权 → 重定向到登录页」——语言面为布尔条件 + 重定向目标声明，编译器消化机制差异（React loader 重定向 / GoRouter redirect / OmniView 守卫 fn）。

**语法形态草案**：

```
component AdminPage {
    @route "/admin" guard: isAdmin redirect: "/login";
    fn isAdmin() -> Bool { ... }   // 组件级 fn，签名 () -> Bool
}
```

- `guard:` 引用组件级 fn（v0 限同步 fn，签名 `() -> Bool`；signal/表达式守卫列入边界 3）
- `redirect:` 目标路径字面量（`/` 开头），与 `guard` 配对（V11）；缺省时默认重定向到 `/`（未决 #2 请拍板）
- 守卫通过 → 正常渲染；失败 → 重定向，不渲染页面

**冲突检查**：guard fn 引用不存在/签名非法 → V9；与嵌套（2.6）可组合（layout 本身也可带守卫，v0 支持 layout 级 guard）。

### 2.6 嵌套路由 — `@layout` 独立布局语法（用户拍板形态）

**意图一句话**：布局组件声明路径前缀，子页面渲染进布局的插槽——独立语法（用户 2026-08-09 拍板：layout 声明，非组合折衷），编译器映射 React Router 嵌套 Route + `<Outlet/>` / GoRouter ShellRoute / OmniView 嵌套渲染。

**语法形态草案**：

```
component AdminLayout {
    @layout "/admin";
    view {
        <div class: AdminShell>
            <header>管理后台</header>
            <@slot />          // 复用既有 @slot 投影：子页面渲染出口
        </div>
    }
}

component AdminUsers {
    @route "/admin/users";    // 子页面照常声明完整路径
}
```

- `@layout "/prefix"` 组件级指令：声明该组件为布局，前缀下所有子路由渲染进其 `<@slot/>`
- 一组件声明 `@route` 与 `@layout` 互斥（V3）
- 布局视图必须含 `<@slot/>`（V10——无出口则布局无意义）

**BNF**：

```
layout_decl := '@layout' STRING
```

**冲突检查**：`@layout` 为新词汇，与既有指令（含批次内 `@route`）无重名；`@slot` 复用域 5 既有语法（语义扩展：布局出口），无新词汇。

### 2.7 深链/URL 同步

**意图一句话**：URL 反映页面状态、可直达可分享。web 端 URL 是路由事实源（React/OmniView 天然）；Flutter 移动端以 GoRouter 路径模拟 + 平台级 App Links/Universal Links 桥接——**floor 内**（车票 07 用户修正），平台形态差异诚实降级注记。

**语言面形态**：无新语法——`@route` 路径声明即隐含 URL 语义（web 端地址栏、hash 或 history 形态由目标端决定）。编译器职责：web 端 URL 同步接线（React Router 天然 / OmniView hashchange 监听）；Flutter 端生成 GoRouter 配置（web 编译目标同步 URL；移动端出平台配置注记——编译器不生成 App Links 清单文件，见 §6 边界 4）。

### 2.8 懒加载 — `lazy` 属性

**意图一句话**：「页面按需加载」意图（车票 07：编译器职责优先——语言面仅留属性扩展点，实现面选路）。

**语法形态草案**：`@route "/heavy" lazy: true;`

- React：路由级 `lazy: () => import(...)` + Suspense（官方形态）
- Flutter web：deferred import；**移动端 no-op**（单 bundle 无运行时按需加载概念——research 01 项 14 实证）→ 编译注记（诚实降级，绝不静默）
- OmniView：动态 import 页面组件

**冲突检查**：`lazy` 为 route 属性词汇，无冲突。

---

## 3. Validation 清单

错误消息草案（中文；前缀区分解析错误/语义硬错误/警告）。

| # | 级别 | 触发条件 | 错误消息草案 |
|---|---|---|---|
| V1 | 解析错误 | `@route`/`@layout`/`@query` 出现在组件顶层块之外（如视图内、文件顶层、store 内） | 「`@route`/`@layout`/`@query` 只能声明在组件顶层块内（与 prop/event/fn 同级）」 |
| V2 | 解析错误 | `@route`/`@layout` 路径为空或非 `/` 开头 | 「路由路径必须以 `/` 开头且非空：'{}'」 |
| V3 | 语义硬错误 | 同一组件同时声明 `@route` 与 `@layout` | 「组件 '{}' 同时声明了页面（@route）与布局（@layout）——两者互斥，请拆分为两个组件」 |
| V4 | 语义硬错误 | 同一组件重复声明 `@route` | 「组件 '{}' 重复声明 @route（'{}' 与 '{}'）——一个组件只对应一个页面，多页面请拆分为多组件」 |
| V5 | 解析错误 | 路径参数段语法非法（`:123`、`::id`、`:id-2` 等） | 「路径参数段 '{}' 非法——参数名为字母/下划线开头，如 `:id`」 |
| V6 | 语义硬错误 | 路径参数段重名（`/a/:id/b/:id`） | 「路径 '{}' 参数段 `:id` 重复——参数名在一条路径内必须唯一」 |
| V7 | 语义硬错误 | 注入参数名与组件显式 prop/signal/event/fn 同名且类型不兼容 | 「路径参数 `:{}` 注入为 String prop，与组件 '{}' 已声明的 '{}' 冲突（类型不兼容）——请改名或删除显式声明」 |
| V8 | 语义硬错误 | `@query` 名与路径参数同名 | 「查询参数 '{}' 与路径参数 `:{}` 同名冲突——两者注入同名 prop」 |
| V9 | 语义硬错误 | `guard:` 引用不存在或签名非 `() -> Bool` | 「守卫 '{}' 未找到或签名不是 `() -> Bool`（v0 只支持组件级同步布尔 fn）」 |
| V10 | 语义硬错误 | `@layout` 组件视图不含 `<@slot/>` | 「布局组件 '{}' 的视图未包含 `<@slot/>`——子页面没有渲染出口」 |
| V11 | 语义硬错误 | `redirect:` 属性未与 `guard:` 配对 | 「`redirect:` 必须与 `guard:` 配对使用（单独使用无意义）」 |
| V12 | 语义硬错误 | `navigate()` 目标不是应用内路径（非 `/` 开头） | 「navigate 只接受应用内路径（`/` 开头）：'{}'——外部链接请用 `<a href>`」 |
| V13 | 语义硬错误 | 组件声明与保留名冲突（fn/event/prop/signal 名为 `navigate`/`back`） | 「'{}' 是内建语义函数保留名，组件 '{}' 不可声明同名成员」 |
| W1 | 警告 | `navigate()` 路径为非常量表达式（signal 拼接等） | 「navigate 路径 '{}' 为动态表达式，无法静态校验是否已注册——目标端运行时负责」 |
| W2 | 警告 | `navigate()` 字面量路径未匹配任何已声明 `@route`/`@layout`（全工程注册表校验） | 「路径 '{}' 未匹配到任何 @route/@layout 声明——跳转将落到空路由（目标端行为差异见注记）」 |
| V14 | 语义硬错误 | 全工程注册表发现两个组件声明相同 `@route` 路径，或 `@route` 路径与 `@layout` 前缀相同（主 agent 审查补录——2026-08-09） | 「路径 '{}' 已被组件 '{}' 声明——同一路径/前缀不得由多个组件声明（@route 与 @layout 前缀冲突检查）」 |

**语义化即切换（ADR-075 原则 8）**：本批次无「旧语法」需硬诊断——导航旧形态是**手写组合**（`@if` 页面互斥 + `event onNavigate` 通知），`@if` 与事件本身是合法语法，不可诊断；以文档引导 + demo 迁移收口（§6 边界 6、§8）。**无新旧双轨**：导航意图只有一个表达面（`@route`/`navigate`/`back`），编译器不生成「手写模拟」形态。

---

## 4. 三目标映射表 + floor 契约

### 4.1 审计细化点 2 决策（本 spec 行使决策权）

**React Router 库绑定与依赖注入**：绑定 **react-router v7**（v7 起主包统一为 `react-router`，声明式路由事实标准——research 01 项 10/04）。依赖注入机制沿 **Zustand 先例**（`omnilang/src/codegen/react/design.rs:556` 发射 `import { create } from 'zustand';`，`react-test-app/package.json` 已含 `zustand: ^5.0.14`）：**编译器发射 import 语句，依赖项由项目模板/CLI 维护**（包名 + 版本号写入模板 package.json；精确版本 pin 由实施计划定）。路由接线形态：**data router**（`createBrowserRouter` + `RouterProvider`——React Router v7 真人主流形态，且守卫（loader 重定向）与懒加载（路由级 `lazy`）是 data router 一等公民；实施期若 data router 迁移代价过大，降级为 `BrowserRouter` + 条件渲染 `<Navigate>`（诚实降级注记，语言面不变）。

**Flutter GoRouter vs Navigator 取舍**：定 **GoRouter**（flutter.dev 官方推荐，声明式 path ↔ `@route` 声明同构；pathParameters/redirect/ShellRoute/uri.queryParameters/web URL 同步全内建——research 01 项 10-16 实证）。Navigator 命令式 push/pop 作为内部实现细节（`back()` 映射），不作为语言面形态。保挂载场景（PageSwitcherDemo 的 IndexedStack 行为）映射 `StatefulShellRoute.indexedStack`（v0 以标准 ShellRoute 落地 + 保挂载差异降级注记，见 §6 边界 7）。

### 4.2 生成代码形态与地道性依据

**React（react-router v7 data router）**：

```tsx
// App.tsx（根，编译器生成路由表 + RouterProvider）
import { createBrowserRouter, RouterProvider, redirect } from 'react-router';
const router = createBrowserRouter([
  { path: '/', element: <HomePage /> },
  { path: '/todos', element: <TodoListPage /> },
  { path: '/todos/:id', element: <TodoDetailPage /> },
  { path: '/login', element: <LoginPage /> },
  { path: '/admin', element: <AdminPage />, loader: () => (isAuthorized() ? null : redirect('/login')) },
  { path: '/admin', element: <AdminLayout />, children: [{ path: 'users', element: <AdminUsers /> }] },
  { path: '/heavy', lazy: () => import('./heavyPage') },
]);
export default function App() { return <RouterProvider router={router} />; }
```

- navigate → 页面组件内 `const navigate = useNavigate();` + `navigate('/todos/42')`（模板路径 + params 由编译器拼接；query 拼入字符串）
- back → `navigate(-1)`
- 参数 → 路由表注入 `element={<TodoDetailPage id={params.id}/>}`（data router loader 内取 `params` 透传）——useParams 为组件内读取形态，编译器选 loader 注入（prop 注入语义的直接对应）
- 查询参数 → `useSearchParams` 读取注入 prop / navigate 时拼 URL
- 地道性依据：`createBrowserRouter`/`RouterProvider`/嵌套 `<Outlet/>`（@slot 出口）/路由级 `lazy`/loader 重定向守卫均为 React Router v7 官方惯用法；库绑定有 Zustand 先例

**Flutter（GoRouter）**：

```dart
final router = GoRouter(
  initialLocation: '/',
  routes: [
    GoRoute(path: '/', builder: (_, __) => const HomePage()),
    GoRoute(path: '/todos/:id', builder: (_, s) => TodoDetailPage(id: s.pathParameters['id']!)),
    GoRoute(path: '/admin',
      redirect: (_, __) => isAuthorized() ? null : '/login',
      builder: (_, __) => const AdminPage()),
    ShellRoute(builder: (_, __, child) => AdminLayout(child: child),
      routes: [GoRoute(path: '/admin/users', builder: (_, __) => const AdminUsers())]),
    GoRoute(path: '/heavy', builder: (_, __) => const HeavyPage()),  // 移动端 no-op 注记（web 端 deferred 策略见 §6 边界 5）
  ],
);
// MaterialApp.router(routerConfig: router)
```

- navigate → `context.push(path)`（栈语义，可 back）；back → `context.pop()`（栈空时 no-op——降级注记：与 web 历史栈行为差异）
- 参数 → `pathParameters` 注入 prop；查询 → `uri.queryParameters` 注入
- 地道性依据：GoRouter 官方推荐声明式路由；redirect/ShellRoute/pathParameters 全部官方内建；Riverpod 先例同规格（官方生态绑定）

**OmniView（hash 路由自绘）**：

```js
// 运行时路由：hashchange 监听 + 路由表匹配（前缀/参数段/查询参数解析）+ 守卫 fn + 嵌套渲染 + history.back()
// navigate() → location.hash = '#/todos/42'; back() → history.back()
```

- 地道性依据：单文件运行环境零依赖约束（runtime.rs「No external dependencies」注记），hash 路由是零依赖环境的事实形态（research 01 项 15：hash/history 两模式）
- 降级注记：URL 形态为 `#/path`（非 history pushState）——运行环境（iframe 沙箱/文件协议）限制，见 §6 边界 4

### 4.3 floor 契约表（意图 × 三目标行为对等/降级）

| 意图 | React（react-router v7） | Flutter（GoRouter） | OmniView（hash） | 对等性 |
|---|---|---|---|---|
| C1 页面切换 | createBrowserRouter + 路由表 | GoRoute 表 | hash 路由表 | ✅ 对等（URL 形态差异：history vs hash，运行环境差异注记） |
| C2 路径参数 | loader 取 params 注入 prop | pathParameters 注入 | 路由表解析注入 | ✅ 对等 |
| C3 返回 | navigate(-1) | context.pop() | history.back() | ✅ 对等（Flutter 栈空 back no-op 注记） |
| C4 深链/URL 同步 | URL 天然事实源 | web URL 同步；移动端 App Links/Universal Links 平台配置 | hash 可直达可分享 | ⚠️ 部分降级：Flutter 移动端需平台侧清单配置（编译器出注记，不生成清单）；web 端对等 |
| C5 守卫 | loader 重定向 | redirect 回调 | 守卫 fn | ✅ 对等（机制差异编译器消化） |
| C6 嵌套 | 嵌套 Route + Outlet | ShellRoute | 嵌套渲染 | ✅ 对等（Flutter ShellRoute 子栈语义注记；保挂载差异见 §6 边界 7） |
| C7 查询参数 | useSearchParams | uri.queryParameters | URLSearchParams | ✅ 对等 |
| C8 懒加载 | 路由级 lazy + Suspense | web deferred；**移动端 no-op** | 动态 import | ⚠️ 部分降级：Flutter 移动端无按需加载概念 → 编译注记（绝不静默） |

**floor 外仅保留机制语法面差异**（车票 07 修正）：URL 形态（history vs hash）与平台清单配置属目标专属表达通道，非功能缺失。

---

## 5. 登记册扩展

### 5.1 semantic-universe 新域 11 导航（登记表格草案，照域 8/9 格式）

> 登记先行纪律（ADR-074）：本表格随批次计划执行——**先登记 → 才进 BNF/白名单 → 才实施**。

**语义域定义**：域 11 导航——页面切换、路径/查询参数、返回、深链/URL 同步、守卫、嵌套布局、懒加载。语义集全集封闭（8 条目，v0 定界：动态守卫目标/通配符 404/滚动恢复/预取不进语言，见 §6）；保留名 `navigate`/`back` 硬错误（V13）；页面组件（`@route`）与布局组件（`@layout`）互斥（V3）。

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|---|---|---|---|
| @route 页面声明 | 组件声明为路由页面（路径 + 参数段） | 🔜 本批次 | V1-V6（位置/路径/重复/参数段语法） |
| 路径参数 prop 注入 | `:id` 段注入为 String prop | 🔜 本批次 | V7（同名类型冲突） |
| @query 查询参数 | 声明查询参数注入 prop | 🔜 本批次 | V8（与路径参数同名） |
| guard 守卫 | 布尔条件 + 重定向目标 | 🔜 本批次 | V9/V11（引用校验/配对） |
| @layout 嵌套布局 | 布局组件 + @slot 子页面出口 | 🔜 本批次 | V3/V10（互斥/无出口） |
| navigate 内建函数 | 应用内跳转（params/query） | 🔜 本批次 | V12/W1/W2（路径形态/静态校验） |
| back 内建函数 | 返回上一页 | 🔜 本批次 | V13（保留名） |
| lazy 懒加载 | 页面按需加载（编译器职责优先） | 🔜 本批次 | 无（属性扩展点） |

**已实现白名单（实施后回填编译器事实源锚点）**：

| 条目 | 编译器事实源（预期锚点） |
|---|---|
| @route/@layout/@query 解析 | `crates/omnilang-core/src/parser/top_level/route.rs`（新） |
| guard/lazy/redirect 校验 | `crates/omnilang-core/src/ir/lower/route.rs`（新）+ V1-V13 校验位 |
| navigate/back 语义函数 | `crates/omnilang-core/src/parser/expr/`（内建调用登记） |
| React 路由接线 | `omnilang/src/codegen/react/router.rs`（新：createBrowserRouter/loader/Suspense 发射） |
| Flutter GoRouter 接线 | `omnilang/src/codegen/flutter/router.rs`（新：GoRoute/redirect/ShellRoute 发射） |
| OmniView hash 路由 | `omnilang/src/codegen/omniview/router.rs`（新：hashchange/守卫/嵌套渲染） |
| 全工程 @route 注册表 | 编译器 import 图遍历（跨文件收集，支撑 W2） |

### 5.2 vocabulary-registry 新域 7 导航词汇（登记草案）

| 语义词汇 | 来源域 | 目标映射 | 排除词汇 |
|---|---|---|---|
| `@route` | 域 11 导航 | 页面路径声明 → React Router Route / GoRouter GoRoute / OmniView 路由表 | `path`/`route`/`Routes`/`GoRoute`（目标端声明词，禁止借词） |
| `@layout` | 域 11 导航 | 布局前缀声明 → 嵌套 Route / ShellRoute / 嵌套渲染 | `Outlet`/`ShellRoute`/`Layout`（目标端词） |
| `@query` | 域 11 导航 | 查询参数声明 → useSearchParams / uri.queryParameters / URLSearchParams | `query`/`searchParams`（目标端词） |
| `navigate` | 域 11 导航 | 应用内跳转 → useNavigate / context.push / hash 更新 | `push`/`go`/`useNavigate`/`Navigate`（目标端词） |
| `back` | 域 11 导航 | 返回 → navigate(-1) / context.pop / history.back | `pop`/`back()`（目标端/平台词） |

维护者注记：路径参数注入 prop 名（`:id` → `id`）为**编译器保留名判定**（随路径声明动态产生），非登记词汇条目（照 builtin signal 先例：formErrors/loading 等不登记）。

### 5.3 verify 脚本锚点方案

- `verify-semantic-universe.py`：域 11 条目存在性断言——语义集 8 条目全绿 + 已实现白名单 6 锚点（存在性检查，机制级条目照 ADR-074 例外：存在性断言即可）+ 保留名 `navigate`/`back` 与既有保留名表无重
- `verify-vocabulary.py`：域 7 五词汇四字段校验 + 排除词汇禁借断言（目标端词不得成为语义词汇）
- 文档一致性：docs/omniview 导航章节与域 11 条目同步（docs-gate）

---

## 6. 语言边界与触发条件

本批次未进语言项与边界记录（semantic-universe 状态值域「留生态」+ 触发条件）：

| # | 边界 | 类型 | 说明与触发条件 |
|---|---|---|---|
| 1 | 动态守卫目标（按角色跳不同页） | 扩展点 | v0 `redirect:` 限静态路径字面量；按角色/上下文计算目标的守卫在语料出现（真实多角色权限项目）时重评 |
| 2 | 通配符路径 / 404 fallback | 留生态 | React Router `*`、GoRouter `notFound` 为目标端机制；learn-omniview 无 404 语料，触发条件 = 语料 |
| 3 | 表达式守卫（`guard: { isAdmin && isEnabled }` 形态） | 扩展点 | v0 限组件级 fn（V9）；表达式守卫随语料重评（形态成本低，但 YAGNI） |
| 4 | 深链平台清单配置 | 机制语法面 | App Links/Universal Links 清单文件（AndroidManifest/Info.plist）是平台专属表达通道，编译器只出注记不生成清单——诚实降级（ADR-054） |
| 5 | 懒加载移动端 | 机制语法面 | Flutter 移动端单 bundle 无运行时按需加载概念（research 01 项 14）——编译器 no-op + 编译注记，绝不静默 |
| 6 | 手写页面切换（`@if` 互斥 / `event onNavigate` 通知） | 语义化即切换收口 | `@if` 与组件事件本身合法，**不可硬诊断**；收口 = 文档引导（导航意图写 @route/navigate）+ demo 迁移（§8）；tab 面板（UI 态）不自动路由化（未决 #1） |
| 7 | 页面保挂载（IndexedStack 行为） | 降级注记 | PageSwitcherDemo 既有行为（切页表单输入不丢）；GoRouter 标准形态下页面卸载，保挂载 = `StatefulShellRoute.indexedStack`——v0 以标准 ShellRoute 落地 + 差异注记，indexedStack 变体随语料（未决 #3） |
| 8 | navigate 路径跨文件注册校验 | 编译器深化 | W2 字面量校验依赖全工程注册表（实施含 import 图收集）；动态表达式校验属 1F 跨文件验证空洞深化（基线 §5 注记），不阻塞本批次 |
| 9 | 滚动恢复 / 预取 / 缓存策略 | 留生态 | research 01 遗漏补充项裁决：留生态（编译器优化职责候选），触发条件 = 语料 |
| 10 | 外部链接 | 语言边界 | 外链走既有 `<a href>`（V12 强制），导航语义只覆盖应用内路径 |

---

## 7. 门禁与验证（五层）

| 层 | 门禁 | 本批次验收内容 |
|---|---|---|
| 1 | tsc | react-test-app 全量 `tsc --noEmit` 零错误——新增 router 接线/页面组件生成代码（createBrowserRouter/RouterProvider/loader/useNavigate/useSearchParams）通过类型检查 |
| 2 | dart analyze | flutter-test-app `dart analyze` 零错误——GoRouter 接线（GoRoute/redirect/ShellRoute/pathParameters/uri.queryParameters/context.push/pop）零 analyzer 告警 |
| 3 | Playwright | behavioral-parity.spec.ts 扩展路由断言（见下方清单）——React + OmniView 编译产物浏览器实测 |
| 4 | docs-gate | verify-semantic-universe.py 域 11 全绿 + verify-vocabulary.py 域 7 全绿 + docs/omniview 导航文档与登记册同步 |
| 5 | 注入语料 | 855 元组扩展：路由语法元组 12+（`@route 静态路径`/`@route :param`/`@route guard`/`@route lazy`/`@layout`/`@query`/`navigate 字面量`/`navigate + params`/`navigate + query`/`back`/`参数注入引用`/`守卫失败重定向`）；learn-omniview NavDemo 三目标编译全绿 |

**Playwright 浏览器行为断言清单**：

1. 应用启动 → 默认路由（`/` 或 `/` 重定向）渲染
2. 点击导航按钮 → 页面切换 + 地址栏 URL 同步（React history 形态 / OmniView hash 形态）
3. 直接访问 `#/todos/42` → 参数注入渲染（显示「待办 #42」）
4. 访问 `#/todos?filter=all` → `filter` prop 注入生效
5. 点击返回按钮 → 上一页（back 语义）
6. 浏览器后退按钮 → 页面返回（URL 同步行为）
7. 未授权访问 `/admin` → 重定向 `/login`，admin 内容不渲染
8. 嵌套布局：访问 `/admin/users` → AdminLayout 外壳（header）渲染 + AdminUsers 内嵌于 `<@slot/>` 出口
9. 懒加载页面 → Suspense fallback 出现后内容渲染（React 端）
10. `navigate` 未注册路径 → 空路由兜底（目标端行为，W2 已警告）

---

## 8. demo 迁移清单（learn-omniview 用点逐一）

| # | 现状（语料位） | 迁移动作 | 新语法替换 |
|---|---|---|---|
| 1 | `src/components/PageSwitcherDemo.omni`：HomePage/AboutPage 手写 `@if` 互斥 + `event onSwitch` 通知 + PageSwitcherDemo 外壳 | 迁移为**导航演示**：HomePage `@route "/home"`、AboutPage `@route "/about"`、按钮改 `navigate("/about")` / `navigate("/home")`、删除 PageSwitcherDemo 外壳与 onSwitch 事件；Flutter 保挂载行为注记（边界 7） | `@route` + `navigate` |
| 2 | 同文件构建产物 12 件（PageSwitcherDemo.tsx/.dart、HomePage、AboutPage、behavioral-parity.spec.ts、page_switch_test.dart 等） | 同步重生成 + 测试断言改路由形态（切换断言 → URL 变化 + 页面渲染） | — |
| 3 | `src/main.omni` App 9 标签页 `@if` 链 + TabBar | **保留 tab UI 态**（语言边界 6：tab ≠ 路由）；不强制迁移（未决 #1 用户拍板后执行） | — |
| 4 | `src/pages/Overview.omni`：`event onNavigate: String` + `@onNavigate={onNavigate}` + `tabName` prop（CapabilityCard 通知父级切页模式） | CapabilityCard 组件事件模式**保留**（合法组件事件）；Overview 内「跳页面」意图改 `navigate()`（新 NavDemo 入口卡片用 navigate("/nav")） | `navigate` |
| 5 | 新增 `src/pages/NavigationDemo.omni`（或 components/NavDemo.omni） | 新 demo 覆盖 C1-C8 全要素：页面切换 + `/todos/:id` 参数详情 + `@query filter` + guard 登录示例（admin 页面 + 重定向）+ `@layout` 嵌套（AdminLayout）+ `lazy: true` 页面 + back 按钮 | `@route`/`@layout`/`@query`/`navigate`/`back`/`guard`/`lazy` 全量 |
| 6 | main.omni 引入 + TabBar/Overview 挂 NavDemo 入口 | 接线新 demo 页面 + 编译三目标全绿 | — |
| 7 | `docs/omniview/` 导航相关章节（13-project-structure 等） | 导航原语文档：`@route`/`navigate`/`back` 用法 + floor 差异注记（深链/懒加载/栈空 back）+ tab 与路由边界 | — |

---

## 9. 未决问题（需用户/主 agent 拍板，本 spec 不擅自裁决）

1. **learn-omniview 9 标签页是否迁移为 @route 页面**：tab 面板（UI 态）vs 路由（URL 语义）边界已定（tab 不自动路由化），但 learn-omniview 作为演示语料，9 标签页是否改为真路由（URL 可直达每标签）影响 demo 迁移范围与验收口径——请拍板（§8 #3）
2. **守卫失败默认重定向目标**：`guard:` 无 `redirect:` 属性时默认重定向到 `/`（本 spec 草案）——或强制 `redirect:` 必填（无默认）。默认 `/` 更少样板但隐式；必填更显式。请拍板（2.5）
3. **Flutter 保挂载语义 v0 范围**：`StatefulShellRoute.indexedStack`（保挂载）是否纳入 v0，或 v0 标准 ShellRoute + 差异注记（本 spec 草案：注记先行）——影响 PageSwitcherDemo 迁移后表单输入不丢行为的保留方式（§6 边界 7）
4. **React Router v7 精确版本与包形态**：网络封锁下无法核实 v7 API 细节（research 01 同口径）——库绑定决策已定（react-router v7 data router），精确版本号与 package.json 模板更新方式由实施计划定；若 v7 实际 API 与草案有出入，实施期以官方文档为准并回填注记（不属语言面变更）

---

## 自查结论

- 占位符（TBD/TODO）：清零
- 节间矛盾：清零（floor 契约 §4.3 ↔ 边界 §6 ↔ 门禁 §7 口径一致：C4/C8 部分降级、C1-C3/C5-C7 对等）
- 与基线矛盾：零——未推翻任何裁决（8 项全进语言、@layout 独立语法、深链进 floor、审计细化点 2 已行使）；§9 三项为基线未定细节的拍板请求，非矛盾
