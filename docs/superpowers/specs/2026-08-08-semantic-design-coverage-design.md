# 语义级 DSL 设计覆盖补全设计

**日期**: 2026-08-08
**状态**: 待审阅
**上游**: ADR-052/053/054/056/062/067/071/073/074（语义 DSL 时代决策链）
**配套计划**: 待 writing-plans 生成

---

## 1. 背景与动机

用户要求：**编译器通过语义级语法理解「前端功能该怎么设计」，就像理解「该怎么实现」一样，然后针对编译目标（React / Flutter 生产级，OmniView 调试级）生成地道生产代码。**

2026-08-08 实证（三探索 agent × 源码/文档/ADR 全链审计）结论：

| 层 | 现状 | 证据 |
|---|---|---|
| 机制级意图 → 地道实现 | ✅ 已达成 | @drag 三目标选路（React 4 pointer props 合成 / Flutter GestureDetector / OmniView capturePointer）、@ref 直写（React useRef 零重渲染 / Flutter ValueNotifier）、store（Zustand slice / Riverpod Notifier）、生成代码过 tsc+dart analyze+浏览器三门禁，React codegen 65-70% 意图决策 |
| 机制残留半壁 | ⚠️ 透传非理解 | 54 个 DOM 事件名、DOM 形状事件字段（`e.x→clientX`）、任意 CSS 属性/伪类/`var()`、ARIA 值域直通——Flutter 端靠诊断兜底，跨目标可移植性在这半壁失效 |
| 功能设计级 | ❌ 零词汇 | 表单校验、路由、数据获取三态、modal/toast、tabs、响应式断点、焦点管理、i18n、拖拽排序全部无语言词汇；learn-omniview 靠手写组件（Modal/NotificationsPanel/ValidationInput）人肉拼装 |

**用户决策（2026-08-08 拍板）**：
- 补缺方向：**方向 3 分阶段全补**（先机制级事件意图化，再功能设计级）
- 事件意图化取向：**取向 B 增量登记**（首批 14 个语义事件，登记册逐事件标注状态，随语料需求渐进上移）
- 新原则：**语义化即切换，不留旧残留**（见 §2 原则 8）
- 阶段二范围：**四组全做**（表单+数据三态 / 反馈+模态 / 导航 / 响应式断点）
- 实证载体：**重构 learn-omniview/src**（102 个 .omni 全量，见 §6）

## 2. 设计原则

1. **语义原语优先**（ADR-054）：语言表达「做什么」，平台映射是 codegen 职责
2. **编译器责任模式**（ADR-071）：源码表达意图，编译器选路实现——可行性由语义完备性决定
3. **语义域登记先行**（ADR-074）：新增语法先登记 semantic-universe → 才可进 BNF/文档/白名单；门禁双向断言
4. **诚实降级 > 静默错误**（ADR-054）：无法表达 → 硬诊断/警告/文档如实记录，绝不静默
5. **官方 API 优先**（ADR-052）：库选择 = 官方/标准 → 热门社区 → 删除
6. **高频 = 简单直接**（ADR-070）：高频操作用底层原语，编译期无壳
7. **YAGNI**：只加有真实语料需求的能力；未覆盖面记录「语言边界」
8. **语义化即切换，不留旧残留**（2026-08-08 新增）：一旦构造进入语义层，旧形态不留透传共存——旧 DOM 事件名/字段名产生硬诊断 + 迁移提示（capturePointer→@drag 先例），demo 与文档一次性迁移，登记册状态同步切换

## 3. 阶段 1：事件意图化

### 3.1 首批 14 个语义事件名（9 个语义族）

选择标准：① 跨目标痛点最高（Flutter 诊断兜底重灾区）② 阶段 2 前置（表单/导航/焦点建立其上）③ 有明确语义可定义

语义族：press / input / change / submit / key（down+up）/ pointer（down/move/up）/ scroll / focus+blur / enter+leave

