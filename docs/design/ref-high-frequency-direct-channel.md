# ref 高频操控——三目标直写通道设计

**Status**: 三阶段全部实施完成（Phase 1 React 直写 f819ba5f..92fea90a；Phase 2 @drag 意图化 6efd6853..4d40afed；Phase 3 Flutter 表达式化 ca502c54..4cccf1a8）| **Date**: 2026-08-07 | **Related**: ADR-070, ADR-071, ADR-073, element-ref-system v2

## 1. 背景与动因

ADR-070 三目标机制契约实测暴露：同一 `.omni` 源码（`@ref` + 信号 + `style: { expr }` 绑定）在三目标的性能语义**不齐**：

| 目标 | 现状机制 | 性能语义 | 证据 |
|------|---------|---------|------|
| OmniView | `el.style.transform = ...` 直接 DOM 写 | 零 reconciliation ✅ | build/index.js 产物实测 |
| React | `useState` → 整组件重渲染 → `style={{...}}` | 每帧全组件 reconcile ⚠️ | ElementRefDemo.tsx 产物实测 |
| Flutter | 动态 style 值**静默跳过** | 拖拽位移不渲染 ❌ | view.rs inline_style `as_str().unwrap_or("")` |

用户哲学确认（ADR-071 决策 1）：**ref 是语义级语法——源码表达前端功能意图，三目标编译器自行选路实现**。`style: { transform: expr }` 的语义是「元素的该属性**恒等于**该表达式」（ADR-071 决策 2），该语义在三目标都必须兑现，而不是靠目标巧合。

本设计将 ADR-071 的实现路线具体化：**Phase 1 React 直写通道（优先）→ Phase 2 capturePointer 意图化（独立 ADR）→ Phase 3 Flutter inline_style 表达式化**。

## 2. 语义定义：style 动态绑定「恒等于」

### 2.1 语义契约（三目标共有，ADR-071 决策 2 形式化）

```
style: { prop: expr }（元素绑定 @ref 时）
  语义：元素的 prop 属性恒等于 expr 的当前值
  - expr 变化 → 元素 prop 必须跟随（无显式时机承诺，但不得晚于同一事件循环）
  - expr 不变 → 元素 prop 不得被框架改动（reconciliation 不得覆盖它）
  - 元素卸载/重建 → prop 随元素重建恢复
```

三目标实现指引（编译器的选路，非语言差异）：

| 目标 | 实现 | 性能语义 |
|------|------|---------|
| OmniView | 直写（现状，保持） | 零 reconciliation |
| React | **本设计 Phase 1**：useRef + 事件回调/effect 直写 `el.style`，属性不进 style prop | 零 reconciliation |
| Flutter | **本设计 Phase 3**：ListenableBuilder 包裹 + Transform | 局部重建零整树 diff |

### 2.2 与 v2 决策的修订关系

- v2 决策「`ref.style` 写是编译错误」**保持**——它防的是命令式逃逸，不是本通道。本通道激活的是**声明式 `style: { expr }` 绑定**，编译器自行选路直写，语言层无任何新语法。
- element-ref-system.md:388-393 注记「React 目标退化为 useState + style={{...}}」**修订为本设计的 Phase 1 产出**（该注记的 v2 代价陈述被撤销）。

### 2.3 激活条件（三目标通用）

style 条目进入直写通道，当且仅当**同时满足**：

1. 元素绑定了 `@ref`（`ElementIR.ref_binding = Some`）
2. 条目值为动态表达式（含 `$signal`/`$ident`/`$field`/`$call`/`$concat` 元对象，即 `shared/style.rs:34 format_style_value_expr` 的判定集）
3. 元素不在 `@for` 内（v2 Phase 2 未支持 @for+ref，保持不激活）

不满足 → 维持现状路径（React：useState + style prop；Flutter：静态字符串直译或缺口登记）。

## 3. 三目标手写对照样例

同一功能意图——「指针拖动时元素 transform 恒等于位移信号」，三目标工程师的地道手写：

**React（useRef + 直写，本设计 Phase 1 目标产物形态）**：

```tsx
const dragXRef = useRef(0);
const dragYRef = useRef(0);
const dragBoxRef = useRef<HTMLDivElement>(null);

const onDragMove = (e: React.PointerEvent<HTMLDivElement>) => {
  if (!dragging) return;
  dragXRef.current = e.clientX - dragOriginXRef.current;
  dragYRef.current = e.clientY - dragOriginYRef.current;
  // 直写：属性不进 style prop，避开重渲染覆盖
  dragBoxRef.current?.style.transform = `translate(${dragXRef.current}px, ${dragYRef.current}px)`;
};
```

