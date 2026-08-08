# Context — OmniLang 域词汇表

> 本文件是本仓库的**唯一**域词汇表：领域概念的定义与规范用词。不存放实现细节（函数名、行号、代码结构）、规格或决策——那些归 ADR（`.architecture/decisions/adrs/`）与 spec（`docs/superpowers/specs/`）。
> 使用纪律：产出（issue 标题、评审结论、测试名、设计文档）命名领域概念时使用本表词汇，不漂移到同义词；表中未载的概念要么是新造词（三思），要么是词汇缺口（交给 /domain-modeling）。
> ⚠️ = 词汇冲突未决；「词汇缺口」= 已实现构造但未在语义全集登记（待登记或判为机制级）。

## 语言与编译器

- **OmniLang** — 本项目的 DSL。`.omni` 源码文件声明界面：组件、状态（store）、设计 token、视图、事件、指令。多目标编译，`omnic` 为编译器 CLI。
- **编译器管线** — 词法/解析（lexer/parser）→ AST → 语义验证（semantic）→ IR → 目标代码（codegen）。「源文件 → Intent IR → 目标代码」是 Cargo.toml 描述语，也是本词表的事件顺序。
- **目标（target）** — 编译产物平台。**三目标** = React / Flutter / OmniView，是「地道」支持与跨目标保证的定义域；Vue/Dioxus/Python/Rust 等为次要目标。「后端（backend）」是旧称，正式产出不用（Cargo feature 名等代码标识除外）。
- **地道 codegen** — 目标惯用实现，非逐字翻译：@drag 在 React 合成 4 pointer props、在 Flutter 走 GestureDetector、在 OmniView 走 capturePointer。
- **语言语义（language semantics）** — 构造被编译器理解、跨目标有统一保证的性质。反面是**透传**。
- **语义验证（semantic validation）** — 编译器语义层的验证 pass 集合：对 AST 施加规则（ref 校验/事件名校验/表单数据校验等），产出中文诊断（硬错误/warning/提示）。
- **验证网（validation net）** — 语言的持久验证基础设施：注入语料（855 元组 + 模式缺失断言）、门禁（docs-gate/Flutter 门禁/浏览器套件）。「验证网先行」——任何语法域修改前必须先闭合门禁盲区。
- **诊断（diagnostic）** — 编译器对非法/可疑源码的输出：**硬诊断/硬错误**（error，阻断）、warning、迁移提示（hint）。
- **迁移提示（migration hint）** — 硬错误附带的「改用 @onX」式指引。
- **信号（signal）** — 响应式状态原语：store 信号、每实例 props 信号、只读信号（readonly signal）。DSL 侧声明，目标侧映射到地道机制（Zustand/Riverpod 等）。
- **store** — 状态容器。DSL 顶层声明（信号组），编译器映射到目标状态库。
- **元素局部坐标（element-local coordinates）** — 指针事件 x/y 以元素自身为基准（offset 语义），非视口坐标。事件 schema 的坐标基准契约。
- **语义键枚举（semantic key names）** — KeyEvent 的 key 字段语义化：19 个命名键 + 单字母/单数字，非法键值硬错误 + 近邻建议。

## 意图与指令

- **指令（directive）** — DSL 表面形态：元素/视图上的 `@` 前缀构造（@ref、@drag、@dragEnd、@form、@load、@if、@for、@empty、@store、@mount、@effect、@unmount、@theme、@themes…）。指令**表达意图**，编译器按目标选路实现。
- **意图（intent）** — 指令的编译器语义抽象，IR 层节点（DragIntent、FormIntent、LoadIntent、EffectIntent…）。一对概念：**DSL 指令（表面）→ IR 意图（语义）**。
- **事件意图化** — 把旧 DOM 事件名收敛为语义事件意图的过程（ADR-075）：旧名硬错误 + 迁移提示，无新旧双轨。
- **语义化（semanticize）** — 某构造从「透传」升级为「编译器理解 + schema 契约 + 三目标地道映射」。
- **机制级意图** — DSL 已理解的机制层语义：@drag/@ref/store/直写通道等。**机制残留（mechanism residue）** = 仍是透传的机制半壁（事件名/CSS/ARIA）。
- **功能设计级词汇** — DSL 尚缺的功能设计层词汇：表单校验、数据获取三态、路由、反馈/模态、导航、响应式……（阶段 2 起逐域补齐）。
- **语义化即切换（原则 8）** — 一旦语义化，旧形态不留透传共存：硬诊断 + 迁移提示，demo 与文档一次性迁移。
- **取向 B 增量登记** — 按语料需求渐进语义化：先收跨目标痛点最高的事件，其余维持透传并在登记册逐事件标注状态。
- **生命周期（lifecycle）** — 组件级声明钩子：@mount / @effect / @unmount（编译端 LifecycleHook 机制）。

