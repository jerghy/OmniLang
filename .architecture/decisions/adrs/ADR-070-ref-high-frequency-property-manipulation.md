# ADR-070: ref 高频属性操控——三目标机制契约 + measure 可空性兜底

## Status

Implemented

## Context

用户评审 learn-omniview ElementRefDemo 时拍板：**高频拖拽操作此前用 @motion scrubbing 套壳（「又套了一层壳」）——高频操作就是高性能，高性能就是要简单直接**，且「不只是拖拽，有很多是直接改组件属性的，它们的高频操作你也要设计」。同时质疑：「你确定这个 ref 语法，能够实现高频高性能地操控组件属性吗？」

改造后（2026-08-07）需要把三个问题的答案固化为架构契约：(1) 高频属性操控的正确原语模式是什么；(2) 同一 .omni 源码在三目标（OmniView/React/Flutter）上的实际编译路径与性能语义差异是什么（此前无文档，靠实测产物才知道）；(3) ref-op 返回值可空性（measure 等）在跨目标代码生成中的处理规则（此前依赖运行时兜底，React tsc 门禁暴露 7 个 TS18048/TS2345 错误类）。

## Decision Drivers

* **高频 = 简单直接（用户拍板）**：逐帧操作（拖拽/选区/进度）必须用底层原语（@ref + 信号 + style:{expr} 绑定 + 原生事件），禁止动画库套壳
* **三目标性能语义诚实标注**：展示项目是「可执行文档」——目标差异必须写进设计文档（docs/design/element-ref-system.md），不能假装全平台同等性能
* **门禁可验证**：React tsc 门禁（verify-react）对 ref-op 生成代码的严格性要求——possibly-undefined 与优先级错误必须在 codegen 侧根治，而非进豁免表
* **Flutter 能力底线如实呈现**：capturePointer 是 spec §10.3 设计 D3 硬错误（「Flutter 不支持指针捕获，用 GestureDetector」）；动态 style 值静默跳过是已知缺口——展示项目要展示真实行为（含目标警告），不回避
* **ref 操作要展示灵活运用**：字段读取形式（ref.position/.width/.height）、单参 scrollTo、嵌套组合（scrollTo(0, ref.measure().y)）、无参 capturePointer——「可执行文档」要可看可学

## Decision

**高频属性操控确立底层原语模式，并固化三目标机制契约与 ref-op 兜底规则。**

1. **高频操控原语模式（用户拍板）**：`@ref` + 信号（`onDragMove` 中 `dragX = e.x - origin`）+ `style: { transform: ... }` 表达式绑定 + `@onPointerDown/Move/End` 原生事件——编译期无壳（OmniView 产物为 `el.style.transform = ...` 直写 + 原生 addEventListener + 信号 `.set()`）。禁止用 @motion scrubbing 包裹逐帧操控（motion 保留给声明式动画：触发器/keyframes/过渡）。
2. **三目标机制契约（实测产物验证，写入 element-ref-system.md 状态表）**：
   * **OmniView**：`el.style.transform = __omni_sanitizeCss(...)` 直接 DOM 写 + 原生事件 + 信号直写——**零 reconciliation**，真·高频高性能（build/index.js:13803 实测）
   * **React**：`setDragX(...)` → useState → 整组件重渲染 → `style={{transform}}`——**v2 主动接受的代价**（element-ref-system.md:388-393 注记；ref.style 直接写是编译错误）
   * **Flutter**：动态 style 绑定值**静默跳过**（view.rs inline_style 只处理静态字符串，`Some(_) => skip`）——拖拽位移不渲染，已知缺口（未修，记录在案）；高频操控在 Flutter 只能走声明式 signal → widget 重建
3. **capturePointer 三目标行为**：OmniView/React 支持（原生 API 直译）；Flutter **编译期硬错误**（设计 D3）——omnic build -t flutter 对含 capturePointer 的项目 EXIT=1 是预期行为，flutter gate（generated_code_validation.rs）显式过滤该诊断，其余错误才算回归。
4. **ref-op 兜底契约（codegen 侧根治）**：
   * **measure 结果可空兜底**：`ref.measure()` 返回零矩形时结果为 null——生成代码统一发射 `?? {left:0, top:0, width:0, height:0}` 兜底（零矩形/0/'' 三态）
   * **字段访问括号化**：成员访问优先级高于 `??`——`(A ?? {zero}).y` 必须整体括号化，否则 `.y` 绑定到兜底字面量恒 undefined（React TS18048 根因）
   * **len/is_empty 多态 dispatch**：Map 字面量 = 普通对象，`({}||[]).length` 恒 undefined——omniview 目标 len/is_empty 按 Array/string/object 运行时分派
5. **展示扩充（可执行文档）**：ElementRefDemo 加入 .select/.setSelection+@onSelect 回显/.value 读写联动/.position/.width/.height 字段读取/嵌套 scrollTo(0, ref.measure().y) 组合——ref 全部 10 成员展示齐（focus/blur/scrollTo/scrollBy/measure/select/setSelection/value/capturePointer/releasePointer），覆盖审计缺口 0。