**Flutter（ValueNotifier + ListenableBuilder，本设计 Phase 3 目标产物形态）**：

```dart
final _dragX = ValueNotifier<double>(0);
final _dragY = ValueNotifier<double>(0);

// widget 树中：
ListenableBuilder(
  listenable: Listenable.merge([_dragX, _dragY]),
  builder: (_, __) => Transform.translate(
    offset: Offset(_dragX.value, _dragY.value),
    child: dragBox,
  ),
);
// 拖动回调：_dragX.value = ...（不 setState，零整树 build）
```

**OmniView（现状，保持）**：

```js
// 产物形态：el.style.transform = ... 直接 DOM 写（build/index.js 实测）
onDragMove(e) {
  if (!this._dragX()) return;
  __omni_el("el-div-042").style.transform = `translate(${e.x - origin}px, ...)`;
}
```

**同一 .omni 源码**（learn-omniview ElementRefDemo 拖拽区，三目标共享，不因目标而变）：

```omni
<div
  @ref={dragBox}
  @onPointerDown={onDragDown}
  @onPointerMove={onDragMove}
  @onPointerUp={onDragEnd}
  @onPointerCancel={onDragEnd}
  style: { transform: "translate(" + dragX + "px, " + dragY + "px)" }
>
```

---

## 4. Phase 1：React 直写通道

> **实施状态：Phase 1 已实施（commit f819ba5f，Task 1-5；浏览器实测 Task 6）**。产物审计（learn-omniview ElementRefDemo.tsx）：`dragX`/`dragY` 无 `useState`（单 `useRef` 存储）、`onDragMove` 内无 `setDragX`（`dragXRef.current = ...` 直写 2 处 + apply 直写）、DragBox 无 `style` prop（transform 条目已剔除）、B2 effect 直写（boxWidthPx）；verify-react GATE PASSED、tsc 72 基线不变、react 套件 442/442 全绿。浏览器实测：拖拽精确跟随指针（15 次 pointermove → `translate(60px, 30px)` 像素级一致），pointermove 期间零渲染提交（详见 §4.5.5）。

### 4.1 激活判定（React 目标）

在 `react/component/mod.rs` 生成流程中新增一 pass：对每个元素，若满足 §2.3 三条件，则该元素的**动态 style 条目**进入直写通道——从 `style={{...}}` prop 剔除，改为直写发射。

判定输入（全部已有基建）：
- `el.ref_binding`（`view.rs:839-842` 已发射 `ref={nameRef}`）
- `el.inline_style` 条目值经 `format_style_value_expr` 的**动态判定**（`shared/style.rs`）
- `@for` 上下文（遍历 view 树时已知）

### 4.2 信号域分类

直写条目的信号按**消费面**分类（决定存储与发射路径）。分类 pass 复用 `state.rs:753-859 collect_view_read_signals` + `state.rs:260 collect_signal_mutations`：

| 域 | 判定 | 存储 | 发射 |
|----|------|------|------|
| **D1 直写域** | 信号**所有读**都在激活直写条目的值内（无文本/class/prop/条件读） | `useRef` | mutation 点注入 apply |
| **D2 写专用** | 信号无输出读（仅 mutation RHS 自读，如 dragOriginX） | `useRef` | mutation 直接写 ref（无 apply） |
| **B2 共享域** | 有渲染读（文本/class/prop/非直写 style） | `useState`（保持） | `useEffect([deps])` 直写 |

**D1 判定细节**：直写域信号**不得**被 B2 条目或任何渲染位置读——否则其更新必须走渲染路径（useRef mutation 不触发渲染，render 会读到陈旧值）。判定 = 信号的所有读位置 ⊆ 激活直写条目的值表达式集。

**demo 实况映射**（学习样例即契约样例）：

| 信号 | 消费 | 域 | 产物变化 |
|------|------|----|---------|
| `dragX`/`dragY` | 仅 transform 绑定 | D1 | `useState` → `useRef`；handler 直写 |
| `dragOriginX`/`dragOriginY` | 仅 mutation RHS 自读 | D2 | `useState` → `useRef`（pointerdown 零重渲染） |
| `dragging` | class 绑定 + if 条件 | B2 | 保持 useState |
| `boxWidthNum`/`boxHeightPx` | 文本显示 + range bind + 绑定 | B2 | 保持 useState + effect 直写 |