## 语义级 DSL 设计（OmniLang 语义级时代）

> 本簇是 OmniLang 的定义性设计词汇（ADR-052/053/054 时代）：语言表达意图、编译器按目标选路实现。

- **语义级 DSL（semantic-level DSL）** — OmniLang 的定义性设计：`.omni` 描述「想做什么」（意图），编译器理解意图后为每个目标生成最地道代码；与「平台形状语法」（直接暴露 DOM 形状 API）相对。旧称「OmniView 语义 DSL」——OmniView 是首个目标/调试运行时，时代标题沿用旧名（ADR-054）。
- **OmniView-first 时代** — 语义级 DSL 之前的架构形态（DOM 形状 ref API、动画只测 OmniView、Design Token 仅 Web 路径）。「OmniView」双义是历史遗留：作**目标名**（零依赖单文件 HTML 调试运行时）与作生态旧称——规范用目标名。
- **语义原语（semantic primitive）** — 平台无关的语言构造（触发器/动画属性/RefOp/语义绑定/token 类型/数组方法/指令）；平台映射（DOM/Widget/VDOM）收敛在 codegen 层。「语义原语优先」= 设计原则 1。
- **三层架构（three-tier architecture）** — 语义原语 → IR → codegen（ADR-052）。
- **触发器（trigger）** — 视图/运动触发器，8 个：@enter/@hover/@press/@focus/@in-view/@scroll/@drag-horizontal/@drag-vertical。与事件不同（@press 是触发器非事件名，不进事件校验）。**词汇缺口**：未在语义全集登记。
- **跨目标对等 floor（24-item feature floor）** — floor 内三目标行为对等；floor 外功能产生硬诊断，绝不静默降级（ADR-021 重定义；ADR-054 原则 2）。
- **诚实降级（honest degradation）** — 目标无法表达某功能时：生成代码携带说明注释、文档表格如实记录每目标支持状态（ADR-054 原则 3）。
- **静默错误（silent error）/ 雷（landmine）** — 时代要消灭的缺陷类：「无法实现或已删除的功能必须显式诊断，而不是悄悄失效」。
- **运行时正确性由真实执行验证** — 测试三层：CI 工具链门禁（tsc/dart analyze）+ node 执行测试 + 浏览器冒烟（ADR-054 原则 4）。
- **文档与展示对齐（docs & showcase parity）** — 功能全集 × 展示项目 × 文档三方一致；展示项目与 BNF 共为**可执行文档**（ADR-054 原则 5）。
- **可执行文档（executable documentation）** — 语法规范（BNF）+ 展示项目（learn-omniview）作为语言规范的权威参照。
- **五方对齐** — 语言、编译器、运行时、展示项目、文档一致（语义级 DSL 时代的标志）。
- **一步到位（no transition period）** — 迁移决策原则：不设过渡期（ADR-052 决策 5）；与「语义化即切换」同源，后者是前者的语义化纪律形态。
- **BNF 语法规范（syntax spec）** — 语法的单一事实源，与设计文档分离：设计文档管「意图→平台代码」，语法规范管「意图怎么表达」（ADR-053 D1）。
- **混合 ref 模型（mixed ref model）** — ref 双形态：高频声明式绑定（focus=/scroll-x=/scroll-y=）+ 低频语义方法（ref.select()/ref.measure() 等 RefOp；ADR-053 D2）。
- **官方 API 优先** — 目标库选择原则：官方/标准 API → 热门社区库 → 无法实现则讨论删除（ADR-052 决策 4）。
- **Slots** — 组合机制：body 级 @slot 声明 + 命名 props 消费（设计文档域 4）。**词汇缺口**：未在语义全集登记。

## 功能设计级域（阶段 2 设计契约）

> 阶段 2 补缺的四个功能设计级语义域（spec 2026-08-08 §4）。域 A 已进入实施（登记全集域 8+9）；域 B/C/D 为**设计契约草案**（未实现、未在全集登记——登记发生在域登记流程第 4 步，先例 @drag）。

