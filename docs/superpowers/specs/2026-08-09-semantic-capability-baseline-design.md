# 语义级语法能力基线（Semantic Capability Baseline）

**日期**: 2026-08-09
**状态**: 定稿（方向级）
**上游**: ADR-052/053/054/056/062/067/071/073/074/075（语义 DSL 时代决策链）；wayfinder 地图 12/12 车票决议（`.scratch/semantic-capability-baseline/`）
**配套**: 各批次计划待 writing-plans 生成（批次 1-5）

---

## 1. 背景与目的

用户诉求：**语义级语法能否完成真实的复杂项目、实现复杂的功能**——以能力为主、缺口为辅。手写体感不重要（poiesis 是作者——可视化编辑器承载编写体验），语法可表达性才是范围。poiesis 用户的能力天花板 = 语义语法的表达能力上限。

本 spec 定义「真实复杂前端项目」的能力全景（14 维清单终稿），逐项给出边界裁决（进语言/机制语法面/编译器职责），为进语言项标注设计方向，并按能力批次切分实施路线图。它是语义全覆盖收官（ROADMAP 1B-1F 重排依据）与冻结 ADR 的前置文档。

**核心哲学（2026-08-09 用户校准）**：
> **前端功能是跨框架通用的**。语言表达功能意图是默认；编译器为每个目标（React/Flutter/OmniView）找到地道映射是它的职责。「一端实现不了」是编译器的映射缺口（设计任务），不是把功能排除出语言的理由。编译器要生成「真人写这个功能时会写的代码」。

## 2. 能力矩阵（14 维终稿）

状态图例：✅ 已支持 / 🔜 进语言（方向已定或强候选）/ ⏳ 候选（批次内细化）/ 🏞 机制语法面·编译器职责·后议

| 维度 | 能力项（状态） |
|---|---|
| 1 状态管理 | signal/getter/store/props·emit·slot/持久化（✅）；深层嵌套/不可变更新（✅） |
| 2 异步与数据 | @load 三态（✅）；async·await/并行·竞态·超时（✅ std/async）；**@load 深化：多例/参数/Reset/分页/轮询/无限滚动触发（🔜 批次 4）**；乐观更新（⏳ 写路径语义化后）；防抖节流（🔜 stdlib）；缓存策略（🏞 配置面，失效意图 retry 已覆盖） |
| 3 导航路由 | **页面切换/参数/返回/深链/守卫/嵌套/URL 查询参数（🔜 批次 2 域 C，全量进 floor）**；路由懒加载（🔜 批次 2，编译器职责优先） |
| 4 表单 | @form+validate 六规则/错误信号（✅）；动态字段/多步骤/联动/自动保存（🏞 语言边界待批次——用户拍板与当前目标关联弱，先成熟再议） |
| 5 渲染组合 | @if/@for+key/@slot/组件嵌套（✅）；骨架屏（⏳ @load loading 形态扩展）；空状态（🏞 @if 已表达）；虚拟化（🏞 后议——用户拍板先成熟再谈）；错误边界（🔜 批次 1 翻案定：进语言带降级） |
| 6 交互 | 语义事件 54 名（✅）；@drag 位移（✅）；**拖拽重排/基本手势/剪贴板/快捷键（🔜 批次 5 交互归组）**；焦点陷阱（🔜 随 @modal）；系统级文件拖放/图片剪贴板（🏞 随剪贴板批次评估） |
| 7 响应式 | **bp("md") 逻辑断点 + breakpoint token（🔜 批次 3）**；CSS 布局（✅）；用户级 @media（🏞 机制语法面透传 + Flutter 映射策略批次 3 细化）；容器查询（⏳ 低优先候选） |
| 8 反馈与模态 | **@modal 元素指令 + showToast 内建函数（🔜 批次 1）**；motion 动画（✅）；tooltip（🔜 低优先——属性级指令，双端映射存在）；错误边界（🔜 批次 1） |
| 9 国际化 | **@t 文本翻译（ICU 消息语法子集）+ @locale 语言切换（🔜 批次 5 i18n 归组）**；日期数字格式化（🔜 stdlib 最简形态；参数面留生态） |
| 10 无障碍 | a11y 块 label/role/live（✅ 部分：动态值 defer #85）；屏幕朗读语义（✅ 归属 a11y 维） |
| 11 性能 | computed/信号批处理（✅）；组件 memo（🏞 编译器职责——React Compiler 先例）；懒加载（🏞 编译器职责优先 + 路由懒加载随域 C） |
| 12 生命周期 | @mount/@effect/@unmount（✅ 未入册，1B 项）；**@effect 强化：定时器/订阅清理 + @store/@mount/@effect 入册（🔜 批次 5 生命周期归组）** |
| 13 媒体与文件 | img/audio/video（✅）；canvas/文件上传/摄像头（🔜 低优先——媒体归组，权限模型批次 spec 细化） |
| 14 样式 | 类/标签/伪类/复合/后代选择器（✅）；CSS 变量/token/主题/variant（✅）；**深色模式（✅ design @themes 已支持）**；内联+动态样式（✅）；复杂选择器（🏞 机制语法面透传 + **静默跳过缺陷修复（🔨 批次 3，诊断先行）**）；@media/容器查询（🏞 透传）；样式作用域（✅ 组件级） |