### 4.3 发射设计

#### 4.3.1 存储（gen_ref_hooks 扩展）

`react/component/mod.rs:802 gen_ref_hooks` 之后新增 **gen_direct_write_storage**：D1/D2 信号发射为

```tsx
const dragXRef = useRef<number>(0);   // 初始值取信号 initial_value
```

（D2 若无初值 → `useRef(0)` 或 `useRef(undefined as T)`，按现有 initial 翻译。）

**hook 选择联动**（mod.rs:207-229）：D1/D2 信号全部从 `mutated_signals` 的 useState 池剔除（它们在 `read_signals` 里仍出现，但存储判定改为「D1/D2 → useRef」）——否则 useState 与 useRef 双存储。**注意与 write-only DCE（mod.rs:151-175 pruned_signals）的交互**：D1 信号有 style 消费读，不满足剪枝条件（`!value_consumed` 为假），天然保留——本通道把它们的「消费」从渲染路径改为直写路径，需在 `has_mutable_states` 判定（mod.rs:172-175）中把 D1/D2 从 useState 需求中扣除。

#### 4.3.2 apply 直写（D1）

每条激活条目生成一个 apply 语句模板（表达式翻译复用 `format_style_value_expr` 的 `$field/$call/$concat` 路径，但信号标识符解析为 `{name}Ref.current`）：

```tsx
// 内联于 mutation 注入点（每 D1 信号 mutation 后）：
dragXRef.current = e.x - dragOriginXRef.current;
dragBoxRef.current?.style.transform = `translate(${dragXRef.current}px, ${dragYRef.current}px)`;  // ← apply
```

**发射规则**：
- **注入点**：对 D1 信号赋值的语句之后（statement 发射处，`react/component/function.rs` + `state.rs` 的 effect/lifecycle 发射）追加 apply——复用 `state.rs:260 collect_signal_mutations` 的扫描结果，将 mutation 目标与直写条目信号取交集
- **批量收敛**：一个 mutation 语句同时命中多个 D1 信号时，**只注入一次**（demo 的 `dragX = ...; dragY = ...` 两语句各注一次，第二次的 apply 读到最新双值——语义等价，重复写一次成本 μs 级，接受）
- **null 安全**：`{ref}Ref.current?.style.{prop}` 可选链（挂载前事件不会触发；生命周期 mutation 在挂载后）
- **sanitize**：值含 CSS 字符串拼接（$concat）时复用 `react/design.rs:411-485 emit_sanitize_css` 的模块级 `sanitizeCss` helper（与 OmniView `__omni_sanitizeCss` 同逻辑，单一来源注记）

#### 4.3.3 B2 effect 直写

对每个 B2 条目（元素 ref + prop + 值），在 `react/component/mod.rs:867 gen_semantic_binding_effects` 之后新增 **gen_ref_style_effects**——完全复用 `state.rs:1162-1367 emit_element_binding_effects` 的既有形态（focus/scroll-y 直写 effect 先例）：

```tsx
useEffect(() => {
  sizeBoxRef.current?.style.width = `${boxWidthPx}px`;
}, [boxWidthPx]);   // deps = 条目读的信号（useState 态）
```

**style prop 剔除**：B2/D1 条目都不进 `style={{...}}`（view.rs:718-731 发射处按激活集过滤）——消除「effect 直写 vs render 重写」双路径覆盖竞态。

#### 4.3.4 与 pruned_signals 的交互汇总

| 信号域 | read_signals | pruned 判定 | 本通道后 |
|--------|--------------|------------|---------|
| D1 | 在（style 读） | 不剪（值被消费） | useRef，mutation 注入 apply |
| D2 | 在或不在 | 现行为不变量 | useRef（替代「剪枝后无存储」或「useState 浪费」两态） |
| B2 | 在 | 不剪 | useState + effect 直写 |

### 4.4 边界案例

| 案例 | 处理 |
|------|------|
| 静态 + 动态 style 条目共存 | 静态条目留 style prop；动态条目直写（互不干扰） |
| computed 派生（boxWidthPx） | 走 B2：computed 有渲染读 → useState + effect 直写，deps 含 computed 值 |
| store 字段 | 走 B2（store 跨组件共享，必须保持响应式：effect 读 store 值直写） |
| @for 内 @ref | 不激活（§2.3 条件 3） |
| 元素在 @if 分支 | 激活；卸载后 `ref.current` 为 null，`?.` 兜底；重建后 effect 重跑恢复 |
| @mounted 内改 D1 信号 | 生命周期 mutation 发射点同样注入 apply（挂载后 ref 已就绪） |
| React StrictMode 双 effect | apply 幂等（重复赋值同值），无碍 |
| 组件 ref（forwardRef） | 不激活（v2 未支持组件级 ref，保持） |