- **功能设计级域（feature-design domain）** — 表单/数据/反馈/导航/响应式这类「前端功能该怎么设计」的语义域；与机制级意图（拖拽/引用/直写）相对。四域：域 A 表单+数据三态 / 域 B 反馈+模态 / 域 C 导航/路由 / 域 D 响应式断点；实施顺序同此（导航跨目标差异最大，后置细究）。
- **域登记流程（@drag 模式）** — 新域落地七步：意图语义定义 → Validation 清单 → 三目标地道实现 → semantic-universe 新域登记 → BNF/文档 → 门禁 → demo 迁移（ADR-073 先例）。
- **意图契约（intent contract）** — 每域设计前期的契约草案形态：意图陈述 + 语法草案 + 三目标映射 + 决策点（spec §4 各域，实现细节归实施计划）。
- **内建语义函数（built-in semantic function）** — 编译器识别的内建函数调用（showToast / navigate / back / bp），DSL 语义词汇的第三类别：非 @ 指令、非 RefOp 方法、非事件。**设计草案**。

### 域 B：反馈 + 模态

- **反馈（feedback）** — 域主题：操作结果的短暂或模态呈现。
- **toast / 通知（notification）** — 短暂通知：kind（info/success/error）+ duration + 自动消失；内建 `showToast(msg, {kind, duration})` 替代手写 NotificationStore（learn-omniview 重构点预判）。三目标：React 轻量自实现 / Flutter ScaffoldMessenger.showSnackBar / OmniView 自绘。决策点：容器样式可定制性（默认样式 vs 设计 token 接入）。**设计草案**。
- **模态（modal）** — 覆盖层对话框：焦点陷阱（Tab 循环）、Esc 关闭、遮罩点击关闭、内容投影（复用 @slot）。元素指令形态 `<dialog @modal={openSig}>`；a11y（role=dialog/aria-modal/aria-labelledby）；进出场动画复用 @enter。Validation：@modal 值必须为信号、重复 @modal 错误、dialog 与 @for 组合规则。**设计草案**。
- **焦点陷阱（focus trap）** — 模态打开期间 Tab 循环于对话框内。

### 域 C：导航 / 路由

- **导航（navigation）/ 路由（routing）** — 页面切换、带参数、可返回：页面组件 `@route "/todos/:id"` 声明 + 内建 `navigate(path, {params})` / `back()`；参数经 prop 注入。三目标：React Router（事实标准）/ Flutter Navigator（官方）/ OmniView hash 路由。**设计草案**。
- **floor 界定（floor scoping）** — 域 C 的设计核心：页面切换/参数/返回为 floor 内行为对等；深链/URL 同步为目标差异 + 诚实降级注记（四域中跨目标差异最大的域）。

### 域 D：响应式断点

- **响应式断点（responsive breakpoint）** — 布局在断点处适应：design 域新增 breakpoint token（`breakpoint md: 768`）+ 使用形态 `@if bp("md")` 条件表达式（或变体维度，实施计划细化）。三目标：matchMedia hook / MediaQuery、LayoutBuilder / matchMedia。Validation：breakpoint 名必须声明，未声明引用 → 硬错误 + 建议。**设计草案**。

## 事件

- **语义事件（semantic event）** — 被编译器理解的 14 个事件名（首批）：有 schema 契约、坐标基准、语义键枚举，三目标地道映射。
- **语义族（semantic family）** — 14 个语义事件归 9 族：press / input / change / submit / key / pointer / scroll / focus / enter-leave。注意 input 与 change 是不同语义，非旧名关系。
- **canonical 事件名** — 54 个事件名的封闭表（14 语义化 + 40 透传），命中判定 = canonical 同值（裸拼写/大小写变体均命中）。
- **透传（passthrough）** — canonical 但无语言语义的事件/属性/词：原样传到目标 DOM；Flutter 端诊断兜底。语义化的反面。
- **事件 schema（event schema）** — 每语义族的事件载荷字段契约（编译器单一事实源）；字段名语言面统一，提取源按目标映射（如 Flutter `pointer` → `pointerId`）。
- **旧名 / legacy 拼写** — 被语义事件取代的旧 DOM 拼写（onClick→onPress、onMouseEnter→onEnter…），命中即硬错误 + 迁移提示。

## 语义治理

