# 02 — 能力全景实证：反馈/渲染/样式（研究产出）

> 车票：[issues/02-capability-landscape-feedback-and-style.md](../issues/02-capability-landscape-feedback-and-style.md)
> 研究方法：官方文档一手抓取（URL 见文末「来源」），浏览器支持基线用 MDN BCD API（v8.0.9，2026-08-03）逐项验证。不采用二手博客。
> 判据口径：① 生态化程度——有标准库 = 留生态候选，无标准形态 = 进语言候选；② 跨目标一致性——同一意图在 React/Flutter 两端能否对齐（进语言判据输入）。

## 速览表

| # | 能力 | React 侧 | Flutter 侧 | 生态化 | 跨目标一致 | 初步倾向 |
|---|------|----------|------------|--------|------------|----------|
| 1 | toast/通知 | 库生态（Sonner/Radix Toast） | SnackBar（平台内置） | 无标准形态（库是事实标准） | 可对齐 | 进语言候选 |
| 2 | modal/dialog | 原生 `<dialog>` + Radix/Headless UI | showDialog（平台内置） | 双端均有标准形态 | 强对齐 | 进语言候选 |
| 3 | tooltip | Radix Tooltip（库） | Tooltip（平台内置） | React 端无标准形态 | 部分对齐 | 留生态候选 |
| 4 | 骨架屏 | 无标准（Suspense fallback 近似） | 无内置（shimmer 库） | 无标准形态 | 可对齐 | @load 形态扩展 |
| 5 | 空状态 | 无标准（条件渲染模式） | 无内置 | 无标准形态但语义过窄 | 可对齐 | 留生态 |
| 6 | 错误边界 | 官方模式 + react-error-boundary | 无组件级标准（ErrorWidget 全局兜底） | 部分标准 | 部分对齐 | 进语言候选（弱） |
| 7 | 虚拟滚动 | TanStack Virtual（库） | ListView.builder（平台内置） | React 端无标准形态 | 意图对齐、实现差异大 | 留生态 |
| 8 | 懒加载 | React.lazy + Suspense（官方） | 无组件级等价 | React/Vue 官方、Flutter 无 | 部分对齐 | 留生态（车票 01 已裁） |
| 9 | keep-alive | 无标准 | AutomaticKeepAliveClientMixin（窄） | 无全目标标准形态 | 部分对齐 | 留生态候选 |
| 10 | 复杂选择器 | CSS 原生（平台能力） | 无选择器概念 | 平台能力非库 | 不一致 | 留生态（CSS 透传） |
| 11 | 用户级 @media 断点 | CSS @media + Tailwind 断点 | MediaQuery（无断点系统） | 平台能力 | 意图对齐 | 留生态（CSS 透传） |
| 12 | 容器查询 | CSS @container（2022-23 基线） | LayoutBuilder 近似 | 平台能力（较新） | 不一致 | 留生态 |
| 13 | 样式作用域/模块化 | CSS Modules/Tailwind/vanilla-extract 多标准 | 无（widget 内联样式） | 多标准并存 | 不一致 | 留生态（已支持） |
| 14 | 深色模式 | prefers-color-scheme + Tailwind dark 变体 | ThemeMode/ThemeData（平台内置） | 双端均有标准形态 | 强对齐 | 进语言候选（token 扩展） |

---

## 一、反馈簇

### 1. toast/通知

**① 标准形态**：无跨框架统一标准，三端形态各异。
- React：库生态是事实标准——**Sonner**（命令式 `toast()` 调用 + 全局挂载 `<Toaster/>`，可放 server components；作者 Emil Kowalski）与 **Radix Toast**（headless 声明式：`Toast.Provider` 全局 duration 默认 **5000ms**、`Root` 可按条覆盖；`Title/Description/Action/Close/Viewport` 分部；`swipeDirection` 默认 right、`swipeThreshold` 默认 50；**悬停/聚焦/窗口失焦时暂停计时**；hotkey 默认 F8 跳转视口；`type` foreground/background；键盘交互 Tab/Enter/Space/Esc 齐全）。
- Vue：组件库内置命令式——**Element Plus ElMessage**（`type: success/warning/info/error`，**duration 默认 3000ms、0 = 不自动关闭**，支持 grouping、offset、showClose）。
- Flutter：平台内置 **ScaffoldMessenger.showSnackBar**（命令式，SnackBar 为底部临时通知，`MaterialBanner` 为顶部横幅；嵌套 Scaffold 时由最近的 ScaffoldMessenger 管理）。