### 4.5 验证

1. **verify-react.sh**：直写通道产物无新错误类（D1 信号 useRef 化后无 setState 残留、无 TS18048 possibly-undefined——useRef 有初值）
2. **react 测试套件**（196/196 基线）+ compilation_test
3. **产物审计断言**（learn-omniview ElementRefDemo.tsx）：
   - `dragX`/`dragY` 无 `useState`，有 `useRef` 存储
   - `onDragMove` 内无 `setDragX`，有 `dragXRef.current = ...` + apply 直写
   - `style={{ ... }}` 中无 `transform` 动态条目
   - 有 B2 effect（boxWidthPx 直写）
4. **浏览器实测**：拖拽跟随与 OmniView 同级流畅；React DevTools Profiler 断言 pointermove 期间**零组件渲染提交**
5. **浏览器实测记录（2026-08-07 Task 6，React DevTools global hook `onCommitFiberRoot` 提交计数）**：
   - 两轮拖拽（15 次 `+4/+2` 移动、8 次 `-5/-3` 移动）：transform 逐帧精确 `translate(20px,10px)`/`translate(60px,30px)`/`translate(-40px,-24px)`，computed matrix 与 DOM rect 位移像素级一致
   - 提交计数：pointerdown → +1（`dragging` B2 信号）、**pointermove ×N → 0 提交（`scheduleCount` 亦为 0）**、pointerup → +1（`dragging` 复位）——D1 通道零 reconciliation 成立
   - `dragging` class 切换（DragBoxActive）走 B2 正常渲染路径，观察期内无异常

---

## 5. Phase 2：capturePointer 意图化（独立 ADR）

> 本设计只给语义草案与三目标映射方向；正式决策走**独立 ADR**（ADR-071 决策 3 要求）。

> **实施状态：Phase 2 已实施（ADR-073 + commit 6efd6853..4d40afed，Task 7-11）**。@drag 意图语义全链落地：parser/AST/IR/lowering/语义验证 → OmniView（setPointerCapture + dx/dy 合成）/React（per-ref useRef 状态 + setPointerCapture）/Flutter（GestureDetector onPan）codegen → demo 迁移（ElementRefDemo 无显式 capturePointer）+ D3 gate 更新（过滤 → 零诊断断言）。三目标 952/952 all-features，React GATE PASSED，Flutter EXIT=0。

### 5.1 问题

`capturePointer()`/`releasePointer()` 是机制泄漏（ADR-071 决策 3）：OmniView/React 有原生对等物（setPointerCapture），Flutter 渲染树无「指针捕获」原语（move 事件靠命中链隐式路由）——spec §10.3 设计 D3 硬错误是语义对齐失败的诚实暴露。机制映射（给 Flutter 找等价）论证困难；**语义上移**（定义「可拖拽」意图）路径易行。

### 5.2 @drag 语义草案

```
@drag={onDrag} @dragEnd={onDragEnd}           ← 元素可拖拽意图
  语义：指针在元素上按下 → 捕获拖动 → 拖动期间持续回调 onDrag
        指针移出元素仍跟随（隐式捕获语义）→ 抬起回调 onDragEnd
  事件数据：{ x, y, dx, dy, pointerId, startX, startY }
```

三目标实现方向：

| 目标 | 实现 |
|------|------|
| OmniView | 现有原语组合（capturePointer + pointermove 直写）——capturePointer 降级为编译器内部细节 |
| React | setPointerCapture + handler 直写（同为内部细节） |
| Flutter | GestureDetector.onPanStart/onPanUpdate/onPanEnd（意图语义的 Flutter 地道实现） |

**demo 改造**：ElementRefDemo 拖拽区从显式 `capturePointer`/`releasePointer` + 三事件 → `@drag` 意图（编译器选路）；capturePointer 教学区改造或标注为目标差异注记。

### 5.3 演进

- spec §10.3 设计 D3：硬错误 → 目标差异注记（「capturePointer 是内部实现细节；Flutter 用户面向 @drag 语义」）
- 若源码仍显式写 capturePointer → 保留 D3 硬错误 + 迁移提示「use @drag 意图 instead」（防机制泄漏回潮）
- flutter gate（generated_code_validation.rs:704-711, 1302-1309 的 D3 过滤）随 demo 改造同步收敛