- **语义域（semantic domain）** — 一组语义同源、封闭的前端能力（与实现无关）。登记于**语义全集**，当前 7 域：可动画属性（motion）/ 数组操作（array）/ 事件（events）/ RefOp / design token 类型 / 值绑定（bind）/ 元素指令与绑定（directives）；域 8 表单、域 9 数据三态登记中（阶段 2）。**全集编号是唯一规范**；引用设计文档写「设计文档域 N」（10 域编号，含 a11y），引用登记册写「词汇域 N」（5 封闭词汇域）。
- **语义全集（Semantic Universe）** — 权威登记册（`docs/superpowers/specs/semantic-universe.md`）：逐条登记「语义集条目 × 实现状态（支持/部分/未支持）× 诊断行为」。新增语法必须先登记，漂移即门禁失败（双向断言）。
- **语义集（semantic set）** — 前端功能全集，与实现无关；**已实现集 / 白名单（whitelist）** = 编译器当前实现子集。语义集 ⊇ 白名单，白名单外条目必有「未支持」状态。
- **P3 机制** — 语法表面只随语义集变化；实现进度只改状态列。白名单不再随实现修正漂移（19 属性白名单教训）。
- **语法表面（syntax surface）** — BNF 接受的语法形态，语义集的投影。
- **登记册（vocabulary registry）** — 词汇来源登记（`docs/superpowers/specs/vocabulary-registry.md`）：语义词汇 ↔ 来源域 ↔ 排除词汇。「词从哪来」由登记册管，「功能有哪些、支持哪些」由语义全集管。
- **语义词汇（semantic vocabulary）** — DSL 理解的词汇本体；平台形状名（onXxx 拼写、Flutter 旗标名、机制词如 transform/transition）是**排除词汇**。
- **借词（borrowed word）** — 借用平台名当语义词汇的违规（React 合成事件名、Flutter 旗标名不得直接成为语义词汇）；a11y 域教训（role="textField" 泄漏）。
- **a11y 语义角色（semantic role）** — 无障碍语义角色词汇（WAI-ARIA canonical 名，Flutter 可映射）；裸 aria-* 属性是已删除形态。登记册独有封闭全集（= 已实现集，无漂移可能；设计文档域 10）。
- **保留名（reserved names）** — 指令生成信号的保留命名：@load 存在时 loading/error/data 保留；@form 存在时 formErrors/formValid/formSubmitting 保留。冲突 = **保留名冲突**硬诊断。
- **规则白名单（rule whitelist）** — 封闭的验证规则集（validate: 的 required/format/min/max/pattern/custom），白名单外规则硬错误。封闭集扩充须先登记。
- **floor 门（floor gate）** — 语言语义构造缺目标支持时的 E_CODEGEN 硬错误（语言方法层操作缺后端实现时禁止 verbatim 落码）。
- **门禁（gate）** — CI 验证关卡：verify-vocabulary / verify-semantic-universe / verify-docs（docs-gate）/ verify-react / WASM 产物新鲜度等。「红门」是门禁失败（红）状态的俗称（「红门归零」= 全绿）——正式产出用**门禁**。
- **双向断言** — 登记册 ↔ 编译器常量（rust_str_array 提取）的交叉断言脚本；漂移即门禁失败，强制「先登记后实现」。

## 状态、绑定与数据

- **绑定（binding）** — 声明式绑定：**双向绑定**（bind:value / bind:checked，控件状态 ↔ 信号双向一致）与**语义绑定**（focus= / scroll-x= / scroll-y=，信号驱动原生 DOM 副作用）。
- **RefOp** — 元素命令式操作语义域：一次性动作（focus/blur/scrollTo/scrollBy/measure/select/setSelection/value/capturePointer/releasePointer，10 操作）。命令式写已删除（ref.value = x → 错误「用 bind:value」）。
- **直写通道（direct-write channel）** — 高频属性更新的目标地道实现：React useRef+DOM 直写（零重渲染）/ Flutter ValueNotifier（零整树重建）/ OmniView el.style.setProperty。
- **设计 token（design token）** — 类型化设计值：@themes { } 声明主题名枚举、@theme 标记当前主题信号、token 类型（color/size/radius/fontFamily/fontSize/fontWeight/lineHeight/letterSpacing/shadow/duration/easing/opacity/zIndex，13 类）。
- **表单意图（form intent）** — @form 指令的语义：字段级 validate: 规则、提交校验、生成错误信号、aria 接线（Flutter 例外走原生 errorText）。
- **数据三态意图（data three-state intent）** — @load 指令的语义：视图级取数指令，自动生成 loading/error/data 三态信号 + retry()，用户显式渲染三态；增量封装，不替代既有 async action。
- **数据三态（data three-state）** — loading / error / data（@load 生成）。与「值三态」（静态文本 / {expr} 动态 / 裸布尔，ADR-068）同名不同义——裸「三态」不在正式产出中使用。