| 语义事件 | 语义（编译器理解的意图） | 字段 schema | 三目标实现方向 |
|---|---|---|---|
| `@onPress` | 点按完成（press/tap 别名收编） | `{x, y}` 元素坐标 | React onClick / Flutter InkWell.onTap / OmniView click |
| `@onInput` | 控件值变化（输入中） | `{value, checked}` 当前值 | React onChange / Flutter onChanged / OmniView input |
| `@onChange` | 选值变化（select/checkbox/radio） | `{value, checked}` | React onChange / Flutter onChanged / OmniView change |
| `@onSubmit` | 表单/输入提交 | `{values}`（`{string: string}`，FormData 收集） | React onSubmit / Flutter onSubmitted / OmniView submit |
| `@onKeyDown` | 按键按下 | `{key: KeyName, code, ctrl, shift, alt, meta}` | React onKeyDown / Flutter KeyEvent / OmniView keydown |
| `@onKeyUp` | 按键抬起 | 同上 | 同上 |
| `@onPointerDown/Move/Up` | 指针原始交互（低层；@drag 仍是高层意图） | `{x, y, pointerId, buttons}`（dx/dy 已 YAGNI 移除——@drag 覆盖增量语义，pointer 原始事件增量需跨事件快照状态、零消费者） | React pointer props / Flutter Listener / OmniView pointer 事件 |
| `@onScroll` | 滚动位置/进度变化 | `{x, y, progress}`（progress 0..1：scrollTop/(scrollHeight−clientHeight)，不可测 0 兜底） | React onScroll / Flutter NotificationListener / OmniView scroll |
| `@onFocus` / `@onBlur` | 焦点进出 | `{value}` | React onFocus/onBlur / Flutter FocusNode / OmniView focus/blur |
| `@onEnter` / `@onLeave` | 指针悬停进出 | `{x, y}` | React onMouseEnter/Leave / Flutter MouseRegion / OmniView mouseenter/leave |

> schema 列为语义子集（可读性截断）；完整 schema = 编译器单一事实源（`ir/events.rs` `event_data_schema`，Task 9 核对基准），docs/omniview/07-events-and-bindings.md 详载。坐标基准：PointerEvent `x/y` = 元素局部坐标（offsetX/offsetY，D1）；ScrollEvent `x/y` = scrollLeft/scrollTop。

### 3.2 三个设计决策

**D1 · 坐标基准 = 元素局部坐标**：`x/y` = 相对元素左上角（Flutter `localPosition` 天然一致；React 由 clientX − rect 换算；OmniView 由 offsetX）。`@drag` 的累计位移 schema 不变（拖拽语义的一部分）；pointer 事件的 `dx/dy` = 相对上一帧位移。

**D2 · 语义 key 枚举化**：`key` 字段是语义键名（`"Enter"`/`"Escape"`/`"ArrowUp"`…；19 命名键 + 单字母/单数字，大小写敏感），语义 pass 校验常量合法性 + 近邻建议（`"enter"` 拼错 → E0006 硬错误 + 建议 `"Enter"`，V3 落地）。`e.key` 字符串本身已跨平台一致，价值在验证而非重命名。

**D3 · schema 提取复用既有基建**：每个语义事件定义 schema（编译器单一事实源），codegen 按 `used_fields` 提取精确字段——复用 `@drag` 的 `emit_event_object` 管线（event_data + used_fields 既有机制），零新机制。

### 3.3 一步到位切换（原则 8 落地）

首批 14 个语义事件落地的同时，以下旧名全部切换为硬诊断 + 迁移提示：

| 语义事件（新） | 旧名 → 硬诊断 + 迁移提示 |
|---|---|
| `@onPress` | `@onClick` / `@click` / 裸 `onClick`，**含别名 `@onTap`/`@tap`（canonical 同值 click，Task 2 实现事实）** → E0006 硬错误「事件名 "@x" 已弃用——使用 "@onPress"」。`@press` 是运动触发器指令（TokenKind::AtPress，非事件名）——不进事件校验，无切换语义 |
| `@onInput` | `@onChange` **维持合法**（select/checkbox/radio 选值语义；change 与 input 是不同事件，非旧名关系，无编译器旧名诊断）；文本输入用点迁移至 `@onInput` 属 demo/文档指引（Task 8 语料判别） |
| `@onSubmit` | `e.value` 等旧 DOM 形状字段 → E0006 无效字段硬错误（schema 仅 `values`——FormData 收集） |
| `@onKeyDown/Up` | `e.which` 等非语义字段 → 无效字段硬错误 |
| `@onPointerDown/Move/Up` | `e.clientX/e.clientY/e.pageX/e.offsetX/e.ctrlKey/…` → 无效字段硬错误（clientX/clientY/ctrlKey 带迁移提示「可改用 e.x/e.y（元素局部坐标）」「e.ctrl」）；**dx/dy 不存在**（YAGNI：@drag 覆盖增量语义） |
| `@onScroll` | `e.deltaY`（wheel 语义混用）→ 无效字段硬错误（schema 仅 x/y/progress——scroll 语义为位置/进度非增量；`@wheel` 才有 deltaY） |
| `@onFocus/@onBlur` | 名不变，仅 schema 化 |
| `@onEnter/@onLeave` | `@onMouseEnter`/`@mouseenter`/`@onMouseLeave`/`@mouseleave` → E0006 硬错误「事件名 "@x" 已弃用——使用 "@onEnter"/"@onLeave"」 |