---

## 6. Phase 3：Flutter inline_style 表达式化

依赖 Phase 2 的 @drag 语义定义（demo 需先去掉显式 capturePointer 才能编译）。

> **实施状态：Phase 3 已实施（commit ca502c54..4cccf1a8，Task 12-15）**。共享分类 pass（shared/direct_write.rs）→ Flutter D1/D2 信号 ValueNotifier 存储 + mutation 直写（`_dragX.value = ...`，零 setState）→ ListenableBuilder 包裹 + Transform 映射 + 不可映射编译期显式诊断（替代静默跳过）。产物审计（ElementRefDemo.dart）：`ListenableBuilder(listenable: Listenable.merge([_dragX, _dragY]), ...)` + `Transform.translate(offset: Offset(_dragX.value, _dragY.value), ...)`；flutter 行为 gate（generated_code_validation.rs `learn_omniview_flutter_phase3_drag_behavior`）断言生成代码含 ListenableBuilder/Listenable.merge/Transform.translate/ValueNotifier；`dart analyze` 0 errors；demo 信号改 f64（String→double 类型缺口显式诊断后）。全量 758/758 all-features。

### 6.1 现状根因

`flutter/view.rs:1597-1689` inline_style：`css_val.as_str().unwrap_or("")` → 动态值 → 空字符串 → **静默跳过**（ADR-070 缺口）。信号 = State 字段 + setState → 整树 build（高频不可行）。

### 6.2 改造设计

沿用 Phase 1 的**信号域分类**（同一判定 pass，跨目标共享）：

| 域 | Flutter 存储 | Flutter 发射 |
|----|-------------|-------------|
| D1/D2 | `ValueNotifier<T>` 字段（替代 setState 信号） | mutation 发射 `_dragX.value = ...`（`flutter/component` 的 statement 发射处）；无 setState |
| B2 | setState 信号（保持） | ListenableBuilder 包裹 + effect 接线 |

**条目发射**：动态 style 条目 → 元素包裹 `ListenableBuilder(listenable: Listenable.merge([...notifiers]), ...)`，builder 内按**属性映射表**重建参数——局部重建，零整树 build。

### 6.3 CSS 属性 → widget 映射表（实况，Task 14/15 后）

| CSS 属性 | widget 参数 | 状态 |
|----------|------------|------|
| `transform: translate(dx, dy)` | `Transform.translate(offset: Offset(dx, dy))`（外层再包 ListenableBuilder） | ✅ 已实施（`match_style_pattern` 识别 5 段/尾融合/首融合三种 concat 形态） |
| `width`/`height` | widget 参数直传（`widget_allows_param` whitelist 校验） | ✅ 已实施 |
| `padding`/`margin` | `EdgeInsets.all(v)` | ✅ 已实施 |
| `opacity` | `Opacity(opacity: v)` | ❌ 未实施——编译期诊断（error_at，诚实契约） |
| `color`/`background-color` | 颜色参数 | ❌ 未实施——编译期诊断 |
| 其他 | **编译期诊断**（替代静默跳过——缺口升级为显式错误，诚实契约） | ✅ 已实施（error_at；String→double 缺口亦显式诊断） |

String 信号读 numeric 参数（width/height/EdgeInsets/Offset）→ 编译期显式诊断（String→double 类型缺口，不再发射坏代码）。

### 6.4 关键取舍：transform 字符串模式解析 vs 属性级映射

`"translate(" + dragX + "px, ...)"` 是 CSS 字符串——Flutter 无 CSS。两案：

- **A. 字符串模式解析**：识别绑定表达式形状（concat 模板匹配 `translate(...)`）→ 提取数值信号 → Transform.translate。优点：demo 源码不变；缺点：模式匹配脆弱（任意 CSS 字符串不可解析）
- **B. 属性级映射（推荐）**：@drag 意图化后，位移走 `@drag` 的 dx/dy 事件数据（天然数值），transform 字符串绑定退为次要路径；无法映射的属性 → 编译期诊断。优点：意图语义为权威（ADR-071 编译器责任模式）、无脆弱解析；缺点：demo 源码需随意图化调整

**推荐 B**：位移类高频操控在 Phase 2 后已有 @drag 数值通道，字符串 transform 绑定不再需要 Flutter 直译——语义上移解决的是「意图」，不是「CSS 字符串」。

