# OmniLang v0.3 — OmniView 执行计划

> **基于：** `OmniLang_v0.3_设计方案.md` v3.0-final + Council 评审（2026-06-04）+ **第二次 Council 修正（2026-06-05）**  
> **IR 版本：** `0.3`  
> **模式：** 里程碑驱动（非固定周期）——每个 Phase 通过用户验证后才解锁下一 Phase  
> **编制日期：** 2026-06-04 | **修订日期：** 2026-06-05（第二次 Council 修正：委托架构 + 风险 R12-R19 + 时间线调整）

---

## 目录

1. [启动前准备 + IR 原型验证](#一启动前准备--ir-原型验证)
2. [Phase 0：React L2 地道生成（10 周）](#二phase-0react-l2-地道生成10-周含-omnilang-委托基础设施)
3. [Phase 1：Vue 3 L2 地道生成（8 周）](#三phase-1vue-3-l2-地道生成8-周)
4. [Phase 2：Dioxus L2 地道生成（10 周）](#四phase-2dioxus-l2-地道生成10-周)
5. [Phase 2a：Flutter 基础验证（6 周）](#五phase-2aflutter-基础验证6-周)
6. [Phase 3：能力深化 + Flutter L1（10 周）](#六phase-3能力深化--flutter-l110-周)
7. [Phase 4：Flutter L2 + 生态完善（8 周）](#七phase-4flutter-l2--生态完善8-周)
8. [里程碑门控机制](#八里程碑门控机制)
9. [跨 Phase 关注项](#九跨-phase-关注项)
10. [风险登记册](#十风险登记册)
11. [治理机制](#十一治理机制)
12. [附录 A：里程碑交付物清单](#附录-a里程碑交付物清单)
13. [附录 B：技术选型参考](#附录-b技术选型参考)

---

## 一、启动前准备 + IR 原型验证

> **周期：** 2 周（含 1 周 IR 原型验证） | **人员：** 技术负责人 + 核心团队 + 前端架构师  
> **★ 关键调整：** Council 评审指出——在 Intent IR 冻结之前，必须用最难映射的目标框架做最早的原型验证。Flutter `ListView.builder` 和 `TextEditingController` 生命周期是两个金丝雀 case：如果 Intent IR 连这两个都映射不好，整个方案需要重新评估。

### 1.0 架构基础：OmniView→OmniLang 依赖关系定调（第 1 周第 1 天）

> **★ 核心原则：OmniView 不是独立语言——它是 OmniLang 上的 UI 领域 DSL。类比：Dart→Flutter ≈ OmniLang→OmniView。**

#### 依赖定调

| 决定 | 内容 |
|------|------|
| **表达式解析** | 全部由 OmniLang Parser 完成。OmniView 的语法文件不包含 `expr`/`stmt`/`block` 规则——改为嵌入点 `{OMNI_EXPR}` 接入 OmniLang |
| **类型系统** | 复用 OmniLang 的类型系统（`omnilang.semantic.type_checker`）。signal/computed/prop 的类型标注由 OmniLang 检查 |
| **语义分析** | 符号解析、作用域分析、依赖收集全部委托给 `omnilang.semantic`。OmniView 只做 UI 特有的约束校验（P0214 bind 约束、P0201 @for key 等） |
| **表达式 Codegen** | 表达式→目标语言的翻译由 `omnilang/codegen/{target}.py` 完成。OmniView 的 codegen 负责将翻译结果嵌入到 UI 模板中 |
| **诊断框架** | 复用 `omnilang.analyze` 的诊断基础设施（级别、规则注册、修复建议） |
| **CLI** | `omnic` 内部调用 `omnilang` 编译器。用户不会直接运行 `omnilang` 来编译 `.omni` 文件 |

#### 设计红线

1. **OmniView 不重复实现 OmniLang 已有的任何能力。** 表达式解析、类型检查、语义分析全部委托给 OmniLang。
2. **OmniView 的语法文件只包含 UI 结构。** `grammar.lark` 中不得出现 `expr`, `binary_op`, `block`, `stmt` 等逻辑规则。
3. **OmniView 不能脱离 OmniLang 工作。** `.omni` 文件必须经过 OmniLang 编译器处理表达式和类型后才能编译。
4. **Codegen 职责分离。** OmniLang codegen 处理表达式翻译，OmniView codegen 处理 UI 模板展开和惯用法选择。

#### 对当前实现的影响

| 现状 | 目标 | 改造项 |
|------|------|--------|
| `grammar.lark` 包含 expr/block/stmt 规则 (311 规则) | 剥离为仅 UI 结构 (~150 规则) | 表达式/语句规则移除，改为 OmniLang 嵌入点 |
| `OmniViewTransformer` 处理表达式解析 | 表达式交给 `omnilang.parser` | Transformer 中的 expr/stmt 方法移除 |
| `omniview/semantic/analyzer.py` 独立实现 | 委托给 `omnilang.semantic` + 仅保留 UI 约束 | 类型相关逻辑迁移到 OmniLang，保留 P0214/P0201 等 UI 规则 |
| `omniview/codegen/react.py` 包含表达式翻译 | 表达式翻译委托给 `omnilang/codegen/javascript.py` | codegen 中的 `_expr_to_js` 委托给 OmniLang |

### 1.1 IR 原型验证 — 金丝雀测试（第 1 周，必须先于任何 IR 冻结）

> **原则：** 先证明 IR 在最难 case 上可行，再投入 Phase 0 实施。不可行则重新评估方案。

| 验证项 | 输入 | 验证内容 | 通过标准 |
|--------|------|---------|---------|
| **Case A: ListView.builder** | `@for item in items { <ListItem key={item.id}> }` | `ListRenderingIntent` 能否同时驱动 React `.map()` 和 Flutter `ListView.builder`？是否需要 `strategy` 字段？ | React `.map()` + Flutter `ListView.builder` 均生成为可编译代码 |
| **Case B: TextEditingController** | `<Input bind:value={email} />` | `BindIntent` 能否同时驱动 React `value+onChange` 和 Flutter 的 5 步生命周期（StatefulWidget + initState + didUpdateWidget + dispose）？ | Flutter 输出包含 StatefulWidget + TextEditingController + 完整生命周期骨架 |
| **Case C: Flex→Row/Column** | `<Flex direction="row" gap="16px">` | `LayoutIntent` 能否精确映射到 CSS flexbox（React）和 Flutter `Row` widget？ | React flex CSS + Flutter `Row(children: [...])` 正确转换 |
| **Case D: @if 空分支** | `@if cond { <A/> }`（无 @else） | `ConditionalIntent.fallback=None` 在各框架的 null/nop 表示 | React `null` / Flutter `SizedBox.shrink()` / Dioxus `VNode::empty()` |

**产出：** `IR_VALIDATION_REPORT.md` — 每个 case 的 IR 结构草图 + 各框架展开伪代码 + 结论（GO / NEEDS_ADJUSTMENT / BLOCKED）。

**决策点：** 第 1 周结束时，基于验证报告做出二选一决策：
- ✅ **GO**：IR 设计方向正确，进入 Phase 0，IR 核心类型在第 3 周冻结
- ⚠️ **NEEDS_ADJUSTMENT**：IR 需要调整但方向可行——1 周内修正后重新验证
- ❌ **BLOCKED**：IR 方案存在根本性缺陷——回到方案设计阶段，不回修改主文档

### 1.2 IR 预埋字段精简（与 1.1 并行）

Council 评审指出 27 个预埋 `Optional=None` 字段是"复杂性的定时炸弹"。**立即精简至 ≤8 个 IR 结构完整性必需字段：**

| 保留字段 | IR 类型 | 保留理由 |
|---------|--------|---------|
| `construction_kind` | `ElementIR` | Flutter 构造器选择是结构性差异，早年预留避免后期破坏性重构 |
| `output_mode` | `StyleRuleIR` | Dioxus→inline-style vs Flutter→widget 是根本性输出通道差异 |
| `update_semantics` | `SignalIR` | Flutter `setState` vs 各框架 setter 是结构性差异 |
| `accessor_style` | `SignalIR` | Dioxus `()` vs Vue `.value` 读取方式差异驱动 codegen 分支 |
| `is_copy` | `SignalIR` | Dioxus Copy/Clone 判断是编译期静态分析，与 Rust 所有权模型绑定 |
| `needs_clone` | `TypeIR` | 同上 |
| `needs_partial_eq` | `TypeIR` | 所有框架 key 比较需要 |
| `execution_mode` | `BlockIR` | 远期 SSR 的安全边界——标记 server-safe vs client-only |

**删除的 19 个字段**（移除字段定义，改为注释 `# reserved for P{X}` 在 IR 源文件中标注）：

```
accessibility_label, accessibility_role → P1+ 添加
portal_target, keep_alive, virtual_list_config → P3 添加
validation_rules, form_state → P3 添加
i18n_resources → P2 添加
async_loader, is_lazy, error_handler → P3 添加
animation_def → P3 添加
generic_params → P2 添加
composables → P3 添加
modes, breakpoints → P3 添加
is_pure → P2 添加
```

**规则：** 字段在对应 Phase 启动前作为**独立 PR** 添加，附带完整的 JSON Schema 更新和至少 1 个构造示例。禁止"提前为未来 Phase 预埋字段"。

### 1.3 最终哲学对齐会议（第 2 周第 1 天）

**议题：**

| # | 议题 | 需达成的书面共识 |
|---|------|----------------|
| 1 | IR 原型验证报告评审 | 基于 §1.1 的 4 个金丝雀 case 结果，确认 IR 方向 GO/NEEDS_ADJUSTMENT/BLOCKED |
| 2 | "范式抽象层" vs "多目标翻译器" 长期定位 | 明确 OmniView 的长期北极星。每个 IR 设计决策需经"5 年后能否适配无虚拟 DOM 框架"检验 |
| 3 | L2 地道性的追求边界 | 确认核心 20 元素 + signal/computed/@effect 追求 L2；平台增强特性通过辅助库提供 L2 |
| 4 | 里程碑门控纪律 | 每个 Phase 的"解锁下一 Phase"条件必须量化且不可绕过。见 §八 |
| 5 | 文档冻结纪律 | IR 调整通过 `IRVersion` 迭代，不回修改已冻结的 `v3.0-final` 主文档 |

**产出：** `DECISIONS.md` — 一页纸的哲学决策记录，所有核心团队成员签字。

### 1.4 发布 PHASE0_SPEC.md（第 2 周第 2 天）

**操作：** 从 `OmniLang_v0.3_设计方案.md` 提取以下章节，形成不可变任务书（精简版——仅包含 Phase 0 需要的部分）：

| 提取章节 | 内容 |
|---------|------|
| §一 设计哲学 | 三条原则 + 安全表达式子集 + 定位再澄清 + 地道转译哲学 |
| §二 组件模型 | 组件定义语法、Event 系统 |
| §三 抽象元素系统 | 20 个核心元素 + 语义等价≠行为等价 |
| §四 @指令系统 | @if/@for/@slot/@native/@fallback、bind:、@native 严格规则 |
| §五 响应式状态 | signal/computed、Signal 行为契约、生命周期 |
| §六 Store | Store 定义、组件使用、适用范围声明 |
| §七 语义样式 | 语义属性、class 边界规则、样式跨框架诚实边界 |
| §八 跨框架编译 | 三阶段管道、L0/L1/L2、降级链、L1 验收清单 |
| §十 IR 规范 | **仅 Phase 0 必须的 8 个意图类型**（StateIntent, ComputedIntent, ListRenderingIntent, ConditionalIntent, BindIntent, EventIntent, StyleIntent, LayoutIntent）+ §10.1a-10.1c 中的基础 IR 类型 |
| §十三 转译约束 | 可移植性层级、Analyze 规则 |
| §十四 Phase 0 | 目标、范围、量化指标、任务清单 |
| §十六 TodoMVC | 完整示例 |
| §十七 测试策略 | 单元测试、行为测试套件 |
| §十八 调试 | 行号注释、自动格式化 |
| §二十 已知未决 | 5 项 P0 阻断 |

**产出：** `PHASE0_SPEC.md` — 冻结的 Phase 0 任务书。

### 1.5 组建核心审查小组（第 2 周第 3 天）

**组成：**

| 角色 | 人数 | 职责 |
|------|------|------|
| 编译器实现者 | 2-3 人 | Phase 0 全部开发工作 |
| 资深前端架构师（React） | 1 人 | **非编译器实现者，从 0.1 阶段即前置介入**——以"代码消费者"视角对语法设计、IR 抽象、Codegen 模板进行前置体验与反馈，防止团队陷入"逻辑正确但直觉难受"的歧途。P0.8 执行正式代码质量审查 + 差异率计算 |
| 技术负责人 | 1 人 | 双周设计偏差审查主持。所有关键技术决策和设计思路强制文档化（非口口相传） |

**产出：** 审查小组成员名单 + 每周可用时间承诺。

---

## 二、Phase 0：React L2 地道生成（10 周，含 OmniLang 委托基础设施）

> **目标：** 一个 OmniView 组件 → 一个 L2 地道质量的 React + Zustand 应用  
> **范围：** React + Zustand + 20 核心元素 + CSS Modules + 3 生命周期 + 精简意图 IR（8 类型）  
> **人员：** 2-3 编译器实现者 + 1 React 架构师（从 0.1 阶段即前置介入）  
> **IR 版本：** `0.3`（起始）  
> **★ 核心调整：** Council 评审后从 12 周压缩至 8 周，第二次 Council (2026-06-05) 因 OmniLang 委托基础设施调整为 10 周。IR 设计不再作为独立 3 周阶段——改为与 codegen 交叉进行（用 codegen 发现的问题驱动 IR 调整）。MVP 交付标准不变：TodoMVC 完整可运行 + L2 地道输出。

---

### □ 0.1 语法解析 + 语义分析（2.5 周，合并原 0.1+0.2）

> **★ 调整：** 原 0.1（2 周）+ 0.2（2 周）= 4 周 → 合并为 2.5 周。Parser 和语义分析在实现中高度耦合——符号解析、类型检查、依赖收集可以在 AST 构建的同时完成，无需串行等待。

**目标：** `.omni` 源文件 → 带类型标注和诊断的 AST

#### 0.1.1 Parser — UI 结构（2 周，表达式/语句由 OmniLang 处理）

> **★ 架构调整：** OmniView 的 Lark 语法文件**只解析 UI 结构**。表达式和语句块通过嵌入点委托给 `omnilang.parser` 解析。
> **★ Council 修正：** 原计划 3 天过于乐观。实际需要 2 周（含委托基础设施）。见 §1.0 改造表。

**0.1.1a 委托基础设施（3 天）**

| 交付物 | 说明 |
|--------|------|
| 大括号平衡预处理器 | 扫描 `.omni` 源文本，识别 `{...}` 插值和语句块区域，用占位符令牌替换，收集原始文本供 OmniLang 解析 |
| 解析器缓存 | `omnilang.parser` 实例复用。单例或惰性初始化，避免每次嵌入表达式都重建 Lark 解析器 |
| `parse_inline_block()` | 新增辅助函数，利用 `omnilang.parser_helpers.parse_inline_expr` 的模式，处理语句块解析 |
| 错误位置映射 | OmniLang 诊断的起始行/列 → `.omni` 源行/列。预处理程序记录每个嵌入点的偏移量 |

**0.1.1b 语法与 Transformer 改造（4 天）**

| 交付物 | 说明 |
|--------|------|
| Token 定义 | 关键字（`component`, `store`, `signal`, `view`, `style` 等）、`@` 前缀指令终端 |
| 表达式/语句规则移除 | 从 `grammar.lark` 中删除 ~30 行 expr/stmt/block 产生式规则。添加嵌入终端 `RAW_BLOCK`/`RAW_EXPR` |
| 组件解析 | `component Name { prop/signal/computed/event/fn/view/style }` 语法——fn body 替换为原始块占位符 |
| Store 解析 | `store Name { signal/getter/action }` 语法——action body 替换为原始块占位符 |
| 视图解析 | `@if`/`@else`/`@for`/`@empty`/`@slot`/`@native`/`@fallback`——条件表达式替换为原始表达式占位符 |
| 委托调用 | Transformer 中的 `_parse_omnilang_expr()` 和 `_parse_omnilang_block()` 方法 |
| AST 定义 | `ComponentAST`、`StoreAST`、`ViewTree`、`ElementAST`、`FnAST`——**所有表达式相关类型复用 `omnilang.ast_nodes`** |
| AST 位置信息 | 每个节点携带 `Span`（复用 `omnilang.utils.Span`） |

#### 0.1.2 语义分析（1.5 周，委托给 OmniLang + OmniView UI 约束）

> **★ 架构调整：** 类型检查、符号解析、依赖收集全部由 `omnilang.semantic` 完成。OmniView 只做 UI 特有的约束校验。
> **★ Council 修正：** 原计划 3 天过于乐观。OmniLang 现有 API（`check(Program)`）需要扩展以支持表达式级调用。实际需要 1.5 周。

| 交付物 | 负责 | 说明 |
|--------|------|------|
| 符号解析 | **OmniLang** | 组件作用域 / Store 作用域 / 视图作用域。`IdentifierExpr` 解析到具体符号 |
| 类型检查 | **OmniLang** | Props 类型检查、signal/computed 类型推断、fn 返回类型检查 |
| 依赖收集 | **OmniLang** | Layer 1（内联表达式精确收集）+ Layer 2（函数调用参数推断）+ Layer 3（`#[deps]` 显式标注） |
| Store 读取分析 | **OmniLang** | 识别组件中读取的 Store 字段，生成精确选择器字段列表 |
| 安全表达式校验 | **OmniLang** | 属性值/文本插值中禁止副作用和函数调用；事件处理器内不限制 |
| computed 链式扁平化 | **OmniLang** | `B→A→signal` 展开为 `B→signal` |
| bind 约束 (P0214) | **OmniView** | bind: 只能绑定 signal，不能绑定 prop——UI 特有规则 |
| @for key 要求 (P0201) | **OmniView** | @for 必须提供 key——UI 特有规则 |
| class 混用提示 (S0110) | **OmniView** | `style.xxx` + 字符串 class 混用提醒——UI 特有规则 |
| @native 校验 | **OmniView** | 非匹配目标时 ERROR——UI 特有规则 |

**0.1 验收标准：**
- [ ] TodoMVC 完整源码可无错误解析为 AST
- [ ] 语义分析通过 TodoMVC（0 个错误）
- [ ] 每个 P 级规则（P0201/P0203/P0214）有 3+ 触发用例，验证诊断含正确行号
- [ ] Store 读取分析能正确识别 `store.todos` 的读取（非 `store.filter`）
- [ ] computed 依赖链 `B→A→signal` 展开为 `B→signal`

---

### □ 0.2 精简 IR 设计 + React Codegen（2.5 周，合并原 0.3+0.4）

> **★ 核心调整：** 原 0.3（3 周 IR 设计）+ 0.4（3 周 React Codegen）= 6 周 → 合并为 2.5 周。IR 类型从 15+12+6=33 个精简至 **8 个意图类型 + 6 个基础类型 = 14 个**。IR 不再作为独立冻结阶段——codegen 实现中发现的问题直接驱动 IR 调整，确保 IR 是被验证过的而非被设计的。

**目标：** 语义 AST → 精简意图 IR → L2 地道 React TypeScript 代码

#### 0.2.1 精简 IR 类型定义（½ 周，与 codegen 交叉）

> **IR 所有权：** Intent IR 由 OmniView 拥有（`omniview/ir_types.py`）。语言级基础类型（`ExprIR`, `TypeIR`, `BlockIR`, `StatementIR` 等）从 OmniLang 导入。当 OmniLang 的表达式 AST 发生变化时，OmniView 的 IR 适配层（`lowering.py`）负责转换。IR 版本号 `IRVersion` 由 OmniView 管理，不受 OmniLang 版本影响。

| 交付物 | 对应设计文档 | 关键字段 |
|--------|------------|---------|
| `StateIntent` + `MutationIntent` | §10.1a | name + type + initial_value + mutability; target + operation（8 种 MutationOp） |
| `ComputedIntent` + `EffectIntent` + `LifecycleIntent` | §十 意图列表 | name + type + body + deps |
| `ListRenderingIntent` | §10.1a | iterable + item_binding + body + key_expr + empty_body |
| `ConditionalIntent` | §10.1a | branches + fallback |
| `BindIntent` | §10.1a | source: SignalRefIR + element_type + direction |
| `EventIntent` + `SlotIntent` | §十 意图列表 | event_name + handler; name + body |
| `StyleIntent` + `LayoutIntent` | §10.1b | target_element + classes/properties |
| 基础类型 | §10.1b-10.1d | `ExprIR` 联合体 + `SignalRefIR` + `BindingIR` + `ComponentInstanceIR` + `EventHandlerIR` + `TypeIR` + `BlockIR` + `StatementIR` 联合体 |

**不进入 Phase 0 的意图类型：** `StoreIntent`（Phase 0 Store codegen 直接从 AST 生成）、`ExternIntent`（P0 不做 extern 编译）、`FallbackIntent`（P0 仅 React 目标不需要降级）。

#### 0.2.2 状态与计算 Codegen（3 天）

| 意图 → React L2 输出 | 
|----------------------|
| `StateIntent(name="count", type=i32, initial=0, mutability=MUTABLE)` → `const [count, setCount] = useState(0);` |
| `MutationIntent(target=count, op=INCREMENT)` → `setCount(count + 1);` |
| `MutationIntent(target=todos, op=SPREAD_APPEND)` → `setTodos([...todos, new_todo]);` |
| `ComputedIntent(name="double", body=..., deps=[count])` → `const double = useMemo(() => count * 2, [count]);` |
| `LifecycleIntent(kind=MOUNT)` → `useEffect(() => { ... }, []);` |
| `EffectIntent(deps=[count])` → `useEffect(() => { ... }, [count]);` |

#### 0.2.3 视图 Codegen（4 天）

| 意图 → React L2 输出 | 说明 |
|----------------------|------|
| `ConditionalIntent(branches=[(cond, A)], fallback=None)` → `{cond ? (<A/>) : null}` |
| `ConditionalIntent(branches=[(cond1, A), (cond2, B)], fallback=C)` → `{cond1 ? (<A/>) : cond2 ? (<B/>) : (<C/>)}` |
| `ListRenderingIntent(iterable=items, body=ListItem, key_expr=item.id)` → `{items.map(item => <ListItem key={item.id} .../>)}` |
| `ListRenderingIntent` + `empty_body` → `{items.length === 0 ? <Empty/> : items.map(...)}` |
| `BindIntent(source=email, element_type=TEXT_INPUT)` → `value={email} onChange={e => setEmail(e.target.value)}` |
| `BindIntent(source=done, element_type=CHECKBOX)` → `checked={done} onChange={e => setDone(e.target.checked)}` |
| `ElementIR` + `ComponentInstanceIR` | 元素→JSX 映射 + 子组件实例化 |

#### 0.2.4 Props Interface + 组件骨架 + 代码格式化（2 天）

| 交付物 | 说明 |
|--------|------|
| **类型映射规格表** | OmniView 类型→TypeScript 类型的精确映射表 |
| Props interface | `type`→TS 类型、`?`→可选字段 |
| 组件骨架 | `export const ComponentName: React.FC<Props> = (props) => { ... }` |
| `test-id` | `test-id="btn"` → `data-testid="btn"` |
| 行号注释 | 每个逻辑节点前 `/* .omni:行号 */` |
| 闭包捕获 | @for 循环变量 → 块级 `const item_ = item;` |
| 自动格式化 | prettier 集成（若已安装）+ 内置缩进引擎 |

**0.2 验收标准：**
- [ ] TodoMVC IR → 生成的 TSX 代码可编译（`tsc --noEmit` 通过）
- [ ] 运行 `npm start` 后 TodoMVC 功能正确
- [ ] 行号注释覆盖率 100%
- [ ] 闭包捕获测试：`@for` 内 `onPress` 闭包引用循环变量，行为正确
- [ ] 8 个意图类型全部有精确 `@dataclass` 定义 + L1 模板伪代码注释
- [ ] IR 预埋字段 ≤ 8 个（§1.2 精简后），其余以注释 `# reserved for P{X}` 标记

---

### □ 0.3 Store Codegen — Zustand L2（0.5 周，从 AST 直接生成，不经过 IR）

> **★ 调整：** Store codegen 不经过 `StoreIntent` IR——直接从 `StoreAST` 生成 Zustand 代码。`StoreIntent` 延至 Phase 1（Vue Pinia codegen 需要时再定义）。这消除了一个投机性 IR 类型，节省约 2 天。

| 交付物 | 说明 |
|--------|------|
| Store 骨架 | `interface TodoStoreState { ... }; export const useTodoStore = create<TodoStoreState>()((set, get) => ({ ... }));` |
| signal→state | `signal todos: Array<Todo> = [];` → `todos: [] as Todo[],` |
| getter→fn | `getter active_count: i32 = ...` → `active_count: () => state.todos.filter(t => !t.done).length,` |
| action→set() | `action add(text: String) { ... }` → `add: (text: string) => set(state => ({ todos: [...state.todos, ...] })),` |
| 精确选择器 | 组件读取 `store.todos` → 生成 `useStore(s => s.todos)` |

**0.3 验收标准：**
- [ ] TodoMVC Store 生成 Zustand 代码，`tsc --noEmit` 通过
- [ ] 精确选择器验证：组件只读取 `store.todos` 时不触发 `filter` 变化导致的重渲染

---

### □ 0.4 CSS 生成（0.5 周）

| 交付物 | 说明 |
|--------|------|
| 语义属性→CSS 查找表 | ~50 条内置映射规则 |
| CSS Modules 输出 | `style { .card { ... } }` → `Component.module.css` |
| class 引用 + `class_map` + `:root` 变量 | 完整 CSS Modules 集成 |

**0.4 验收标准：**
- [ ] TodoMVC style 块生成合法的 CSS Modules
- [ ] 编译产物在浏览器中渲染效果正确

---

### □ 0.5 测试 — IR 快照 + Codegen 快照 + 行为测试（1 周，合并原 0.7+0.7a）

| 测试类型 | 覆盖范围 | 用例数（最低） |
|---------|---------|-------------|
| 解析→IR 快照 | 每个语法构造（component/store/view/style/@if/@for/@slot/@native/bind） | 20+ |
| Codegen 输出快照 | 20 个核心元素 × 每种常见 props 组合 + Store→Zustand + 样式→CSS | 50+ |
| 诊断测试 | P0214（bind 约束）/ P0201（@for key）/ P0203（依赖推断） | 每规则 3+ |
| 行为测试 | counter / list-rendering / conditional / bind-input / store-read — React Playwright 测试 + CI 集成 | 5 用例 |

**0.5 验收标准：**
- [ ] 所有快照测试通过
- [ ] 5 个行为用例 Playwright 测试全部通过 + CI 集成
- [ ] L1 可读性验收自动化检查通过（行号追溯 / 无运行时依赖 / prettier 通过）

---

### □ 0.6 Fixtures + CLI（1 周，合并原 0.8+0.9）

#### 0.6.1 Fixture 库 + 代码质量审查

| 类别 | 数量 | 说明 |
|------|------|------|
| 组件 fixture | 20+ | 每个核心元素至少 1 个使用示例 |
| Store fixture | 3+ | 简单 CRUD、过滤、异步加载 |
| 完整应用 | 2 | Counter（最简）+ TodoMVC（完整 CRUD+过滤） |

**代码质量对照审查（原 0.8.2）：**
- React 架构师逐行对比生成的 TodoMVC 与手写参考实现
- 5 维度评分（变量命名 / Hooks 使用 / JSX 结构 / CSS 类名 / 注释覆盖率）
- **目标：差异率 < 5%**

#### 0.6.2 CLI — `omnic env` + `omnic build`

| 命令 | 功能 |
|------|------|
| `omnic env create --type ui --target react` | 创建 UI 框架环境 |
| `omnic build` | 编译 .omni→React TSX + CSS Modules |
| `omnic check` | 静态分析（诊断输出） |
| `omnic fmt` | 格式化 .omni 源文件 |

**0.6 验收标准：**
- [ ] 20+ 组件 fixture 全部可编译
- [ ] 2 完整应用全部可运行
- [ ] 代码质量对照审查完成，差异率 < 5%
- [ ] 端到端流程：`omnic env create` → `omnic build` → `tsc --noEmit` → Playwright 全部通过

---

### Phase 0 总时间线（10 周，含 OmniLang 委托基础设施）

> **★ Council 修正（2026-06-05）：** 原计划 8 周未计入 OmniView→OmniLang 委托基础设施的构建成本。§1.0 定调后，0.1 需要额外 1 周（大括号预处理器 + 解析器缓存 + `parse_inline_block` + 错误映射）。

```
Week 1:      □ 0.0 委托基础设施（大括号预处理器/缓存/API桥接）— 1 周
Week 2-3:    □ 0.1a 语法改造 + Transformer 委托（1 周）
Week 3-4.5:  □ 0.1b 语义委托 + OmniLang API 扩展（1.5 周）
Week 4.5-7:  □ 0.2 精简 IR 设计 + React Codegen（2.5 周）
Week 7-7.5:  □ 0.3 Store Codegen — Zustand L2（0.5 周）
Week 7.5-8:  □ 0.4 CSS 生成（0.5 周）
Week 8-9:    □ 0.5 测试（1 周）
Week 9-10:   □ 0.6 Fixtures + CLI（1 周）
```

> **注意：** 0.0 委托基础设施可与 IR 原型验证（§1.1）并行，减少关键路径阻塞。任务工作量合计约 9 周，10 日历周中包含 ~1 周缓冲/重叠。

### Phase 0 完成门控 → 解锁 Phase 1

| 门控条件 | 验证方式 |
|---------|---------|
| ✅ TodoMVC 完整可运行（React + Zustand + CSS Modules） | `npm start` + Playwright 全流程通过 |
| ✅ 代码质量差异率 < 5% | React 架构师逐行对比审查报告 |
| ✅ 5 个行为用例 Playwright 测试全部通过 + CI 集成 | CI 日志 |
| ✅ CLI 端到端流程可用 | `omnic env create` → `omnic build` → `tsc --noEmit` |
| ✅ **至少 1 个外部 React 开发者**（非编译器实现者）使用 Phase 0 产物构建了一个简单组件并给出反馈 | 用户反馈记录 |

> **★ 关键门控：** 最后一条是 Council 的核心建议——"先证明有人用，再扩展"。如果 Phase 0 产物在 2 周内无法找到哪怕一个愿意试用的外部开发者，这是比任何技术风险都更强的"不要继续"信号。

---

## 三、Phase 1：Vue 3 L2 地道生成（8 周）

> **目标：** Vue 3 Template + Pinia + Vite plugin  
> **地道性：** Vue → L2  
> **前置：** Phase 0 全部验收通过

### 任务清单

| 编号 | 任务 | 周期 | 关键交付物 |
|------|------|------|----------|
| 1.1 | Vue 3 Codegen — 三阶段管道 | 3 周 | 模式匹配引擎移植 + Vue 惯用法规则库（`ref`/`computed`/`v-model`/`v-if`/`v-for`/`<slot>`/`<Transition>`/`<KeepAlive>`/`<Suspense>`/`<Teleport>`） |
| 1.2 | Pinia Store Codegen | 1 周 | `defineStore` + `ref` state + `computed` getters + actions |
| 1.3 | SFC 输出 (.vue) | 1 周 | `<template>` + `<script setup>` + `<style scoped>` 三块组合 |
| 1.4 | Vite Plugin | 1 周 | `.omni` 文件的热重载（HMR）+ 开发时即时编译 |
| 1.5 | Source Map | 0.5 周 | 编译后代码→`.omni` 源码映射 |
| 1.6 | A11y 基础 | 0.5 周 | `accessibilityLabel`→`aria-label`、`accessibilityRole`→`role` |
| 1.7 | 行为测试 (Vue) | 0.5 周 | 5 行为用例编译到 `tests/frameworks/vue/src/` → Vue Playwright 测试（复用 `tests/frameworks/vue/` 统一环境，无需新建项目） |
| 1.8 | 测试工具 (Vue) | 0.5 周 | `@omnilang/test-utils` Vue 版（包装 `@vue/test-utils`） |

**Phase 1 验收标准：**
- [ ] TodoMVC 编译为 `.vue` 文件，Vite 开发服务器可运行
- [ ] HMR：修改 .omni 源文件 → 浏览器即时更新
- [ ] Vue 输出达到 L2 地道质量（Vue 开发者代码审查差异率 < 5%）
- [ ] 跨 React/Vue 行为测试一致性矩阵填入

---

## 四、Phase 2：Dioxus L2 地道生成（10 周）

> **目标：** Dioxus RSX + Global Signals  
> **地道性：** Dioxus → L2  
> **前置：** Phase 1 全部验收通过

### 任务清单

| 编号 | 任务 | 周期 | 关键交付物 |
|------|------|------|----------|
| 2.1 | Dioxus Codegen — 三阶段管道 | 4 周 | Rust 惯用法规则库（`use_signal`/`use_memo`/`rsx!`/`.iter().map()`/`if`-`else`/`match`/`Clone`/`PartialEq`） |
| 2.2 | 借用冲突处理 | 1 周 | 所有 signal 值在 `rsx!` 前提取为局部变量 + `.clone()`。`#[derive(Clone, PartialEq)]` 自动生成 |
| 2.3 | Global Signals Store | 1 周 | `static SIGNAL: GlobalSignal<T>` + `pub fn use_store()` 模块生成 |
| 2.4 | @error 错误边界 | 0.5 周 | Dioxus 降级：`catch_unwind` + P 级警告（文档化限制） |
| 2.5 | Ref\<T\> | 0.5 周 | `NodeRef<T>` 展开 + `.current` 访问接口 |
| 2.6 | gen-extern 工具 | 0.5 周 | 从 `.d.ts` 自动生成 `extern crate` 声明 |
| 2.7 | 泛型组件 | 0.5 周 | Rust 泛型函数生成（`fn Comp<T: Clone + PartialEq>(props: Props<T>) -> VNode`） |
| 2.8 | i18n 基础 | 0.5 周 | 编译时静态映射 + 运行时 locale context |
| 2.9 | 日期/数字格式化 | 0.5 周 | `chrono`/`num-format` crate 调用 |
| 2.10 | 测试工具 (Dioxus) | 0.5 周 | `dioxus::ssr::render` + DOM 断言 |
| 2.11 | 行为测试 (Dioxus) | 0.5 周 | 5 行为用例编译到 `tests/frameworks/dioxus/src/` → `dioxus::ssr` 测试（复用 `tests/frameworks/dioxus/` 统一环境） |

**Phase 2 验收标准：**
- [ ] Dioxus 输出达到 L2 地道质量（Rust 开发者代码审查差异率 < 5%）
- [ ] 跨 React/Vue/Dioxus 行为测试一致性矩阵填入
- [ ] 借用冲突：生成的代码 `cargo check` 通过，无 borrow checker 错误
- [ ] `<Suspense>` 降级为条件渲染时发出 P 级警告

---

## 五、Phase 2a：Flutter 基础验证（6 周）

> **目标：** 验证意图 IR→Flutter Widget 映射完整性，暴露 IR 设计缺陷  
> **地道性：** Flutter → L1（仅可读代码，不追求 L2）  
> **前置：** Phase 2 全部验收通过  
> **若资源有限：** 缩减到 4 周（仅 5 个核心元素）

### 任务清单

| 编号 | 任务 | 周期 | 关键交付物 |
|------|------|------|----------|
| 2a.1 | 10 核心元素 L1 Codegen | 2 周 | Container/Flex/Text/Heading/Button/Input/Checkbox/List/ListItem/Image → Flutter Widget 映射 |
| 2a.2 | signal/computed L1 | 1 周 | `StateIntent`→`setState`、`ComputedIntent`→手动缓存、`MutationIntent`→`setState` 展开 |
| 2a.3 | @if/@for/@slot/bind L1 | 1 周 | `ConditionalIntent`→`if`-`else` widget、`ListRenderingIntent`→`ListView.builder`、`BindIntent`→L1 `onChanged`+`setState`（非 TextEditingController） |
| 2a.4 | Widget 测试 | 1 周 | 5 行为用例编译到 `tests/frameworks/flutter/lib/` → `flutter_test` 验证（复用 `tests/frameworks/flutter/` 统一环境，一次 `flutter pub get`） |
| 2a.5 | IR 映射缺陷报告 | 1 周 | 记录所有无法通过现有 IR 结构表达的 Flutter 特性，评估是否需要 IR 次版本调整 |

**Phase 2a 验收标准：**
- [ ] 10 个核心元素在 Flutter 中可渲染
- [ ] 5 个行为用例通过 `flutter_test`
- [ ] IR 映射缺陷报告：列出所有需要 IR 调整的问题（如有），评估影响面
- [ ] 若需 IR 调整：发布 `IRVersion 0.4` 草案，评估对 P0-P2 codegen 的回修改影响

---

## 六、Phase 3：能力深化 + Flutter L1（10 周）

> **目标：** 14 项能力深化 + Flutter L1 持续完善  
> **地道性：** Flutter → L1  
> **前置：** Phase 2a 验收通过

### 任务清单

| 编号 | 能力 | 对应设计文档 | 周期 | 关键交付物 |
|------|------|------------|------|----------|
| 3.1 | 复合元素 | §15 + 附录 A（P3 元素） | 1.5 周 | Modal/Toast/Tooltip/Badge/Tabs/Accordion/Nav/Table 等全框架映射 |
| 3.2 | 动画系统 | §15.8 | 1.5 周 | `<Transition>`/`<TransitionGroup>`/`animate()`。React→framer-motion（需 `--animation-lib`）、Vue→`<Transition>`（原生）、Dioxus→CSS class、Flutter→`AnimatedWidget` |
| 3.3 | 表单验证 | §15.2 | 1 周 | `<FormField>` + `rules` 属性 + 同步校验。异步校验通过手动 `errors` signal |
| 3.4 | Suspense / @load / lazy | §15.3 + §15.16 | 1 周 | `<Suspense>` 元素 + `@load` 生命周期 + `lazy()`。Dioxus 降级为条件渲染 |
| 3.5 | KeepAlive | §15.5 | 0.5 周 | React→hidden、Vue→`<KeepAlive>`、Dioxus→`display:none`、Flutter→`AutomaticKeepAliveClientMixin` |
| 3.6 | Portal / Teleport | §15.6 | 0.5 周 | React→`createPortal`、Vue→`<Teleport>`、Dioxus→web_sys portal、Flutter→`OverlayEntry` |
| 3.7 | VirtualList | §15.7 | 1 周 | React/Vue→`--virtual-list-lib` 配置（T3 平台增强）、Flutter→`ListView.builder`（原生） |
| 3.8 | Composable 语法 | §15.9 | 1 周 | 宏展开编译（零运行时开销） |
| 3.9 | 主题深化 | §15.13 | 0.5 周 | 亮/暗模式 + `breakpoints()` 响应式断点 |
| 3.10 | Redux/Jotai Store | 附录 B | 0.5 周 | Redux Toolkit `createSlice` / Jotai `atom` 映射 |
| 3.11 | SCSS 桥接 | §十二 item 2 | 0.5 周 | 桥接用户本地 SCSS 工具链，不内置 SCSS 编译器 |
| 3.12 | bind: 扩展 | §十一.4 | 0.5 周 | `bind:selected`/`bind:open` 等 |

**Phase 3 验收标准：**
- [ ] 14 项能力深化全部完成（每项有至少 1 个跨框架 fixture 验证）
- [ ] Flutter L1 输出满足 §8.1a 的 7 条可读性标准
- [ ] 跨框架行为测试一致性矩阵增长至 10 个用例

---

## 七、Phase 4：Flutter L2 + 生态完善（8 周）

> **目标：** Flutter L2 地道生成 + 框架辅助库 + 文档  
> **地道性：** Flutter → L2  
> **前置：** Phase 3 全部验收通过

### 任务清单

| 编号 | 任务 | 周期 | 关键交付物 |
|------|------|------|----------|
| 4.1 | Flutter L2 Codegen | 2 周 | `TextEditingController` 完整生命周期、`AnimatedWidget`、`Expanded`/`Flexible` 布局推断、构造器选择 |
| 4.2 | Riverpod/BLoC Store | 1 周 | `@riverpod` class / `Bloc<Event, State>` 生成 |
| 4.3 | `omni_flutter_helpers` 库 | 1 周 | `OmniTextField`/`OmniCheckbox` 等薄辅助组件（纯 Dart 实现，封装最佳实践） |
| 4.4 | 完整元素映射 (55+) | 1 周 | 附录 A 全部元素 Flutter 端实现 |
| 4.5 | Composable 完善 | 0.5 周 | 泛型 composable + 嵌套 composable |
| 4.6 | 动画 Flutter 适配 | 0.5 周 | Hero、PageTransition 等 Flutter 特有动画 |
| 4.7 | 测试工具 (Flutter) | 0.5 周 | `WidgetTester` + `pumpWidget` 包装 |
| 4.8 | L3 视觉回归测试 | 0.5 周 | React DOM 截图 vs Flutter Widget 截图对比 |
| 4.9 | 文档 | 0.5 周 | 完整语言参考 + 各框架快速入门 + API 文档 + L1→L2 优化指南 |
| 4.10 | 行为测试 (Flutter) | 0.5 周 | 全部 10 个行为用例编译到 `tests/frameworks/flutter/lib/` → `flutter_test` 验证（复用统一 Flutter 环境） |

**Phase 4 验收标准：**
- [ ] Flutter 输出达到 L2 地道质量（Dart 开发者代码审查差异率 < 5%）
- [ ] 跨框架行为测试一致性矩阵全部填入（✅/⚠️）
- [ ] L3 视觉回归测试：React DOM 截图与 Flutter Widget 截图在关键页面结构上视觉等价
- [ ] `omni_flutter_helpers` 发布到 pub.dev
- [ ] 完整文档上线

---

## 八、里程碑门控机制

> **★ Council 核心建议：** 从"54 周固定周期"转为"里程碑驱动"——每个 Phase 通过用户验证后才解锁下一 Phase。这防止了"为赶进度而降低质量标准"的常见陷阱，也确保方向在获得真实反馈前不会偏离太远。

### 8.1 门控规则

| 规则 | 说明 |
|------|------|
| **硬门控** | 所有门控条件必须全部满足，Phase 才算完成。不满足 → 不解锁下一 Phase |
| **独立验证** | React 架构师的代码审查和外部用户反馈**独立于编译器实现者**，不可由实现者自评自审 |
| **2 周冷静期** | Phase 完成后，给 2 周时间让外部用户试用并反馈。若 2 周内"无外部用户反馈"→ 触发决策点（见 §8.3） |
| **IR 调整规则** | 若用户反馈要求 IR 调整，升级 `IRVersion` 次版本号，不回修改已冻结的 Phase 产出 |

### 8.2 各 Phase 门控条件

| 里程碑 | 门控条件 | 验证人 | 解锁 |
|--------|---------|--------|------|
| M1: Phase 0 完成 | ① TodoMVC 可运行 ② 差异率 < 5% ③ 5 行为用例 CI 通过 ④ CLI 端到端可用 ⑤ **≥1 外部 React 开发者试用 + 反馈** | React 架构师 + 外部用户 | Phase 1 |
| M2: Phase 1 完成 | ① Vue TodoMVC 可运行 (HMR) ② 差异率 < 5% ③ 跨 React/Vue 行为测试矩阵填入 ④ **≥1 外部 Vue 开发者试用 + 反馈** | Vue 架构师 + 外部用户 | Phase 2 |
| M3: Phase 2 完成 | ① Dioxus TodoMVC `cargo check` 通过 ② 0 borrow checker 错误 ③ 跨 React/Vue/Dioxus 矩阵填入 ④ **社区反馈**（GitHub issue/discussion） | Rust 架构师 + 社区 | Phase 2a |
| M4: Phase 2a 完成 | ① Flutter 10 元素可渲染 ② 5 行为用例 `flutter_test` 通过 ③ IR 缺陷报告（决定是否升级 IRVersion） | 技术负责人 | Phase 3 |
| M5: Phase 3 完成 | ① 14 项能力深化各至少 1 个 fixture ② Flutter L1 满足 7 条可读性标准 ③ 跨框架矩阵增长至 10 用例 | 各框架架构师 | Phase 4 |
| M6: Phase 4 完成 | ① Flutter L2 差异率 < 5% ② 55+ 元素全部映射 ③ L3 视觉回归通过 ④ 文档上线 ⑤ `omni_flutter_helpers` 发布到 pub.dev | Dart 架构师 + 社区 | 项目完成 |

### 8.3 "无外部反馈"决策点

如果 Phase 完成后 2 周内无法获得外部用户反馈：

| 情况 | 决策 |
|------|------|
| Phase 0：无 React 开发者愿意试用 | **严重信号。** 召开紧急会议：是产品定位问题？还是产出质量不够好？若定位问题 → 重新审视 §一的设计哲学。若质量问题 → 追加 2 周修复，再试 |
| Phase 1：无 Vue 开发者愿意试用 | **信号。** Vue 市场是否需要跨框架方案？考虑将 Phase 2 (Dioxus) 提前，验证 Web→原生框架的吸引力是否强于 Web→Web |
| Phase 2+：无社区反馈 | **可接受。** Dioxus/Flutter 社区较小。依赖内部架构师审查即可 |

### 8.4 Phase 优先级重排机制

若门控过程中发现某个目标框架的需求远超预期（如 5 个外部 Vue 开发者要求试用，而 Dioxus 需求为 0）：

1. **技术负责人有权调整 Phase 顺序**——例如 Phase 2 (Dioxus) 与 Phase 1 (Vue) 互换
2. **调整需记录在 `DECISIONS.md` 中**，附带原因和数据
3. **主文档** `OmniLang_v0.3_设计方案.md` **不修改**——Phase 顺序是执行层面决策，不影响设计

---

## 九、跨 Phase 关注项

### 9.1 IR 版本迭代纪律

```
IRVersion "0.3"  →  Phase 0 起始
IRVersion "0.4"  →  Phase 2a 后（如需调整）
IRVersion "1.0"  →  Phase 4 完成后（稳定版）
```

**规则：**
- 任何 IR 结构调整 = 升级次版本号
- 新增可选字段 = 不升级版本号（向前兼容）
- 修改现有字段类型 = 升级主版本号（需对所有 codegen 做回修改）

### 9.2 跨框架行为测试一致性矩阵

> **★ 这是项目最核心的资产之一。** 从 P0 开始将其作为项目看板的核心部分，高亮每个 ⚠️ 标记。Phase 评审中不仅看 ✅，更要**深入讨论每个 ⚠️ 的可接受性、文档化补偿和未来消除计划**。此矩阵的健康度直接反映 OmniView 跨框架承诺的可信度。

| 行为用例 | React (P0) | Vue (P1) | Dioxus (P2) | Flutter (P4) |
|---------|-----------|----------|-------------|-------------|
| counter | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| list-rendering | ✅ P0 | ✅ P1 | ⚠️ P2 (key 类型) | ✅ P4 |
| conditional | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| bind-input | ✅ P0 | ✅ P1 | ✅ P2 | ⚠️ P4 (TextEditingController) |
| bind-checkbox | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| slot-default | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| slot-named | ✅ P0 | ✅ P1 | ✅ P2 | ⚠️ P4 (命名参数) |
| event-optional | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| store-read | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |
| store-action | ✅ P0 | ✅ P1 | ✅ P2 | ✅ P4 |

每个 Phase 完成后更新此矩阵。⚠️ 标记已知行为差异，附文档链接。

### 9.3 L1/L2 质量追踪

| Phase | 目标框架 | 目标层级 | 差异率目标 | 实际差异率 | 审查人 |
|-------|---------|---------|----------|----------|--------|
| P0 | React | L2 | < 5% | — | React 架构师 |
| P1 | Vue | L2 | < 5% | — | Vue 架构师 |
| P2 | Dioxus | L2 | < 5% | — | Rust 架构师 |
| P4 | Flutter | L2 | < 5% | — | Dart 架构师 |

### 9.4 范式抵御验证（Phase 2a 后）

在 Phase 2a 完成后，安排 2 周轻量原型：
- 用 SolidJS 或 Svelte 5 实现一个最小 codegen（仅 counter 和 list-rendering 两个行为用例）
- 验证意图 IR（`StateIntent`/`ListRenderingIntent`/`ConditionalIntent`）能否不修改核心结构即适配细粒度响应式模型
- 若验证失败：评估 IR 调整范围，在 `IRVersion 0.4` 中修正

---

## 十、风险登记册

> **★ Council 评审新增 11 项风险（R9-R19）。** R1-R8 为原始风险，R9-R11 为首次 Council (2026-06-04) 新增，R12-R19 为第二次 Council (2026-06-05) 新增（OmniView→OmniLang 委托架构相关）。

| # | 风险 | 严重度 | 可能性 | 缓解措施 | Phase |
|---|------|--------|--------|---------|-------|
| R1 | IR 设计缺陷在 P2a 暴露，需回修改 P0-P2 codegen | **高** | 中 | **§1.1 IR 原型验证前置到启动期**（原缓解：P2a 提前到 P2 之后立即执行）。金丝雀 case（ListView.builder/TextEditingController/Flex→Row）在第 1 周验证 | 启动期 |
| R2 | Dioxus codegen 的借用冲突处理超预期复杂度 | **高** | 高 | P2 预算 10 周（含 2 周缓冲）；`Clone`/`PartialEq` 自动 derive。**新增：P2 启动前用 2 个 Dioxus 金丝雀 case 验证 borrow checker 可行性** | 2 |
| R3 | Flutter `TextEditingController` 生命周期映射出错 | **中** | 中 | **§1.1 Case B 在第 1 周验证**（原缓解：P2a 中验证）。P4 中提供 `omni_flutter_helpers` 辅助库封装 | 启动期, 4 |
| R4 | React 并发特性（`useTransition`）打破 Signal 行为契约 | **中** | 低 | "同步批处理"契约明确不覆盖并发特性；超纲行为用 `@native` | 0+ |
| R5 | `@native` 滥用导致跨框架承诺落空 | **中** | 高 | `@native` 严格规则（非匹配目标 ERROR）；可移植性分数可视化；跨框架模块中禁止 | 0+ |
| R6 | 第三方库依赖（虚拟滚动/动画）绑定特定版本 | **低** | 中 | 通过编译 flag（`--virtual-list-lib`/`--animation-lib`）显式配置；降级为 L1 当库不可用时 | 3 |
| R7 | 框架大版本更新（React 19→20、Vue 3→4）破坏 codegen 输出 | **中** | 中 | 每个 Phase 测试覆盖多个框架版本；CI 中维护框架版本矩阵 | 持续 |
| R8 | P0 代码质量差异率 > 5% | **中** | 低 | 8 周周期（含缓冲）；P0.6 审查发现问题时可在 P0 内修复。**新增：外部 React 开发者反馈作为门控条件** | 0 |
| **R9** | **★ 渐进 L2→L1 质量滑落** — 团队在 Dioxus/Flutter 遇到困难时选择降级到 L1，"后续再优化"变成永久 L1 | **高** | 高 | 每个 Phase 门控条件强制 L2 差异率 < 5%。L1 降级必须记录在 `DECISIONS.md` 中并设定修复期限。连续 2 个 Phase L1 降级 → 触发决策点 | 2+ |
| **R10** | **★ 生态预期不匹配** — 用户发现"写组件一次，写胶水代码 N 次"（路由/Store 配置/控制器生命周期）与"跨框架"承诺不符 | **高** | 中 | 文档首页明确声明"OmniView 是设计系统描述语言，非通用前端语言"。Phase 0 发布时在 README 中诚实列出"你需要手动做什么"清单 | 0+ |
| **R11** | **★ 27→8 个预埋字段精简后，远期 Phase 重新添加时引入破坏性变更** | **中** | 低 | 新字段添加规则：① 对应 Phase 启动前作为独立 PR 添加 ② 附带 JSON Schema 更新 ③ 所有已有 codegen 的 `match/if let` 分支必须显式处理新字段（不允许通配 fallthrough） | 1-4 |
| **R12** | **★ OmniLang 语义 API 不匹配** — `check(Program)` 期望完整 Program 对象，不支持单个表达式/语句块的类型检查 | **高** | 高 | ① 新增 `check_expr(expr, scope)` 公共 API ② 或在 OmniView 侧组装合成 Program。见 §0.1.2 语义委托 | 0.1 |
| **R13** | **★ 大括号平衡预处理器复杂度** — 嵌套 `{}`（如 `{ items.filter(fn(i) { i > 0 }) }`）需要基于字符的扫描器，不能简单用正则匹配 | **中** | 高 | ① 实现括号深度跟踪的字符级扫描器 ② 注释和字符串字面量内的 `{}` 需要正确处理 ③ `@native` 块内容完全跳过 | 0.1 |
| **R14** | **★ OmniLang→OmniView 错误位置映射断裂** — 当 OmniLang 解析器失败时，错误行号指向合成 `fn __tpl()` 包装器，而非原始 `.omni` 行 | **中** | 高 | 预处理器记录每个嵌入点的 `(start_line, start_col)` 偏移量。诊断输出时应用偏移量反向映射 | 0.1 |
| **R15** | **解析器缓存缺失导致性能退化** — 当前 `parse_inline_expr` 每次调用都创建新 Lark 实例（读取语法文件 + 构建解析表），每组件 10-50 次调用 | **中** | 高 | 实现单例解析器缓存。首次调用时初始化，后续调用复用。需处理并发安全（`threading.Lock`） | 0.1 |
| **R16** | **OmniLang 类型检查依赖完整程序上下文** — 嵌入表达式 `{items.filter(fn(t){!t.done}).len()}` 的类型检查需要知道 `items` 的类型、各方法的签名。此信息跨 OmniView-OmniLang 边界 | **中** | 中 | ① OmniView 向 OmniLang 提供合成类型定义（signal/prop/store 字段类型）② 共享类型环境对象跨边界传递 | 0.1 |
| **R17** | **模板字符串 `${}` 嵌套在 `.omni` 的 `{}` 插值中** — 两层大括号导致预处理器需要区分 "OmniView 的插值边界" 和 "OmniLang 模板字符串的插值边界" | **低** | 低 | 预处理器识别模板字符串终端 `TEMPLATE_STRING` 后，跳过其内部的 `${}` | 0.1 |
| **R18** | **`@effect(dep) { ... }` 的混合语法** — `(dep)` 是 OmniView 语法，`{ ... }` 是 OmniLang 块。委托边界在单条 Lark 规则的中间 | **低** | 中 | 将 `@effect` 规则拆分为两部分：OmniView 解析 `(dep)` 部分，原始块占位符保留给 OmniLang | 0.1 |
| **R19** | **Python→Rust 技术栈切换导致委托架构重写两次** — 如果后续从 Python/Lark 迁移到 Rust/Lelwel，嵌入机制和委托 API 需要完全重新实现 | **中** | 低 | Phase 0-2 保持 Python，不做中期切换。若 Ph3+ 迁移到 Rust，定义 JSON Schema 作为跨语言 IR 合约 | 3+ |

---

## 十一、治理机制

### 11.1 双周"设计偏差"审查

| 参与者 | 角色 |
|--------|------|
| 技术负责人 | 主持 |
| 编译器实现者 (2-3) | 汇报进展、提出问题 |
| 前端架构师 (1) | 审查代码质量趋势 |

**议程：**
1. 对比实际进展 vs 执行计划（进度偏差 > 1 周 → 升级讨论）
2. **★ 核心环节：** 通过一个**具体、棘手的例子**（而非抽象原则）审视项目是否按设计初衷前进
3. 审查所有 IR 调整请求（通过 `IRVersion` 迭代，不回修改主文档）
4. 审查新增的 `@native` 使用（评估是否可提升为 IR 意图类型）
5. 审查代码质量指标（快照测试通过率、L1 标准自动化检查通过率）
6. 回溯 `DECISIONS.md`：若发现争论的根源是该文档未覆盖的新领域，标记为"设计偏差"信号
7. **★ 新增：** 审查 L1 降级记录——任何降级都必须有明确的修复期限，不可无限期"后续优化"

### 11.2 Phase 完成审查 + 里程碑门控

| 审查项 | 执行人 | 产出 |
|--------|--------|------|
| 任务清单逐项验收 | 技术负责人 | 验收清单（全部打勾） |
| 代码质量对照审查 | 指定框架架构师 | 差异率报告 |
| 行为测试一致性矩阵更新 | 测试负责人 | 更新后的矩阵 |
| IR 版本号确认 | 技术负责人 | IRVersion 升级记录（如有） |
| 设计偏差记录 | 技术负责人 | 偏差清单 + 影响评估 |
| **★ 门控条件核验** | **技术负责人 + 外部用户** | **门控条件逐条确认（见 §八），通过后解锁下一 Phase。若"无外部反馈"触发，按 §8.3 流程处理** |

### 11.3 关键人员备份与知识共享

52 周的项目周期对关键人员的依赖巨大。若技术负责人或核心架构师意外缺席，项目面临极高停摆风险。

| 机制 | 说明 |
|------|------|
| **强制文档化** | 所有关键技术决策、设计思路必须文档化，禁止"口口相传"。决策记录存放于 `docs/decisions/` 目录 |
| **每周技术分享** | 核心实现者轮流讲解自己负责模块的架构和实现细节（30 分钟/周），确保至少 2 人理解每个核心模块 |
| **代码审查轮换** | 非作者审查所有 IR 和 codegen 变更，确保知识分散 |
| **DECISIONS.md 作为北极星** | 当团队在实施中陷入争论时，应能立即回溯此文件 |
| **★ 门控不可绕过** | 即使关键人员缺席，门控条件不可妥协。外部用户反馈缺失时按 §8.3 处理，不可跳过 |

### 11.4 文档更新流程

```
Phase 实施中                Phase 完成后
     │                           │
     ├─ PHASE0_SPEC.md          ├─ 回写细节到主文档
     │  (可更新，活文档)         │  (过长的查找表→独立附录文件)
     │                           │
     ├─ 禁止回修改              ├─ 升主文档小版本
     │  v3.0-final 主文档        │  v3.1, v3.2 ...
     │                           │
     └─ IR 调整→IRVersion       └─ 发布变更摘要
        次版本号迭代
```

---

## 附录 A：里程碑交付物清单

| 里程碑 | Phase | 交付物 | 类型 | 门控 |
|--------|-------|--------|------|------|
| M0 | 启动前 | `IR_VALIDATION_REPORT.md`（4 个金丝雀 case 验证报告） | 文档 | GO/NEEDS_ADJUSTMENT/BLOCKED 决策 |
| M1 | 启动前 | `DECISIONS.md`（哲学对齐决策）+ `PHASE0_SPEC.md`（冻结任务书） | 文档 | — |
| M2 | 0.1 | AST 定义 + TodoMVC 解析通过 + 语义分析 0 错误 | 代码 | — |
| M3 | 0.2 | 8 意图类型 + 6 基础类型 IR 定义 + TodoMVC React TSX 可编译、可运行 | 代码 | — |
| M4 | 0.3 | TodoMVC Zustand Store 生成 + tsc 通过 | 代码 | — |
| M5 | 0.4 | CSS Modules 生成 + 浏览器渲染正确 | 代码 | — |
| M6 | 0.5 | IR 快照 + codegen 快照 + 诊断测试 + 5 行为用例 Playwright CI 通过 | 测试 | — |
| M7 | 0.6 | 20+ fixture + 2 应用 + 代码质量审查报告（差异率 < 5%）+ CLI 可用 | 测试 + 工具 | **★ Phase 0 门控：外部 React 开发者试用反馈** |
| M8 | 1 | Vue 3 TodoMVC + HMR + 行为测试通过 + 外部 Vue 开发者反馈 | 代码 | **★ Phase 1 门控** |
| M9 | 2 | Dioxus TodoMVC + borrow checker 通过 + 行为测试通过 + 社区反馈 | 代码 | **★ Phase 2 门控** |
| M10 | 2a | Flutter 10 元素可渲染 + 5 行为用例通过 + IR 缺陷报告 | 代码 + 审查 | IRVersion 升级决策 |
| M11 | 3 | 14 项能力深化完成 + Flutter L1 达标 + 跨框架矩阵 10 用例 | 代码 | **★ Phase 3 门控** |
| M12 | 4 | Flutter L2 + 框架辅助库 + L3 视觉回归 + 文档上线 | 代码 + 文档 | **★ Phase 4 门控（项目完成）** |
| M13 | 2a 后 | SolidJS/Svelte 范式抵御原型报告 | 研究 | — |

---

> **本执行计划基于 `OmniLang_v0.3_设计方案.md` v3.0-final（已冻结），经 Council 评审后调整（2026-06-04）。**  
> **关键调整：** ① Phase 0 从 12 周压缩至 8 周→**Council 修正为 10 周**（增加 OmniLang 委托基础设施） ② IR 原型验证前置至启动期 ③ IR 预埋字段从 27 精简至 8 ④ 新增里程碑门控机制 ⑤ 新增 11 项风险（R9-R19） ⑥ **新增 §1.0 OmniView→OmniLang 架构依赖定调** ⑦ **新增 §〇 设计文档依赖关系章节**  
> **Phase 0 启动前请完成 §一 的 IR 原型验证和准备工作。**  
> **任何对本计划的调整需通过双周设计偏差审查会议讨论。**

---

## 附录 B：技术选型参考

> **编制日期：** 2026-06-04 | **来源：** Deep Research（14 个查询，分析 30+ 来源）  
> **用途：** 为 Phase 0-4 各阶段的实现技术选型提供参考。最终技术选型由团队在对应 Phase 启动前确定。

### B.1 编译器实现语言选择

**★ 重要前提：OmniLang 编译器已用 Python 实现（`omnilang/` 目录），包括 Parser、Lexer、类型检查器、Codegen。OmniView 在此基础上构建，不重复实现语言层功能。**

Phase 0 设计文档和 IR 定义使用 Python（`@dataclass`），与 OmniLang 实现语言一致。两个可行路径：

| 路径 | 实现语言 | 优势 | 劣势 |
|------|---------|------|------|
| **A: Python（当前）** | Python 3.12+ | 与 OmniLang 同语言，直接调用 `omnilang.parser`/`omnilang.semantic`/`omnilang.codegen`，零额外对接成本 | 性能上限低，发布为 CLI 需 PyInstaller |
| **B: Rust（远期）** | Rust 1.80+ | 极致性能，`serde` 序列化 IR，可直接编译为原生 CLI 二进制 | OmniLang 也需迁移到 Rust。IR 定义使用 JSON Schema 作为跨语言合约 |

**建议：** Phase 0-2 保持 Python（与 OmniLang 共享代码库，避免跨语言 IPC 开销）。若 Phase 3 后性能成为瓶颈，OmniLang + OmniView 同步迁移到 Rust。

### B.2 解析器 / Lexer 技术选型

执行计划 §0.1 要求构建 `.omni` 自定义语法解析器，含**错误恢复**能力。2025-2026 年主流工具对比：

| 工具 | 语言 | 策略 | 错误恢复 | 推荐场景 |
|------|------|------|---------|---------|
| **[Lelwel](https://lib.rs/crates/lelwel)** | Rust | LL(1) 递归下降 | ★★★ 自动计算恢复集，生成 lossless CST | **首选（Rust 路径）** |
| **[Chumsky](https://lib.rs/crates/chumsky)** | Rust | Parser Combinators (PEG/CFG) | ★★★ 可插拔 `Strategy` trait，多策略可组合编排 | 需要 PEG 灵活性的复杂语法 |
| **[Gibberish](https://research.tedneward.com/langdev/gibberish/)** | Rust/C | 声明式 `.gib` 语法文件 | ★★★ 错误是第一类 CST 节点，始终产出树 | IDE/LSP 优先的工具链 |
| **[Peggy](https://www.npmjs.com/package/peggy)** | JavaScript | PEG | ★★ 生成 `expected` token 列表 | JS 生态、轻量 DSL |
| **ANTLR4** | Java（多目标运行时） | Adaptive LL(*) | ★★ `ErrorListener` 接口 | 复杂语法、需要可视化语法图 |
| **手写递归下降** | 任意 | — | ★ 完全可控但开发量大 | 语法简单（<20 条规则）时 |

**OmniView 的语法特征分析：**
- `.omni` 语法约 30-40 条产生式规则（component/store/view/style/theme + @指令 + 安全表达式 + 事件处理器）
- 关键字有限（~15 个上下文关键字 + ~8 个类型关键字）
- 需要精确的错误恢复（LSP/IDE 场景）
- **推荐：** Rust 路径选 **Lelwel**，Python 路径选手写递归下降 + 参考 Lelwel 的恢复策略

### B.3 跨框架编译的先行者参考

OmniView 不是第一个尝试跨框架编译的项目。以下先行者提供了宝贵的架构参考：

| 项目 | 架构 | Star | 借鉴点 |
|------|------|------|--------|
| **[Mitosis](https://github.com/BuilderIO/mitosis)** (Builder.io) | JSX-like `.lite.tsx` → JSON IR (`MitosisComponent`) → 14 个框架 codegen | 13.6k+ | **最接近 OmniView 的参考架构。** JSON IR 解耦解析与生成，插件系统支持 before/after hooks，`overrides/<framework>/` 处理不可泛化的代码 |
| **[Stencil](https://stenciljs.com/)** (Ionic) | TSX → 编译器 → Web Components + Framework Wrappers | 12k+ | **"Output Targets" 模式。** 单套源码 → 多格式输出（React wrapper / Vue wrapper / Angular module / lazy WC），Apple Music 等企业级验证 |
| **[Style Dictionary](https://github.com/amzn/style-dictionary)** (Amazon) | JSON/YAML 设计令牌 → 多平台样式代码 | 4.4k+ | **跨平台设计令牌 codegen。** OmniView 的 `theme { }` 块设计参考了其 CTI 层级结构和自定义格式系统 |
| **Svelte 5 Compiler** | `.svelte` → 编译器 → 原生 JS（无 VDOM） | — | **编译期响应式系统。** `$state` runes 的编译策略是"细粒度响应式 → 直接 DOM 更新"编译模式的参考 |

**Mitosis 的关键教训（对 OmniView 的启示）：**
1. **JSON IR 是正确方向**——Mitosis 的 `MitosisComponent` JSON AST 证明了框架无关 IR 的可行性
2. **Override 目录不是设计缺陷**——Mitosis 的 `overrides/` 目录与 OmniView 的 `@native` 语义一致：承认不可泛化的代码，给它正式的位置
3. **14 个目标框架的维护成本极高**——Mitosis 112 个贡献者维护 14 个 framework generator，OmniView 的 4 个目标更务实
4. **Deutsche Bahn 的设计系统**是 Mitosis 在 production 中的验证——说明企业设计系统确实是跨框架编译的最佳适用场景

### B.4 代码生成引擎

Codegen 需要将 IR 展开为目标框架的源代码字符串。两种策略：

| 策略 | 工具 | 优势 | 劣势 |
|------|------|------|------|
| **模板引擎** | [Tera](https://lib.rs/crates/tera) (Rust, Jinja2-like), [stringtemplate4ts](https://www.npmjs.com/package/stringtemplate4ts) (TS) | 模板与逻辑分离，非程序员可读，热重载 | 复杂展开逻辑需自定义 filter/function |
| **字符串拼接/Builder 模式** | 直接构建 | 最大灵活性，可做 AST 级优化 | 代码可读性差，缩进管理复杂 |

**推荐：** 采用**混合模式**——简单映射（ElementIR→JSX 标签名、语义属性→CSS 属性）使用**查找表驱动**的字符串拼接；复杂展开（Flutter `TextEditingController` 生命周期、Dioxus `Clone`/`PartialEq` derive）使用**模板引擎**（Tera 或 stringtemplate4ts）。

Tera 的 Jinja2-like 语法天然支持：
```jinja2
// React codegen 模板示例
{% for stmt in block.statements %}
/* .omni:{{ stmt.source_line }} */
{% match stmt %}
{% when MutationStmt with { target, operation: Increment(amount) } %}
set{{ target.name | pascal_case }}({{ target.name }} + {{ amount }});
{% when MutationStmt with { target, operation: SpreadAppend(item) } %}
set{{ target.name | pascal_case }}([...{{ target.name }}, {{ item }}]);
{% endmatch %}
{% endfor %}
```

### B.5 样式 Codegen 参考

| 工具 | 用途 | 借鉴点 |
|------|------|--------|
| **[Lightning CSS](https://lightningcss.dev/)** (Rust) | CSS parser/transformer/minifier | 100x 速度优势。语义属性→CSS 查找表（~50 条）的实现可参考其属性展开逻辑 |
| **[Style Dictionary](https://github.com/amzn/style-dictionary)** | 设计令牌→多平台代码 | `transformGroup` 机制可复用：同一语义属性在不同 `output_mode`（CSS/Widget/Inline）下展开为不同代码 |
| **Tailwind CSS v4** | Utility-first CSS 框架 | 采用 Lightning CSS 替代 PostCSS 的做法验证了 Rust 工具链在生产环境中的可行性 |

OmniView 的语义属性查找表（`layout: column` → `display: flex; flex-direction: column`）本质上是 Style Dictionary "transform" 的编译期版本：输入是 `.omni` 的语义属性 token，输出是目标框架的样式代码字符串。

### B.6 测试技术选型

| 测试类型 | 推荐工具 | 适用场景 |
|---------|---------|---------|
| **IR 快照测试** | [insta](https://lib.rs/crates/insta) (Rust) / `pytest-snapshot` (Python) | IR→JSON 序列化快照，自动 diff 更新 |
| **Codegen 输出快照** | insta / Jest snapshots | 输入 IR → 期望目标代码字符串比对 |
| **诊断测试** | 自定义 assertion | 触发 P 级规则 → 验证 ERROR/WARNING 严重度 + 行号 |
| **行为测试** | Playwright (React/Vue) / `flutter_test` / `dioxus::ssr` | 编译产物 → 框架原生测试工具验证运行时行为 |
| **L3 视觉回归** | Percy / Playwright screenshots | P4: React DOM 截图 vs Flutter Widget 截图 |
| **属性测试** | proptest (Rust) / Hypothesis (Python) | 随机生成合法 `.omni` 片段 → 验证编译不 panic |

**insta 的核心优势：** 快照文件与源码一起提交，CI 中自动 diff。快照更新只需 `cargo insta review`（Rust）或 `pytest --snapshot-update`（Python）。

### B.7 CLI 工具技术选型

执行计划 §0.6（原 0.9）要求构建 `omnic` CLI 工具。

| 工具 | 语言 | 特点 |
|------|------|------|
| **[clap](https://docs.rs/clap)** | Rust | 性能最佳，derive API 声明式，自动生成 help/man 页面 |
| **[commander](https://www.npmjs.com/package/commander)** | Node.js | 生态成熟，子命令支持好 |
| **[click](https://click.palletsprojects.com/)** | Python | 装饰器 API，适合 Python 原型 |

**推荐：** Phase 0 Python 原型用 **click**（或 **typer**），若后续迁移到 Rust 则切到 **clap** 的 derive 模式。

### B.8 Source Map 生成

执行计划 Phase 1 要求 source map 支持。2025 年主流方案：

| 工具 | 语言 | 特点 |
|------|------|------|
| **[source-map-js](https://deepwiki.com/7rulnik/source-map-js)** | JavaScript | 性能优化的 Mozilla source-map fork，`SourceMapGenerator` + `SourceNode` API |
| **[oxc_codegen](https://lib.rs/crates/oxc_codegen)** | Rust | 基于 esbuild 的 codegen，原生 source map 生成 |

**策略：** Phase 0 仅生成 `/* .omni:行号 */` 行号注释（执行计划 0.2.4 已覆盖）。Phase 1 实现真正的 source map 生成——React 目标使用 `source-map-js` 的 `SourceMapGenerator` API 在 codegen 过程中增量添加 position mapping。

### B.9 未来 LSP/IDE 支持技术预留

执行计划 Phase 0 的 parser 必须包含**错误恢复**（原 0.1.2 要求），这为远期 LSP 支持奠定了基础。2025 年 LSP 技术栈：

| 层 | 技术 | OmniView 对应 |
|----|------|-------------|
| 增量解析 | Tree-sitter | Lelwel 的 lossless CST 或手写递归下降的错误恢复 |
| 语义分析 | LSP Server | `omnic check` 的诊断引擎（P0 实现） |
| 语法高亮 | Tree-sitter grammar / TextMate grammar | `.omni` 的 Tree-sitter grammar（社区贡献，非 Phase 0-4） |
| 自动补全 | LSP Completion | 符号表 + 类型信息（P0 已有符号解析） |

**关键认知：** P0 的 parser 如果实现了设计文档要求的"错误恢复 + 局部 AST + 诊断列表"，那么它已经是 LSP 所需的核心基础设施。Tree-sitter grammar 可以由社区后续贡献——Zed 编辑器的 WASM-based grammar 加载机制为此提供了低摩擦的分发路径。

### B.10 技术选型总结

| 组件 | P0 推荐 | P1+ 推荐 | 备选 |
|------|---------|----------|------|
| 编译器语言 | Python 3.12+ | Rust 1.80+ | TypeScript (via tsx) |
| Parser | 手写递归下降 + Lelwel 恢复策略参考 | Lelwel 或 Chumsky | Peggy (JS) |
| IR 序列化 | `dataclasses` + JSON | `serde` + JSON | protobuf (如需二进制) |
| Codegen 模板 | 查找表 + 直接字符串构建 | Tera (Rust) | stringtemplate4ts |
| 测试—快照 | pytest-snapshot | insta | Jest snapshots |
| 测试—行为 | Playwright | Playwright + flutter_test + cargo test | — |
| CLI | click / typer | clap (derive) | commander |
| Source Map | `/* .omni:N */` 注释 | source-map-js | oxc_codegen |
| 样式 codegen | 内置查找表（~50 条规则） | Lightning CSS (仅 Web 目标) | — |
| CI/CD | GitHub Actions | GitHub Actions + framework version matrix | — |

> **决策纪律：** 每项技术的最终选型在对应 Phase 启动前 1 周确定，记录在 `docs/decisions/` 目录（ADR 格式）。上表中的"推荐"是基于 2026-06-04 技术生态的快照——Phase 1+ 启动前应重新评估当时的最新版本和社区趋势。
