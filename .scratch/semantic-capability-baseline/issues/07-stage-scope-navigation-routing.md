# 07 — 批次 2 域裁决：导航与路由

Type: grilling
Status: resolved
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

## Answer

**批次 2（导航路由）域裁决定案**（2026-08-09，用户「前端功能跨框架通用」标准校准后）。

### 标准校准（用户拍板，全体批次适用）

1. 前端功能 = 跨框架通用——**语言表达意图是默认，编译器找每目标地道映射是职责**；「一端实现不了」= 映射缺口（设计任务），不是排除理由
2. 「Flutter 实现不了」前提证伪：深链（GoRouter Deep linking + App Links/Universal Links + web URL 同步）、守卫（GoRouter redirect 内建）、嵌套（ShellRoute）、返回（Navigator 栈）——**映射全部存在**
3. 三判据重校准：跨目标 = 意图能否找到地道映射路径（含选路 + 诚实降级注记）；编译器理解 = 不变；语料需求 = **降级为优先级判据**（排批次），不否决进语言
4. 「留生态」收窄为：① 目标专属机制语法面（CSS 选择器/@media——表达通道差异非功能缺失）② 编译器职责（memo）；**其余全部进语言**（research 的留生态倾向全部翻案重审）

### 裁决表（全量进语言）

| 能力项 | 结论 | 三目标映射方向 |
|---|---|---|
| 页面切换 | 🔜 进语言（域 C） | React Router（库绑定）/ Flutter GoRouter（官方推荐）/ OmniView hash 路由 |
| 路由参数 | 🔜 进语言 | `:id` 路径段跨生态同构 → prop 注入（React useParams / GoRouter pathParameters / OmniView 解析） |
| 返回/前进 | 🔜 进语言 | `back()` 内建 → useNavigate(-1) / Navigator.pop / history.back |
| 深链/URL 同步 | 🔜 **进 floor**（用户修正：映射存在） | React URL 同步 / Flutter Deep linking（移动端 App Links + web URL）/ OmniView hash；形态差异诚实降级注记 |
| 守卫/权限 | 🔜 **域 C v0 含守卫**（语料只影响排期） | React loader 重定向 / GoRouter redirect / OmniView 守卫 fn |
| 嵌套路由 | 🔜 **独立语法**（用户拍板——layout 声明，非组合折衷） | React Router `<Outlet/>` / GoRouter ShellRoute / OmniView 嵌套渲染 |
| URL 查询参数 | 🔜 进语言（随域 C） | useSearchParams / uri.queryParameters / URLSearchParams |
| 懒加载 | 🔜 路由懒加载随域 C（编译器职责优先） | React.lazy+Suspense / Flutter web deferred（移动端 no-op 注记）/ OmniView 动态加载 |

### 设计方向（域 C 契约，方向级）

- `@route "/todos/:id"` 页面组件声明（含守卫/嵌套/lazy 属性扩展点）+ `navigate(path, {params})` / `back()` 内建语义函数 + 参数 prop 注入
- **floor 界定修正**：全部意图 floor 内（行为对等 + 平台差异诚实降级注记——移动端深链形态、web 同步差异）；「floor 外」仅保留机制语法面差异
- **audit 细化点 2 激活**：React Router 库绑定与依赖注入（Zustand 先例）；Flutter GoRouter vs Navigator 取舍——域 C 阶段 spec 定
- semantic-universe 新域 11 导航登记随批次 spec；批次 spec = 域 C 契约细化（showToast/@modal 同规格）