## 3. 边界裁决（三判据 + 真人写法标准）

### 3.1 裁决标准（2026-08-09 校准后）

| 判据 | 口径 |
|---|---|
| 跨目标一致表达 | 意图能否在三目标找到**地道映射路径**（含编译器选路 + 诚实降级注记）——映射困难是设计任务，不是排除理由。「Flutter 实现不了」前提证伪：深链（GoRouter Deep linking + App Links + web URL）、守卫（GoRouter redirect）、嵌套（ShellRoute）、返回（Navigator 栈）——映射全部存在 |
| 编译器理解意图 | 编译器能否识别为语义构造（机制级 @drag / 功能设计级 @form/@load 先例）而非透传——透传不算理解（40 透传事件先例） |
| 真实语料需求 | **降级为优先级判据**（决定批次顺序），不否决进语言——零语料 = 排期后置 + 记录边界，不是排除 |

**「真人写法」测试**（错误边界裁决时确立）：编译器要生成「真人写这个功能时会写的代码」——React ErrorBoundary / Flutter 数据级错误路径都是真人的真实写法；一端无地道形态 → 进语言带诚实降级，而非排除。已实现域（@form/@load/@drag/@ref/事件）生成形态即「地道代码」事实基准（audit 见 `.scratch/semantic-capability-baseline/research/04-design-direction-audit.md`）。

### 3.2 裁决模板（所有批次共用）

```
能力项 | 跨目标一致（✓/✗ + 证据） | 编译器理解（✓/✗ + 证据） | 语料（✓/✗ + 语料位，影响排期）
→ 结论：进语言 / 机制语法面 / 编译器职责 / 后议
→ 设计方向（1-2 句，仅进语言项）
→ 触发条件（仅非进语言项——语料或环境变化时重评）
```

### 3.3 语言边界收窄

「留生态」类别收窄为三类（其余全部进语言）：
1. **机制语法面**：目标专属表达通道（CSS 选择器/@media——web 表达方式；Flutter 以组件模型/逻辑断点表达同等意图——表达通道差异，非功能缺失）
2. **编译器职责**：用户不写意图的优化行为（memo——React Compiler 先例；懒加载以编译器策略生成）
3. **后议项**：用户拍板暂缓（虚拟化——先成熟再谈；表单深化——与当前目标关联弱）

## 4. 设计方向（进语言项，方向级）

### 批次 1 — 反馈与模态
- **@modal**：`<dialog @modal={openSig}>…</dialog>` 元素指令 + @slot 投影 → React 原生 `<dialog>`（受控 `open` prop 形态——audit 细化点 1）/ Flutter `showDialog`+FocusTraversalGroup / OmniView 自绘 + 焦点管理；**焦点陷阱/Esc/遮罩点击三要素编译器内建**（双端原生零手写）
- **showToast**：`showToast(msg, {kind, duration})` 内建语义函数 → React 编译器自生成轻量 toast（零依赖——audit 细化点 3）/ Flutter `ScaffoldMessenger.showSnackBar` / OmniView 自绘
- 骨架屏：@load loading 形态扩展（不独立成项）
- 错误边界：意图「子树渲染失败 → 降级 UI」→ React ErrorBoundary / Flutter 数据级错误（@load error 信号）+ 全局 ErrorWidget 兜底注记 / OmniView try 兜底