**Architectural Components Affected:**
* 展示项目：ElementRefDemo.omni（拖拽改底层原语 + 灵活操作全集）、MapOperationsDemo/StructImportDemo（新增）
* codegen：ref-op 发射（?? 兜底 + 括号化）、shared/expr.rs len/is_empty dispatch
* parser：struct 字面量后缀在赋值 RHS 与三元 else 分支丢失（`s = S { f: v }` 误解析为块调用）→ maybe_struct_literal_suffix 统一四处尾部
* 门禁：verify-react.exemptions 删 2 stale（TS18048 #63/TS2345 #64）+ TS7006 cap 16→18（#67 事件参数类型框架注入）

**Interface Changes:**
* 语言面：无新语法（全部为既有语法的正确用法确立）
* 文档面：docs/design/element-ref-system.md 状态表 + v2 高频路径注记（三目标机制契约）

## Consequences

### Positive

* **高频路径可证伪**：三目标机制不再是宣称而是产物实测（build/index.js 直写证据、ElementRefDemo.tsx useState 证据、view.rs 跳过证据）——未来重构有回归基准
* **React 门禁净减 7 错误**：tsc 77→72——TS18048（4）+ TS2345（3）错误类随兜底与括号化根治消失，非豁免而是修复
* **拖拽教学价值**：展示项目呈现「简单直接」范式——@ref 直写 style 绑定 vs 旧 @motion 套壳的对照即是教学本身
* **灵活操作有实体**：字段读取/嵌套组合/单参调用全部可运行可演示（浏览器实测：拖拽 translate(150,75) 精确跟随、嵌套 scrollTo clamp 183.33、setSelection(2,7) 回显）
* **覆盖缺口 0**：ref 域 10 成员全部展示（verify-demo-coverage.py 可复跑）

### Negative

* **Flutter 高频能力是缺口**：动态 style 绑定静默跳过——拖拽位移在 Flutter 不渲染（记录在案未修；修需要 view.rs inline_style 支持表达式值，跨三目标回归门槛）
* **React 目标重渲染是接受的代价**：高频更新整组件 reconciliation——文档注记 v2 决策，未来若需优化需 ref DOM 直写通道（当前是编译错误，方向性工作量大）
* **TS7006 cap 16→18**：事件参数类型为框架注入（EventDataKind→PointerEvent 为 JS 端标签），显式标注会转 TS2322——deferral #67 持续存在

### Neutral

* ElementRefDemo 展示面积扩大（拖拽/选区/字段/组合四区）
* verify-react 豁免表更新（2 删 1 调 + 基线注释同步）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——展示项目 2 文件 + codegen 3 处 + parser 1 处 + 门禁 1 文件。codegen/parser 修复影响所有目标的 ref-op 输出（方向修复：兜底与括号化使 possibly-undefined 不再出现，React 门禁实测通过）。

**Affected Components**:
- ElementRefDemo.omni —— 重写拖拽 + 扩充灵活操作（4 commits：148da088/021b2441/6fe99766/b2339203）
- ref-op codegen —— ?? 兜底三态 + 字段访问括号化
- shared/expr.rs —— len/is_empty 多态 dispatch（Map 守卫）
- parser —— maybe_struct_literal_suffix 四处尾部统一
- verify-react.exemptions —— 2 删（stale）+ 1 调（cap）

**Risk Mitigation**:
- 三目标验证：react 196/196 + compilation_test 7/7 + verify-react GATE PASSED
- 浏览器全交互实测（拖拽/scrollTo 嵌套/select/字段回显/Mirror 联动）
- 覆盖审计缺口 0（可复跑）
- Flutter EXIT=1 已确认是设计 D3 预期（gate 显式过滤），非回归

### Reversibility

**Reversibility Level**: 高——展示内容可回退；codegen 兜底为纯缺陷修复（不可逆也不应逆）；parser 修复有 compilation_test 7/7 守护。

### Sequencing & Timing

已完成（2026-08-07）。前置：element-ref-system v2 设计（ref 系统基线）；ADR-069 覆盖审计基建。

## Implementation

**Phase 1：展示改造（用户拍板原语）**
* ElementRefDemo 拖拽从 @motion scrubbing → @ref + @onPointerMove + 信号 + style:{transform} 绑定
* 扩充灵活操作全集（字段读取/嵌套 scrollTo/select/setSelection/value 联动）

**Phase 2：codegen 根治（React 门禁暴露）**
* measure 结果 `?? 零矩形兜底`（零矩形/0/'' 三态）+ 字段访问括号化（成员访问优先级 > ??）
* omniview len/is_empty 多态 dispatch（Map 字面量 = 普通对象）
* parser struct 字面量后缀四处尾部统一

**Phase 3：门禁与文档同步**
* verify-react.exemptions 2 删 1 调 + 基线注释（77→72）
* element-ref-system.md 状态表 + 三目标机制契约注记

## Alternatives Considered