### 6.5 gate 演进

`generated_code_validation.rs`：从「过滤 D3 诊断」→ **已实施 flutter 行为验证**（`learn_omniview_flutter_phase3_drag_behavior`：确定性内容断言——生成代码含 `ListenableBuilder` + `Listenable.merge` + `Transform.translate` + `ValueNotifier`，不依赖 dart 工具链；`learn_omniview_flutter_output_passes_dart_analyze`：拖拽项目全量 `dart analyze` 0 errors）。D3 过滤已随 demo 改造移除——`compile_learn_omniview_flutter` 改为断言 D3 诊断数为零（显式 capturePointer 回潮即回归）。

---

## 7. 排期与验证矩阵

| 阶段 | 内容 | 依赖 | 验证 | 状态 |
|------|------|------|------|------|
| P1 | React 直写通道（§4） | 无 | verify-react 无新错误类 + react 套件 + 产物审计 + 浏览器 60fps 实测 | ✅ 已实施（f819ba5f..92fea90a） |
| P2 | @drag 意图语义（§5） | 独立 ADR 评审 | ADR 通过 + demo 无显式 capturePointer | ✅ 已实施（6efd6853..4d40afed，ADR-073） |
| P3 | Flutter 表达式化（§6） | P2 | flutter 行为 gate + 拖拽位移渲染实测 | ✅ 已实施（ca502c54..4cccf1a8） |

**跨阶段不变量**（ADR-070 三目标机制契约为行为基准，任何阶段不得破坏）：
- OmniView 直写路径不变
- React 门禁不新增豁免（修复而非豁免堆积）
- Flutter 无静默跳过（要么实现、要么显式诊断）

## 8. 风险与失败方案

| 风险 | 失败方案 |
|------|---------|
| D1 判定漏算消费面（信号在隐蔽位置被读 → 直写读陈旧值） | 保守策略：判定任何不确定 → B2（useState + effect 直写，语义等价仅性能降级）；D1 判定 pass 单测先行 |
| 双路径竞态（style prop 与直写同属性） | 硬规则：激活条目永远从 style prop 剔除（§4.3.3），产物审计断言守护 |
| pruned_signals 交互破坏（D1/D2 信号被剪枝或 setState 残留） | has_mutable_states 判定联动（§4.3.1）；react 套件 + 产物审计 |
| apply 重复注入（多 mutation 多条目） | 接受重复写（幂等，μs 级）；不做跨语句批处理（YAGNI） |
| React 19 行为差异（ref 清理/StrictMode） | apply 幂等设计；浏览器实测覆盖 |

## 9. 验收清单

- [x] P1：ElementRefDemo.tsx 产物审计 4 断言全过（§4.5.3）
- [x] P1：verify-react GATE PASSED（无新错误类、无新豁免）
- [x] P1：react 套件全绿；浏览器实测断言 pointermove 零渲染提交（§4.5.5）
- [x] P2：@drag 独立 ADR 评审通过（ADR-073）；demo 源码无显式 capturePointer；D3 演进为注记
- [x] P3：Flutter 拖拽位移正确渲染（ListenableBuilder + Transform）；flutter gate 行为验证替代 D3 过滤
- [x] 覆盖审计（verify-demo-coverage.py）随语义成员变化复跑

## 10. 参考

- [ADR-071 ref 语义级语法的编译器责任模式](../.architecture/decisions/adrs/ADR-071-ref-semantic-compiler-responsibility.md) — 本设计的总纲（决策 1/2/3/4）
- [ADR-070 ref 高频属性操控——三目标机制契约](../.architecture/decisions/adrs/ADR-070-ref-high-frequency-property-manipulation.md) — 行为基准与缺口登记
- [docs/design/element-ref-system.md](element-ref-system.md) — v2 ref 系统（:388-393 注记待修订）
- [docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md](../superpowers/specs/2026-08-02-semantic-syntax-v2-design.md) — 域 1 Ref 系统（v2 基线）
- [docs/spec/10-interactions.md](../omniview/10-interactions.md) — spec §10.3 设计 D3（capturePointer 硬错误）
- codegen 锚点：`omnilang/src/codegen/react/{view.rs:718-731, component/mod.rs:151-229/791-867, component/state.rs:753-859/1162-1367, component/function.rs, design.rs:411-485}`；`omnilang/src/codegen/shared/style.rs:34`；`omnilang/src/codegen/flutter/view.rs:1597-1689`