### 批次 2 — 导航路由（域 C，全量进 floor）
- `@route "/todos/:id"` 页面组件声明（守卫/嵌套/lazy 属性扩展点）+ `navigate(path, {params})` / `back()` 内建语义函数 + 参数 prop 注入
- 三目标：React Router（**库绑定与依赖注入——audit 细化点 2**，Zustand 先例）/ Flutter GoRouter（官方推荐；深链 App Links + web URL 同步）/ OmniView hash 路由
- floor：全部意图 floor 内（行为对等 + 平台差异诚实降级注记——移动端深链形态差异）
- 嵌套路由 = **独立 layout 语法**（React Router `<Outlet/>` / GoRouter ShellRoute / OmniView 嵌套渲染）

### 批次 3 — 响应式 + 样式修复
- **bp() 逻辑断点**：design 域 `breakpoint md: 768` token + `@if bp("md")` 条件表达式 → React useMediaQuery/matchMedia / Flutter MediaQuery / OmniView matchMedia
- **选择器静默跳过修复**（真实缺陷，`parser/view/style.rs` ~:166）：未支持选择器部分（`input:checked + span`/兄弟/属性/:nth-child）→ **warning 硬诊断**（含选择器文本 + 截断说明）——诚实降级（ADR-054），绝不静默
- @media 用户级：CSS 透传（web 通道）+ Flutter 映射策略（MediaQuery 分支）批次 spec 细化
- 容器查询：低优先候选（web @container / Flutter LayoutBuilder——形态批次 spec 细化）

### 批次 4 — 数据能力（@load 深化）
- @load v0 边界补齐：**多例（每组件多 @load）/ 参数 / Reset**——**DataFetching 迁移定案：@load 扩展后迁移**（此前未决项关闭；双取数/Reset 意图经扩展语义表达）
- **分页**：@load 分页扩展（page/cursor 参数 + 页码状态机）——库绑定（TanStack Query/infinite_scroll_pagination 风格）vs 编译器自生成，批次 spec 定
- **无限滚动**：装载触发语义（@load scroll 触发 / @for 容器滚动接近底部）→ IntersectionObserver / ScrollController 阈值
- **轮询**：@load interval 参数 → refetchInterval / Timer.periodic
- 乐观更新：@load 写路径 mutate（远期——写路径语义化后）
- 防抖节流：stdlib 语言函数 `debounce(fn, ms)` / `throttle(fn, ms)`（时间语义跨平台一致）

### 批次 5 — 长尾四归组（互相独立，可并行）
- **交互**：拖拽重排（@for reorder / @reorder → dnd-kit / ReorderableListView）、基本手势（@swipe/@longPress/@pinch → GestureDetector 双端）、剪贴板（@copy 意图 → Clipboard API / services）、快捷键（@key 组合 → React keydown / Flutter Shortcuts+Actions）
- **i18n**：@t 文本翻译（**ICU 消息语法子集——三端同基座**：react-intl / vue-i18n / gen_l10n+ARB）+ @locale 语言切换 + formatDate(locale) stdlib 最简形态（参数面留生态）
- **媒体**：canvas（命令式绘制意图——RefOp 扩展或元素指令 → canvas 2D / CustomPaint）、文件上传（input[type=file] / file_picker）、摄像头录音（mediaDevices / camera 插件——权限模型批次 spec 细化）
- **生命周期**：@effect 强化（定时器/订阅清理 → useEffect cleanup / State.dispose）+ **@store/@mount/@effect 入册**（1B 项并入本归组）

## 5. 批次路线图