未被首批覆盖的 40 个 DOM 事件名（54 − 14 已语义化；`load/error/play/ended/copy/paste/dragstart/…`）维持透传 + Flutter 诊断，semantic-universe 域 3 逐事件标注「透传」状态；上移一个、切换一个，**不存在新旧双轨期**。

### 3.4 架构影响面

- **AST/parser**：`ElementProp::Event` 扩展语义事件识别 + schema 关联；旧名分派到诊断
- **IR**：事件 schema 常量表（编译器单一事实源，域 3 登记册锚点）
- **semantic**：新 pass `validate_semantic_events`（字段名合法性 + key 枚举 + 旧名迁移提示）
- **codegen 三目标**：按 schema + used_fields 发射；Flutter 端诊断兜底大幅收敛
- **登记册**：semantic-universe 域 3 事件行重分类（语义化/透传）
- **demo/文档**：learn-omniview 事件用点一次性迁移（§6 重构载体）；docs 16 篇同步（§7）

### 3.5 Validation 清单

| # | 检查 | 级别 |
|---|---|---|
| V1 | 语义事件名合法（白名单） | 解析错误 |
| V2 | 语义事件字段名 ∈ schema | 硬错误 |
| V3 | `key` 字段常量 ∈ 语义键枚举 | 硬错误（近邻建议） |
| V4 | 旧名（click/mouseenter/…）→ 迁移提示 | 硬错误 |
| V5 | DOM 形状字段名（clientX/ctrlKey/…）→ 迁移提示 | 硬错误 |
| V6 | 语义事件 + 字段类型检查（value 类型与控件匹配） | 警告（v0 范围） |

## 4. 阶段 2：功能设计级语义域

每域落地流程（@drag 模式，ADR-073 先例）：意图语义定义 → Validation 清单 → 三目标地道实现 → semantic-universe 新域登记 → BNF/文档 → 门禁 → demo 迁移。以下为**意图契约草案**（实现细节归实施计划）。

### 4.1 域 A：表单 + 数据三态

**A1 表单语义**
- 意图：「这是一个表单——字段有校验规则、提交时收集值并执行提交、错误显示在字段旁」
- 语法草案：`<form @form @onSubmit={fn}>` + 字段校验声明（内联规则 map 或声明式块，形态实施计划细化）；校验规则：`required / format / min / max / pattern / custom fn`
- 编译器生成：字段值信号（bind:value 接线）、错误信号 map、提交时校验执行、错误展示接线（字段旁容器）、a11y（aria-invalid/aria-describedby）、禁用提交态
- 三目标：React 受控 + 校验逻辑 / Flutter `Form`+`TextFormField.validator`（地道原生）/ OmniView 原生 HTML5 校验 + 自绘错误

**A2 数据三态**
- 意图：「这个数据源是异步的——加载/错误/数据三态由编译器管理」
- 语法草案：`@load` 指令（绑定 fn，自动生成 `loading/error/data` 三态信号 + 重试）；形态与绑定方式实施计划细化
- 三目标：React `useEffect`+三态 state（或库）/ Flutter `FutureProvider`（Riverpod 3.0 既有依赖）/ OmniView 自绘
- 决策点：`@load` 与既有 `async action + await` 的关系（三态封装是增量，不替代既有原语）

#### 实施收口（2026-08-09，阶段 2 域 A Task 7——实施完成，形态拍板）