**② 生态化程度**：React 侧**无平台标准形态**（Sonner/Radix 是第三方事实标准），Flutter 侧**无 toast 概念**（只有 SnackBar/MaterialBanner）。按判据「无标准形态 = 进语言候选」。

**③ 跨目标一致性**：通知意图（短暂消息 + kind + duration + 可关闭 + 堆叠）两端**可对齐**：React 生成 toast 库调用或自定义实现、Flutter 生成 SnackBar。差异在形态（Material SnackBar vs web toast）与触发细节，但语义层一致。

**④ 对 OmniLang 启示**：**进语言候选**——「告诉用户一个短暂消息」是高频、意图清晰、编译器可理解并映射双端的命令式语义，与域 9 `@load` 三态先例同类（命令式副作用意图）；详细签名（showToast kind/duration/堆叠）留各阶段 spec。

### 2. modal/dialog

**① 标准形态**：双端均有平台级标准形态。
- React：原生 **`<dialog>.showModal()`**（MDN）——top layer 渲染、`::backdrop` 遮罩伪元素、**Esc 默认关闭**（cancel 事件）、**焦点自动移入对话框内第一个可聚焦元素**、`form method="dialog"` 提交即关闭、`:modal` 伪类；无障碍配套 Radix Dialog（Portal/Overlay/Content，Title/Description 供读屏）与 Headless UI Dialog（renderless）。
- Vue：Element Plus el-dialog（v-model 控制；**close-on-click-modal 默认 true、close-on-press-escape 默认 true、show-close 默认 true**；before-close 回调可拦截；destroy-on-close、fullscreen、modal 默认 true、**lock-scroll 默认 true**（body 滚动锁定）、嵌套需 append-to-body；top 默认 15vh、width 默认 50%）。
- Flutter：**showDialog**（返回 **`Future<T?>`** 结果、**barrierDismissible 默认 true**、`Navigator.pop` 关闭返回结果）+ FocusTraversalGroup（焦点遍历策略）。

**② 生态化程度**：双端均有标准形态（React 原生 `<dialog>`、Flutter showDialog），库（Radix/Headless UI）提供的是无障碍/样式增强而非替代。这是清单中唯一「双端平台级标准齐全」的能力。

**③ 跨目标一致性**：**强对齐**——打开/关闭/遮罩点击/Esc/焦点陷阱/内容投影六要素两端原生对应（showModal vs showDialog；::backdrop vs barrier；Esc 默认关闭双端一致）。React 端 `<dialog>` 的焦点管理为浏览器内建，Flutter 端由 showDialog 的 focus scope 内建。

**④ 对 OmniLang 启示**：**进语言候选（强）**——车票 02 已点名此例（「React 原生 `<dialog>` vs Flutter showDialog」）；意图「模态对话框」跨目标一致表达 + 编译器可直接生成地道原生实现（`showModal`/`showDialog`），是「进语言」判据的理想样本。`@modal` 语法方向留各阶段 spec。

### 3. tooltip

**① 标准形态**：React 侧无平台标准——事实标准为 **Radix Tooltip**（headless，Provider/DelayGroup 控制延迟分组）；Vue 侧 Element Plus el-tooltip（**placement 9 方位** `[orientation]-[alignment]`，effect dark/light）；Flutter 侧平台内置 **Tooltip**（`message`、`waitDuration`（悬停多久后显示）、`showDuration`、`exitDuration`、`triggerMode`（longPress/tap/manual + `ensureTooltipVisible`）；移动端长按触发、桌面悬停触发；IconButton/FAB/PopupMenuButton 均带 tooltip 参数）。