```
批次 1  反馈模态（@modal/showToast/骨架屏形态/错误边界）── 语料在手（手写 NotificationStore/Modal 先例）
批次 2  域 C 导航路由（@route/navigate/back/守卫/嵌套/深链/查询参数）── 语料在手（PageSwitcher 手写模拟）
批次 3  响应式+样式修复（bp()/breakpoint token/选择器诊断/@media 映射策略）── 小批次合并
批次 4  数据能力（@load 深化/分页/无限滚动/轮询/DataFetching 迁移/防抖节流 stdlib）
批次 5  长尾四归组（交互/i18n/媒体/生命周期——互相独立可并行）
```

每批次独立走 **spec → plan → subagent-driven**（writing-plans + SDD），检查点不跳（可验证产物 + 门禁绿）。

**与 ROADMAP 1B-1F 的关系**：本路线图吸收重排——1C 域 B（反馈模态）→ 批次 1；1D 域 C（导航路由）→ 批次 2；1E 域 D（响应式）→ 批次 3；1B 机制级登记（@store/@mount/@effect 入册）→ 批次 5 生命周期归组；1A 域 A 已收口完成；**1F 已知缺陷保持独立检查点**（IR type_ann / 跨文件验证空洞 / cache 性能——类型检查深化已裁归 1F）。

## 6. 验收（矩阵全绿 + 基准应用）

1. **能力矩阵全绿**：清单 14 维所有 🔜 项实施完成（三目标 + 五层门禁：tsc/dart analyze/Playwright/docs-gate/注入语料）
2. **阶段 2 基准应用**：poiesis 全流程（编辑 + 实时预览 + 编译运行，单目标）实现一个中等复杂度应用——覆盖域 A（表单/校验/数据）+ 模态（批次 1）+ 路由（批次 2）；具体应用选择在基线定稿后定义（ROADMAP 决策 14）

## 7. 冻结衔接（收官冻结）

基线 spec 定稿（本文件，含冻结清单草案）→ 各批次实施（批次语法变更照常走 ADR + 同 commit 全生态迁移——ROADMAP 硬规则）→ 全部实施完 = **语义全覆盖收官** → 冻结 ADR 定稿（收官宣言 + 语法面冻结清单 + 硬规则生效）。

**冻结清单草案**（收官时定稿）：
- 语法面：本 spec 能力矩阵全量（✅ + 🔜 项全部）为冻结面；🏞 机制语法面/编译器职责/后议项记录为语言边界，不入冻结承诺
- 硬规则：语法/语义变更 = ADR + 同 commit 全生态迁移（编译器 + poiesis + 文档 + registry + 门禁）
- 登记纪律：新增语法先登记 semantic-universe → 才进 BNF/白名单（ADR-074）；语义化即切换、不留旧残留（ADR-075 原则 8）

## 8. 治理

- **semantic-universe 扩展**：状态值域新增「留生态」（现：支持/部分/未支持）——表示语义集成员但裁决不进语言（附语言边界注记 + 触发条件 + 目标生态映射指引）；新域登记：域 10 反馈模态 / 11 导航 / 12 响应式 / 13 数据深化（随各批次）
- **登记册**：vocabulary-registry 随新词汇登记（bp/breakpoint/@modal/showToast/navigate/@t/@locale/reorder/swipe 等）
- **验证门禁**：verify-semantic-universe.py / verify-vocabulary.py 改造随各阶段 spec；注入语料扩张无新面
- **半实现信号**：form.submitting 恒 false、Flutter form.valid 恒 true、@load v0 边界——标 v0 桩 + 补齐批次（05 原则），语言面不动
- **语言边界记录**：🏞 项在 semantic-universe 状态列 + 文档注记（含触发条件）

## 9. 与既有文档的关系

- 上游：ADR-052/053/054/056/062/067/071/073/074/075（决策链）；`2026-08-08-semantic-design-coverage-design.md`（域 B/C/D 契约草案——本 spec 的方向级裁决将其细化）；`semantic-universe.md` / `vocabulary-registry.md`（登记扩展）；ROADMAP.md 阶段 1（重排依据）
- 实证：`.scratch/semantic-capability-baseline/`（wayfinder 地图 12/12 车票决议 + 3 份能力全景研究 + 真人写法审计）
- 下一动作：批次 1（反馈模态）writing-plans 生成实施计划