| # | 决策点 | 收口决策 |
|---|---|---|
| 1 | 字段校验声明形态 | **字段级 `validate: 规则` 属性**（`<input validate: required>`；规则闭集 6 名 = `VALIDATE_RULES`：required/format/min/max/pattern/custom；format 闭集名字分派 email/url，其余值跳过 → 需 custom） |
| 2 | 错误展示接线 | **错误信号显式渲染**（`form.errors` + `@if form.errors.name` 用户显式渲染，React/OmniView）；**Flutter 例外**：错误经 `TextFormField.errorText` 自动展示，`form.errors` 发射为常量空 map——语言面跨目标一致保留、行为注记（semantic-universe 域 8 诊断行为列） |
| 3 | `@load` 形态 | **视图级指令**：`@load loadData` 绑定组件 fn，mount 后自动装载；生成 `loading/error/data` 三信号 + `retry` 重试；至多一个 `@load` 每组件（与 `@form` 同） |
| 4 | v0 范围边界 | `@load` **仅 Array 返回**（`{T}[]` 三目标形态）；`form.submitting` 恒 false（三目标声明但从不写入，语义 v0 未实现）；custom 仅组件 fn 名（无字面量自定义消息）；Flutter 无 submit 按钮场景 handler 无门禁直发（v0 边界） |
| 5 | 语义验证 | 保留名冲突硬错误（formErrors/formValid/formSubmitting、loading/error/data/retry——`validate_form_load.rs` 保留名检查）；`@form`/`@load` 重复硬错误 |

> 注：A1/A2 语法草案中的「内联规则 map 或声明式块」「绑定方式」等未决形态均以上表 1/3 行收口；三目标实现细节见 Task 4/5/6 报告（react/component、flutter、omniview codegen）。

### 4.2 域 B：反馈 + 模态

**B1 toast/通知**
- 意图：「显示一条短暂的通知（kind + duration + 自动消失）」
- 语法草案：内建语义函数 `showToast(msg, {kind: "info"|"success"|"error", duration})`（编译器识别，替代手写 NotificationStore）
- 三目标：React 自实现轻量 toast（无事实标准库）/ Flutter `ScaffoldMessenger.showSnackBar`（地道原生）/ OmniView 自绘
- 决策点：容器样式可定制性（默认样式 vs 设计 token 接入）

**B2 modal/dialog**
- 意图：「这个覆盖层是模态对话框——焦点陷阱、Esc 关闭、遮罩点击关闭、内容投影」
- 语法草案：`<dialog @modal={openSig}>…</dialog>`（元素指令，投影内容走既有 @slot）
- 编译器生成：覆盖层、焦点陷阱（Tab 循环）、Esc 关闭、a11y（role=dialog/aria-modal/aria-labelledby）、进出场动画（@enter 复用）
- 三目标：React 原生 `<dialog>`+`showModal()`（更地道）/ Flutter `showDialog`+`FocusTraversalGroup` / OmniView 自绘 + 焦点管理
- Validation：@modal 值必须为信号；重复 @modal 错误；dialog 语义与 @for 组合规则

### 4.3 域 C：导航/路由

- 意图：「页面之间切换，带参数，可返回」
- 语法草案：页面组件 `@route "/todos/:id"` 声明 + 内建 `navigate(path, {params})` / `back()` 语义函数；参数经 prop 注入
- 三目标：React React Router（事实标准）/ Flutter `Navigator`（官方）/ OmniView hash 路由自绘
- 决策点：URL 深链跨目标差异大——行为对等 floor 界定（页面切换/参数/返回为 floor 内，深链/URL 同步为各目标差异 + 诚实降级注记）；这是四域中跨目标差异最大的域，floor 界定是设计核心

### 4.4 域 D：响应式断点

- 意图：「布局在断点处适应」
- 语法草案：design 域新增 breakpoint token（`breakpoint md: 768`）+ 使用形态 `@if bp("md")` 条件表达式（或变体维度，实施计划细化）
- 三目标：React `matchMedia` hook / Flutter `MediaQuery`/`LayoutBuilder` / OmniView `matchMedia`
- Validation：breakpoint 名必须声明；未声明引用 → 硬错误 + 建议

## 5. 阶段间依赖与顺序