**② 生态化程度**：Flutter 有平台标准，React 无（依赖 Radix 等库）。**半标准**——按判据倾向留生态，但因 Flutter 内置而存在跨目标表达可能。

**③ 跨目标一致性**：**部分对齐**——语义（简短说明 + 位置 + 触发方式）可表达，但触发模型差异显著（web 悬停/聚焦 vs 移动端长按），且 tooltip 是附着于任意组件的修饰属性，与组件组合性强。

**④ 对 OmniLang 启示**：**留生态候选**——React 端无标准形态、需生成第三方库引入，且语义面窄（属性级修饰而非独立能力），进语言收益低于 toast/dialog；可作为属性级修饰方向在后续批次再议。

### 4. 骨架屏

**① 标准形态**：React 无标准（官方近似是 `<Suspense fallback>` 占位）；Vue 组件库内置——Element Plus el-skeleton（**loading prop 控制显示骨架还是真实 DOM**、animated 动画、el-skeleton-item variant="image"/"text"）；Flutter 无内置——第三方 **shimmer** 包是事实标准（pub.dev 3.0.0，约 143 万下载）。

**② 生态化程度**：三端均无平台标准形态（Vue 组件库内置、Flutter 靠库）。

**③ 跨目标一致性**：**可对齐**——骨架屏本质是「加载状态的视觉占位」，与车票 01 已裁决进语言的加载三态（域 9 `@load`）是同一语义的不同呈现形态。

**④ 对 OmniLang 启示**：**不独立成项，作为 `@load` 的形态扩展**（loading 态 → 骨架屏/三态动画），随域 9 批次一并设计；单独进语言收益低（呈现细节非独立意图）。

---

## 二、渲染簇

### 5. 空状态

**① 标准形态**：React/Flutter 均**无标准组件**（通用模式是条件渲染分支）；Vue 组件库内置 Element Plus el-empty（`description` 文本、`image`/`image-size` 自定义图，default 槽放底部操作按钮）。

**② 生态化程度**：无标准形态，但**语义过窄**——「无数据时显示占位 UI」本质是条件渲染 + 占位内容，`@if`/`@for` 条件语法已能完整表达。

**③ 跨目标一致性**：可对齐（描述文本 + 可选操作按钮），但编译器无意图增益——它不改变渲染结构，只决定显示什么内容。

**④ 对 OmniLang 启示**：**留生态**——编译器理解它没有独立意图价值，进语言是语法冗余；如真实语料证明高频，可作 `@for` 空集合的默认分支（`empty:` 槽）轻量支持，留批次再议。

### 6. 错误边界

**① 标准形态**：React 有**官方模式**——class 组件 `getDerivedStateFromError` + `componentDidCatch`（不捕获事件处理器/异步代码/SSR/边界自身错误），社区事实标准封装 **react-error-boundary**；Vue 框架内置——组件级 `errorCaptured` 钩子 + 全局 `app.config.errorHandler(err, instance, info)`；Flutter **无组件级标准**——只有 `ErrorWidget.builder` 全局兜底。

**② 生态化程度**：React/Vue 有标准形态，Flutter 无——**跨目标一致性不足**是硬伤。

**③ 跨目标一致性**：**部分对齐**——意图「子树渲染失败 → 显示降级 UI」在 React/Vue 可声明式表达，Flutter 无组件级对应（需自建 widget 包裹或全局兜底）。

**④ 对 OmniLang 启示**：**进语言候选（弱，待批次裁决）**——意图清晰（错误降级）、真实语料需求高，且与「诚实降级 > 静默错误」（ADR-054）哲学呼应；但 Flutter 端无标准形态，进语言须设计诚实降级路径（ErrorWidget/全局兜底 + 文档声明），跨目标一致性判据不满足时的取舍留批次讨论。