## 编辑器（Poiesis）

- **Poiesis** — 可视化编辑器应用（Tauri + React）：设计画布 + CodeFlow + 代码编辑器 + 调试窗。
- **CodeFlow** — 画布式代码编辑器（ReactFlow），以节点编排 OmniLang 函数体，编译回 `.omni` 源码。
- **节点（node）** — CodeFlow 的编排单元：原始 4 类（fn / code-block / condition / return）+ 语句级语法节点（let / assign / store-call / call）+ try 节点（catch/finally）+ 生命周期节点（@mount/@effect/@unmount）+ action 节点。
- **公式框（formula box）** — 画布内表达式编辑形态。
- **分组（group）** — 画布节点的编组概念：成员须同属一个 fn、组有包围盒与颜色（8 色）、可折叠（组级折叠，折叠态连线重路由）。规范用词：概念**分组**、UI 组件**分组框（GroupBox）**、数据记录**组记录**——口语「组」不在正式产出中指代。
- **四链同步** — 编辑器内事件意图化的四个同步面：parser-view 归一化、source-emitter 映射、EVENT_GROUPS、UNIVERSAL_EVENTS；一处漂移即事件名脱节。
- **设计画布（design canvas）** — 可视化设计界面（view 结构编辑，区别于 CodeFlow 的逻辑代码编辑）。
- **调试窗（debug window）** — 运行编译产物验证真机的窗口；**直接载入（direct load）** = 直接加载编译输出，沙箱桥已移除（ADR-072）。
- **lib 文件** — 库文件（imports/structs/types）在编辑器内的常驻面板与保存全链路。

## 实证与质量

- **learn-omniview** — 演示/学习应用：语义化的**实证载体**——真实使用语料驱动哪些构造被语义化（取向 B），也是语义化即切换的迁移对象（零旧名实证）。
- **注入语料（injected corpus）** — 断言诊断消息逐字文本的测试语料；安全注入语料 855 元组（19 载荷 × 9 插入点 × 5 目标 + 45 控制编译），模式缺失断言（非仅可编译）。
- **设计覆盖（design coverage）** — 评估语义级 DSL 对「前端功能该怎么设计」覆盖度的方法论：机制级（达成）/ 机制残留（收敛中）/ 功能设计级（逐域补齐）。

---

## 用词约定（2026-08-08 定，/domain-modeling 首轮）

1. **「域」编号以语义全集为准**（域 1-7 + 域 8 表单/域 9 数据三态）；引用设计文档写「设计文档域 N」（10 域编号，域 1 RefOp 诊断表…域 10 a11y），引用登记册写「词汇域 N」（5 封闭词汇域）。阶段 2 的「域 A」= 全集域 8 + 域 9。
2. **指令与意图分层**：指令 = DSL 表面（`@` 构造），意图 = 编译器 IR 语义（IntentIR）。「事件意图化」= 事件名语义化为意图。
3. **三态消歧**：数据三态（loading/error/data）与值三态（静态文本 / {expr} / 裸布尔）规范用词各归各，裸「三态」不用。
4. **目标（target）为规范词**；「后端」为旧称，仅代码标识（Cargo feature 名）可保留。
5. **设计文档域对照表**（引用设计文档一律写「设计文档域 N」；全集域为规范编号）：

| 设计文档域 | 名称 | 语义全集对应 |
|---|---|---|
| 1 | Ref 系统 | 域 4 RefOp + 域 6 bind:value + 域 7 成员（ref/focus/scroll-*/style） |
| 2 | 动画系统 | 域 1 motion + 触发器（未登记） |
| 3 | Design Tokens + 主题 | 域 5 token 类型 |
| 4 | Slots | 无（未登记） |
| 5 | Stores | 无（机制级，非语义词汇） |
| 6 | Control Flow | 无（@if/@for/@empty，机制级） |
| 7 | Events | 域 3（1:1） |
| 8 | Expressions & Statements | 域 2（数组方法语言层）+ 表达式基座 |
| 9 | Styling | 域 7 成员（style 动态绑定/布尔属性）+ 值三态 |
| 10 | a11y | 无（登记册独有 a11y role） |

> 两套编号是不同分组维度：设计文档按**功能系统**（Ref/动画/Store…），全集按**语义词汇类别**（可动画属性/RefOp/指令…）。仅 Events 是 1:1。另：855 元组指注入语料（非验证规则数）。