### 保留 @motion scrubbing 拖拽（被用户否决）

**Pros:**
* 动画库自带缓动/惯性，代码量少

**Cons:**
* 套壳违背「高频 = 简单直接」；@motion 是声明式动画工具，逐帧操控是其薄弱路径
* 用户明确拍板否决

### ref.style 直接写（DOM 直写通道）

**Pros:**
* React 目标也能绕开 reconciliation（类似 OmniView 直写）

**Cons:**
* React 受控模型下直写与状态不一致（当前是编译错误，element-ref-system v2 决策）；方向性工作量大、跨目标语义分裂风险高

### measure 兜底放运行时（不修 codegen）

**Pros:**
* 改动面最小

**Cons:**
* React tsc 门禁 TS18048 类持续存在（possibly-undefined 传播到整个表达式链）；门禁原则是修复而非豁免堆积

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 决策本身是**简化**而非增复杂度——用户拍板「去壳」（@motion → 原语），codegen 兜底是为正确性（非新抽象），文档契约是既有事实的固化。三目标机制契约不是新设计，是实测结果记录（避免未来重测/误判）。

**Decision Challenge**:

**Proposed Decision**: "高频属性操控确立 @ref + 信号 + style 绑定的底层原语模式，并固化三目标机制契约与 ref-op 兜底规则"

**Necessity Assessment**: 8/10
- **Current need**: 用户明确要求（「高频操作就是高性能，高性能就是要简单直接」「你确定这个 ref 语法能实现吗」）——当下、具体、有拍板
- **Future need**: 契约文档防未来重构误判（三目标差异是重构雷区）
- **Cost of waiting**: 不修则 React 门禁 TS18048 类持续 + 拖拽继续套壳 + 性能承诺无证据
- **Evidence of need**: 用户 4 轮追问 + 门禁实测 7 错误 + 产物三处实测证据

**Complexity Assessment**: 3/10
- **Added complexity**: 无新抽象；codegen 兜底 + 括号化为正确性修复（~3 处发射逻辑）
- **Maintenance burden**: 低——契约写入 element-ref-system.md 即文档化
- **Learning curve**: 低——原语模式即现有语法正确用法
- **Dependencies introduced**: 零新依赖

**Alternative Analysis**: 已考虑三项（保留套壳/ref.style 直写/运行时兜底）——「不做」选项（维持现状）被用户直接否决，不成立。

**Simpler Alternative Proposal**: 无——本决策已是当前需求的最简形态。Flutter 动态 style 跳过缺口按 YAGNI 原则**不修**（记录在案，触发条件：Flutter 高频操控成为实际需求）。

**Recommendation**: ✅ Approve decision

**Justification**: 用户拍板 + 门禁实证 + 零新依赖 + 契约文档防未来误判；唯一开放项（Flutter 动态 style）明确 deferral 且有触发条件——不修不是忽略而是有记录的等待。

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 3/10
- **Ratio**: 0.375 *(Target: <1.5)*

**Overall Assessment**: 适当工程——本次工作是「减法」（去套壳）与「正确性修复」（兜底/括号化），三目标契约是对既有事实的诚实记录。无过度设计。

## Validation

**Acceptance Criteria:**
- [x] 拖拽用底层原语（@ref + 信号 + style 绑定），无 @motion 套壳（ElementRefDemo.omni 实测）
- [x] React 门禁 GATE PASSED：tsc 72 errors 全豁免（TS18048/TS2345 错误类根治消失，非豁免）
- [x] react 196/196 + compilation_test 7/7 全绿
- [x] 浏览器实测：拖拽 translate(150,75) 精确跟随、嵌套 scrollTo clamp 183.33、setSelection(2,7)、字段读取回显、Mirror 联动
- [x] 覆盖审计缺口 0（ref 域 10 成员全展示）
- [x] Flutter build EXIT=1 确认为设计 D3 预期（capturePointer 硬错误），非回归

**Testing Approach:**
* 生成代码产物审计（build/index.js 直写证据 / ElementRefDemo.tsx useState 证据 / view.rs 跳过证据）
* verify-demo-coverage.py 覆盖审计（可复跑）
* verify-react.sh 门禁（豁免表 2 删 1 调）
* flutter gate（generated_code_validation.rs，显式过滤 D3 诊断）

## References

* [docs/design/element-ref-system.md](docs/design/element-ref-system.md) — v2 状态表 + 三目标机制契约（:388-393 注记）
* [spec §10.3 设计 D3](docs/) — Flutter capturePointer 硬错误
* [ADR-069 learn-omniview 展示项目完整化](ADR-069-learn-omniview-demo-coverage-standard.md) — 覆盖审计基建
* [memory: session-2026-08-07-ref-flexible-ops](memory://session-2026-08-07-ref-flexible-ops) — 会话实况与证据（Claude 记忆库）
* [memory: ir-type-ann-attachment-defect](memory://ir-type-ann-attachment-defect) — len typed 分派死代码根因（runtime dispatch 兜底语义等价）