### 7. 虚拟滚动

**① 标准形态**：React 无平台标准——事实标准 **TanStack Virtual**（headless，`useVirtualizer`，React/Vue/Svelte/Solid 通用，无包裹 DOM）；Vue 组件库内置虚拟化表格（Element Plus Table 虚拟化行、Virtualized Tree）；Flutter **平台内建**——ListView.builder/GridView.builder 惰性构建（虚拟化是默认行为）；web 通用原语 MDN IntersectionObserver。

**② 生态化程度**：React 端无标准形态（靠库），Flutter 端内建——但这是**性能类能力**。

**③ 跨目标一致性**：意图对齐（大数据列表只渲染可视区域），**实现差异巨大**（web 需虚拟化库/手写窗口化，Flutter 内建滚动惰性构建）。

**④ 对 OmniLang 启示**：**留生态**——与车票 01 已裁决一致（性能类分页/缓存/竞态/懒加载均留生态）；虚拟滚动是优化而非语义意图，编译器生成「正确但低效」的简单列表 + 文档指引即可，优化细节留给用户选库。

### 8. 懒加载

**① 标准形态**：React **官方内置**——`React.lazy`（动态 import + 默认导出）需 `<Suspense fallback>` 包裹；Vue **框架内置**——`defineAsyncComponent` + `<Suspense>`（`#default`/`#fallback` 槽；初始渲染遇异步依赖 → pending 显示 fallback → 全部 resolve 显示 default；**回退场景保留旧 default 内容**，超过 timeout 才切 fallback，timeout 0 立即切）；Flutter **无组件级等价**（只有包级 deferred loading，无按组件代码分割）。

**② 生态化程度**：React/Vue 官方标准、Flutter 无——**跨目标不一致**。

**③ 跨目标一致性**：部分对齐（React/Vue 对齐，Flutter 无法表达）。

**④ 对 OmniLang 启示**：**留生态（车票 01 已裁决）**——懒加载是构建/性能策略而非界面语义，且 Flutter 目标无法对齐；维持裁决，不做进语言设计。

### 9. keep-alive

**① 标准形态**：Vue **框架内置**——`<KeepAlive>` 缓存组件实例（`include`/`exclude` 按组件 name 匹配，逗号字符串/RegExp/数组；**`max` 上限 = LRU 缓存**，超限逐出最久未访问实例；`activated`/`deactivated` 生命周期钩子）；Flutter 部分内置——`AutomaticKeepAliveClientMixin`（PageView/TabBarView 内保活）；React **无标准**（状态保留需状态提升或第三方库）。

**② 生态化程度**：无全目标标准形态（React 缺失是硬伤）。

**③ 跨目标一致性**：部分对齐——意图「组件离开视口保留状态」Vue/Flutter 有（Flutter 仅限滚动容器场景），React 无标准表达。

**④ 对 OmniLang 启示**：**留生态候选**——与路由（车票 01 已进语言）关联度高，可作路由/页面切换的状态保留选项（tab 场景），但不独立进语言；React 端无标准形态 + Flutter 端场景受限，跨目标一致性不足，留批次再议。

---

## 三、样式簇

### 10. 复杂选择器（兄弟/属性/:nth-child/后代伪类）

**① 标准形态**：CSS 平台原生能力——相邻兄弟 `+`、一般兄弟 `~`、属性选择器 `[attr^=]`、`:nth-child`、逻辑伪类 `:not/:is/:where`、关系伪类 `:has()`（BCD 基线：Chrome 105 / FF 121 / Safari 15.4，2022-23 全基线）；Flutter **无选择器概念**（widget 树内联样式，样式只能作用在组件自身）。

**② 生态化程度**：平台能力而非库——web 端是 CSS 语言的一部分，用户可直接书写。

**③ 跨目标一致性**：**不一致**——React 端即 CSS 原生（透传即达），Flutter 端无任何对应表达。