```
阶段 1（事件意图化）──────────────┐
  前置：无（独立单元，先行实施）    │
  阶段 2 域 A/C/D 建立其上          ├──→ 阶段 3（重构 learn-omniview）
  阶段 2 域 B 依赖 @slot（已有）   │     覆盖验证 + demo 迁移 + 缺漏迭代
  阶段 2 域 C 依赖 @route 声明      │          │
  阶段 2 域 D 依赖 design 域扩展    │          ▼
──────────────────────────────────┘   阶段 4（重构 docs/omniview）
```

阶段 2 四域按优先级实施：**表单+数据三态 → 反馈+模态 → 导航 → 响应式**（表单最高频且直接建在阶段 1 之上；导航跨目标差异最大后置细究）。

## 6. 阶段 3：重构 learn-omniview/src（实证载体）

- 载体：`test/projects/learn-omniview/src`（102 个 .omni，9 标签页，20 pages + 56 components + 4 stores + 18 lib + 4 root，Task 9 实测计数）
- 目标：用新语法全量重构——① 覆盖验证（新语义事件/新功能域的真实使用）② demo 迁移（一步到位切换纪律）③ 缺漏迭代（重构中写不出来的界面 = 缺漏 → 追加设计或明确语言边界）
- 已知重构点（实证预判）：
  - Modal 组件 → `@modal` 原语
  - NotificationsPanel/NotificationStore → `showToast` 内建
  - ValidationInput（名为校验实为关键字 demo）→ 真实表单语义
  - PageSwitcherDemo（条件渲染模拟页签）→ 导航原语（若优先级到达）
  - 全部事件用点 → 语义事件名 + 语义字段
- 门禁：verify-demo-coverage.py（域 8-11 新域审计接入）、verify-semantic-universe.py（新域断言）、三目标编译 + 浏览器全量

## 7. 阶段 4：重构 docs/omniview 语法文档

- 载体：`docs/omniview/` 16 篇 + README + BNF（14/15-grammar）
- 同步内容：事件域（语义事件 + 字段 schema + 迁移对照表）、功能设计级域（表单/数据三态/反馈/模态/导航/响应式各新节）
- 门禁：docs-gate（verify-docs.py 248 blocks 0 failed）、verify 双脚本锚点同步

## 8. 验证与门禁

| 门禁 | 要求 |
|---|---|
| cargo workspace | 全绿（含新 pass/新测试） |
| React verify-react | GATE PASSED，豁免不新增 |
| Flutter dart analyze | warnings ≤ 基线（新语法引入的诚实降级需注记） |
| 浏览器 Playwright | 46+ 全量 + 新语义事件行为断言（press/keydown/scroll 各目标） |
| docs-gate | 248+ blocks 0 failed |
| verify-semantic-universe.py | 新域（8-12）断言 0 problem |
| verify-demo-coverage.py | 新域 usage 审计接入 |
| 注入语料 | 新事件/新域插入点进 855 元组矩阵 |

测试策略：@drag 先例（RED→GREEN、组合 fixture、浏览器行为断言）；每个新语义事件的「旧名切换」有正/负向测试（旧名编译错误 + 迁移提示文本断言）。

## 9. 治理

- 每阶段收口 ADR（阶段 1 → 1 个 ADR；阶段 2 每域 1 个 ADR 或合并，沿 ADR-056/062 先例）
- semantic-universe 新域登记（域 8 表单/9 数据三态/10 反馈模态/11 导航/12 响应式——编号实施时定）
- deferrals 更新：关闭 #85（若 i18n 动态 label 未随行，更新触发条件）；#81/#83 观察清单
- 语言边界记录：未被四组覆盖的实证卡点 → 明确「边界 + 触发条件」（YAGNI）

## 10. 验收标准

- [ ] 阶段 1：14 语义事件 + 旧名切换全落地，learn-omniview 事件用点零旧名
- [ ] 阶段 2：四域意图契约实现（各带 Validation 清单 + 三目标门禁）
- [ ] 阶段 3：learn-omniview 全量重构完成，覆盖审计新域缺口 = 0，重构卡点清单闭环（追加设计 or 边界记录）
- [ ] 阶段 4：docs/omniview 全量同步，docs-gate 零失实
- [ ] 全部门禁绿；注入语料扩张无新面