**④ 对 OmniLang 启示**：**留生态（CSS 透传）**——复杂选择器是 CSS 原生语法面，OmniLang 样式块透传即可（编译器不解析选择器内部结构）；Flutter 端诚实降级（样式作用于组件自身），不抽象进语言——抽象它等于重造 CSS。

### 11. 用户级 @media 断点

**① 标准形态**：CSS `@media` 媒体查询，现代 **range 语法** `width >= 900px`（MDN 现行推荐写法）；Tailwind 提供预设断点体系（sm/md/lg/xl/2xl）与任意值扩展；Flutter **无内置断点系统**（MediaQuery 只提供当前尺寸，断点定义是用户责任）。

**② 生态化程度**：平台能力（CSS 原生）——web 端透传即达。

**③ 跨目标一致性**：意图对齐（响应式断点 = 尺寸条件分支）——CSS 侧 `@media` 直写，Flutter 侧 MediaQuery 尺寸判断可映射；但断点定义方式无标准（CSS 数值 vs Flutter 代码条件）。

**④ 对 OmniLang 启示**：**留生态（CSS 透传）**——响应式断点是 CSS 原生语法面，样式块透传 `@media`；Flutter 端由编译器将尺寸条件映射为 MediaQuery 分支（或诚实降级为文档指引）。用户级断点无需新语法，进语言收益低。

### 12. 容器查询

**① 标准形态**：CSS `@container` + `container-type: inline-size`（BCD 基线：Chrome 105 / FF 110 / Safari 16，2022-23 全基线）；vanilla-extract 等样式工具已支持容器查询语法；Flutter **无等价**（LayoutBuilder 仅近似，无「容器尺寸条件样式」语义）。

**② 生态化程度**：平台能力（CSS 原生），基线较新（2022-23 才全浏览器可用）。

**③ 跨目标一致性**：**不一致**——React 端 CSS 原生可透传，Flutter 端无任何对应（LayoutBuilder 是命令式逻辑非声明式样式条件）。

**④ 对 OmniLang 启示**：**留生态（CSS 透传）**——同 @media：CSS 原生语法面，样式块透传；Flutter 端诚实降级。容器查询语义（组件自适应容器而非视口）是 CSS 层演进，不属于语言级抽象范围。

### 13. CSS 作用域/模块化

**① 标准形态**：生态多标准并存——**CSS Modules**（构建期局部类名散列化，:local/:global/composes）、**Tailwind**（utility-first，类名即样式）、**vanilla-extract**（零运行时 CSS-in-TypeScript：类型安全局部类 + 变量 + createTheme 主题契约，构建期生成静态 CSS，webpack/esbuild/Vite 官方集成，支持含容器查询在内的全部 CSS 特性）、CSS-in-JS（styled-components，运行时）；Flutter **无此概念**（widget 内联样式天然局部）。

**② 生态化程度**：生态化程度极高（多标准并存）——作用域是构建工具链职责。

**③ 跨目标一致性**：**不一致**——React 端是构建期类名策略，Flutter 端无类名体系（样式直接绑定 widget）。

**④ 对 OmniLang 启示**：**留生态（已支持）**——OmniLang 已有组件级样式作用域（StyleEntry/设计 token 体系，样式绑定组件即局部），作用域问题已被语言结构解决；模块化细节（类名散列策略）是 codegen 内部实现选择（React 端按目标生成 CSS Modules 或内联，Flutter 端 widget 样式），无需用户语法。

### 14. 深色模式

**① 标准形态**：双端均有平台标准。
- CSS：`prefers-color-scheme` 媒体查询（BCD 基线：Chrome 76 / FF 67 / Safari 12.1）；
- Tailwind：`dark:` 变体，**默认 media 策略**（跟随 prefers-color-scheme），可覆盖为 **class 策略**（`@custom-variant dark (&:where(.dark, .dark *))`，html 加 `.dark` 类手动切换，配 localStorage 持久化）——双策略是生态事实标准；
- Flutter：平台内置 **ThemeMode.system/light/dark + ThemeData.dark**（跟随系统或应用内切换）。

**② 生态化程度**：双端均有标准形态——与 dialog 并列的「平台级标准齐全」能力。

**③ 跨目标一致性**：**强对齐**——意图「跟随系统/手动切换的主题色替换」两端原生对应（prefers-color-scheme vs ThemeMode.system；Tailwind dark 变体 vs ThemeData.dark）。

**④ 对 OmniLang 启示**：**进语言候选（token 体系自然扩展）**——OmniLang 已有设计 token 管线（__omni_theme 优先级、动态 token），深色模式 = token 层的系统级主题分支；编译器可生成 `@media (prefers-color-scheme: dark)` / `ThemeMode.dark`，与现有 token 语法无缝整合，留批次随 token 体系设计。

---

## 清单对照

车票覆盖清单 14 项全部完成实证，无遗漏。补充观察：

1. **清单外发现——Svelte 过渡体系**（`transition:` 指令 + `svelte/transition` 的 fade/fly/slide/scale/blur/crossfade + `svelte/easing`/`svelte/motion`/`svelte/animate`（flip））：显示/隐藏过渡与反馈簇强相关（toast 滑入、dialog 淡入、骨架屏闪动都是其应用），但骨架屏/dialog 的过渡是形态细节，非独立能力；且 Flutter 侧等价物为 AnimatedSwitcher（Key 身份交叉过渡），跨目标可部分对齐。建议：不独立成项，过渡作为 toast/@modal/@load 形态参数（duration/easing）在各自批次内处理。
2. **Flutter 无「toast」「空状态」「骨架屏」「错误边界」四个平台组件**——这是反馈/渲染簇进语言的最大障碍项（需诚实降级设计），也是留生态项的共性理由。
3. **样式簇四能力（复杂选择器/@media/容器查询/模块化）共性**：都是 CSS 原生语法面，OmniLang 样式块透传即达；进语言 = 重造 CSS，均留生态。真正值得进语言的样式能力只有与现有 token 体系整合的深色模式。
4. **焦点陷阱细节**：React `<dialog>` 焦点管理为浏览器内建（移入第一个可聚焦元素），Radix/Headless UI 提供的是库级增强；Flutter showDialog 内建 focus scope——`@modal` 进语言时编译器两端均无需手写焦点逻辑，这是 dialog 进语言可行性的关键事实。
5. **验证缺口**：Svelte 侧组件库形态（svelte-sonner 等）与 react-hot-toast 未完成一手抓取（GitHub 页面 JS 渲染失败），不影响本批结论（Svelte 非三目标之一；React toast 结论已由 Sonner/Radix 官方文档覆盖）。

## 来源

一手官方文档（抓取时间 2026-08-09，均 HTTP 200）：
- MDN：`<dialog>`、prefers-color-scheme、@media（含 range 语法）、容器查询、:has()、IntersectionObserver、CSS containment（top layer/inert）
- BCD API：bcd.developer.mozilla.org/bcd/api/v0/current/（版本基线：@container Chrome 105/FF 110/Safari 16；:has() Chrome 105/FF 121/Safari 15.4；prefers-color-scheme Chrome 76/FF 67/Safari 12.1）
- Radix UI Primitives：Dialog / Toast / Tooltip 组件文档
- Headless UI：Dialog（renderless）
- Sonner 官方文档（getting-started）
- react.dev：lazy、Component（错误边界）
- vuejs.org：KeepAlive、Suspense、Teleport、application（app.config.errorHandler/warnHandler）
- Element Plus：message / dialog / skeleton / empty / tooltip 组件文档
- api.flutter.dev：ScaffoldMessenger / showDialog / Tooltip / FocusTraversalGroup / AnimatedSwitcher
- pub.dev：shimmer（3.0.0）
- TanStack Virtual 官方文档
- tailwindcss.com：dark-mode
- vanilla-extract.style 官方文档
- svelte.dev：transition 指令与 svelte/transition
