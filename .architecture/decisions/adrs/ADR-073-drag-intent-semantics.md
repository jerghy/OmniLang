# ADR-073: @drag 可拖拽意图语义（capturePointer 语义上移）

## Status

Implemented

> 语义链（parser/AST/IR/lowering/语义验证）已实现并测试通过；codegen 三目标（Task 8/9/10）在实施前保持语义定义不变。

## Context

ADR-071 决策 3 定义机制泄漏治理路线：定义高层「元素可拖拽」意图语义（@drag 或等效绑定），capturePointer 降级为支持目标（OmniView/React）的编译器内部实现细节。设计文档 `ref-high-frequency-direct-channel.md` §5（Phase 2：capturePointer 意图化）给出语义草案：

```
@drag={onDrag} @dragEnd={onDragEnd}           ← 元素可拖拽意图
  语义：指针在元素上按下 → 捕获拖动 → 拖动期间持续回调 onDrag
        指针移出元素仍跟随（隐式捕获语义）→ 抬起回调 onDragEnd
  事件数据：{ x, y, dx, dy, pointerId, startX, startY }
```

三目标实现方向（§5.2）：OmniView 用现有原语组合（capturePointer + pointermove 直写）、React 用 setPointerCapture + handler 直写、Flutter 用 GestureDetector.onPanStart/onPanUpdate/onPanEnd——全部为编译器内部选路，源码只写 @drag 意图。

本 ADR 将草案固化为正式语言语义，并落地语义链（AST → IR → lowering → 语义验证）。**编号说明**：设计文档 §5 预留本 ADR 为「独立 ADR」；实际编号为 **ADR-073**（ADR-072 被并行会话占用：debug-window-direct-load-compiled-output）。本决策内容与 §5 草案一致，不受编号影响。

## Decision Drivers

* **ADR-071 决策 3（机制泄漏治理）**：capturePointer 意图化——语义上移替代机制映射（Flutter 无指针捕获原语，近似等价论证困难）
* **语义级 DSL 哲学（ADR-053/054/056）**：源码表达意图，编译器选路实现
* **设计文档 §5 草案**：@drag/@dragEnd 语义、事件数据 schema、三目标实现方向
* **@ref 前置要求**：目标实现（setPointerCapture / 直写通道）需要元素句柄——意图语义必须绑定 @ref
* **「高频 = 简单直接」用户拍板（ADR-070）**：编译期无壳、意图语法最小

## Decision

**1. 语言新增 @drag / @dragEnd 元素指令**：语义为「指针在元素上按下 → 捕获拖动 → 拖动期间持续回调 @drag 处理器；抬起回调 @dragEnd 处理器」。事件数据 `{ x, y, dx, dy, pointerId, startX, startY }`——dx/dy 为本次事件相对按下点的累计位移，数值型（供各目标地道映射：Flutter pan 的 delta 语义由编译器换算）。两个属性**顺序无关**（同一元素上可任意排列）。

**2. @drag 要求同一元素绑定 @ref**（语义验证硬错误）：目标实现（React setPointerCapture、OmniView capturePointer 直写）需要元素句柄，且 Phase 1 直写通道以 `ref_binding` 为键——无 @ref 的 @drag 无法选路。诊断示例：「@drag requires @ref on this element (e.g. @ref={box})」。

**3. capturePointer/releasePointer 降级为编译器内部实现细节**：demo 源码不再显式书写；显式书写保留编译错误 + 迁移提示「use @drag instead」（防机制泄漏回潮）。

**4. spec §10.3 设计 D3 从硬错误演进为目标差异注记**：capturePointer 是支持目标的内部细节；Flutter 用户面向 @drag 语义（GestureDetector pan 是意图的 Flutter 地道实现，非近似等价）。

**语义绑定契约（Validation 清单）**：

| # | 检查 | 级别 | 触发 |
|---|------|------|------|
| V1 | 元素含 @drag/@dragEnd 时必须有 @ref | 硬错误 | 语义验证（`validate_drag` pass） |
| V2 | 处理器名必须解析为本组件 fn | 硬错误 | 语义验证（lowering 存名，pass 查存在性） |
| V3 | 处理器最多 1 参数（拖拽事件对象） | 硬错误 | 语义验证（0 参数合法，≥2 报错） |
| V4 | @dragEnd 单独出现（无 @drag） | 解析错误 | parser（prop 循环结束最终检查） |
| V5 | @drag/@dragEnd 重复 | 解析错误 | parser（push 合并时检测） |
| V6 | 值必须为裸组件函数名（禁内联箭头/调用） | 解析错误 | parser |

## Architectural Components Affected

* **AST**：`parser/ast/view.rs` `ElementProp::Drag { on_drag: String, on_drag_end: Option<String> }`
* **Parser**：`parser/view/element.rs` `parse_event_prop` 名称分派特例 + `push_element_prop` 合并；`parser/view/mod.rs` prop 循环收口最终检查
* **IR**：`ir/types.rs` `ElementIR.drag_binding: Option<DragBindingIR>`（独立字段，不入 `el.events` map——避免污染事件名验证与产生垃圾事件输出）；`ir/events.rs` `EventDataKind::DragIntent` + schema 7 字段
* **Lowering**：`ir/lower/view.rs` `drag_handler_ir`（FnRef + event_data=DragIntent + used_fields 扫描 + pd/sp 扫描）
* **语义验证**：`semantic/validate_drag.rs` 新 pass（run_ir_passes 注册，walk_view 模式）
* **codegen（Task 8/9/10）**：React/OmniView/Flutter 从 `drag_binding` 选路实现

**Interface Changes**:
* 语言面：新增 @drag / @dragEnd 元素指令（语义级意图，三目标可移植）
* capturePointer/releasePointer：保留编译错误，语义上移为内部实现细节

## Consequences

### Positive

* **机制泄漏消除**：capturePointer 从公开 API 降级为内部细节，Flutter 缺口从「机制映射」（论证困难）变为「意图语义 + 地道实现」（易）
* **高频拖拽语义三目标统一**：源码表达「可拖拽」意图，React setPointerCapture 直写 / OmniView 原语组合 / Flutter GestureDetector pan 由编译器选路——符合 ADR-071 编译器责任模式
* **事件数据数值化**：dx/dy 为相对按下点累计位移，避免「CSS 字符串模式解析」脆弱路径（§6.4 方案 A 被否决的根因）

### Negative

* **@ref 前置要求**：意图语义绑定元素句柄，无 @ref 用例（纯回调式拖拽）被硬错误拒绝——当前无此需求，YAGNI
* **codegen 三目标实现成本转移**：语义链完成后编译器责任加重（Task 8/9/10），每目标需独立验证

### Neutral

* 设计文档 §5 草案的 `{x,y,dx,dy,pointerId,startX,startY}` schema 原样落地（无演进）
* `drag_binding` 独立于 `events` map：事件名验证（validate_event_names）与事件输出路径零污染，codegen 读取点单一（`el.drag_binding`）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——语义链（AST/parser/IR/lowering/验证）为纯增量；`ElementIR` 加字段波及 ~40 个显式构造点（全部补 `drag_binding: None`，无一遗漏，编译期强制）。codegen 三目标为 Task 8/9/10 独立实施。

**Affected Components**:
- `crates/omnilang-core`：parser/ast/view.rs、parser/view/{element,mod}.rs、ir/{types,events,mod}.rs、ir/lower/view.rs、semantic/（validate_drag 新 pass + resolve_view/lints/构造点）
- `omnilang` codegen 测试构造点（fixtures_shared、react/vue/dioxus/flutter/omniview tests）
- codegen 实现（Task 8/9/10 另起）

**Risk Mitigation**:
- `drag_binding` 独立字段 + `#[serde(default, skip_serializing_if = "Option::is_none")]`——旧 WASM 产物与反序列化路径向后兼容
- 构造点全部编译期强制（无 `..Default` 偷懒模式），缺漏即编译错误
- 语义验证硬错误在 lowering 后立即拦截，不给 codegen 留死回调

### Reversibility

**Reversibility Level**: 高——语义链纯增量、可整体回退；`DragBindingIR` 存 `EventHandlerIR`（非裸 String）是为 codegen 事件发射路径（event_data + used_fields）预留，若实现中发现不需要可简化。

### Sequencing & Timing

1. 本任务（Task 7）：ADR + parser/AST/IR/lowering/语义验证 ✅
2. Task 8/9/10：React / OmniView / Flutter codegen 从 `drag_binding` 选路实现 + 门禁
3. demo 改造（ElementRefDemo 拖拽区 → @drag 意图）与 flutter gate D3 过滤收敛

## Implementation

**AST**：`ElementProp::Drag { on_drag: String, on_drag_end: Option<String> }`——@dragEnd 单独出现时 on_drag 为空串（占位符），由 `push_element_prop` 与顺序无关地合并成对；prop 循环结束后最终检查拒绝孤悬 @dragEnd。

**Parser**：`parse_event_prop` 在 `event_name == "drag" || event_name == "dragEnd"` 分派到 Drag 构造——值必须为裸组件函数名（`EventHandler::FnRef`），内联箭头/调用硬错误「@drag/@dragEnd 值必须是组件函数名」。`push_element_prop` 合并：首个 @dragEnd 入列占位符（@drag 可后到）、@drag 后到回填、@dragEnd 后到校验已存在 @drag 且未重复。

**IR**：`ElementIR.drag_binding: Option<DragBindingIR>`，`DragBindingIR { on_drag: EventHandlerIR, on_drag_end: Option<EventHandlerIR> }`。**偏离 brief 的 String 形状**：存完整 `EventHandlerIR`（FnRef + event_data=Some(DragIntent) + used_fields + pd/sp 标志）——codegen 事件对象发射路径（`emit_event_object_js/react`）按 event_data + used_fields 提取精确字段，String 形状会在 codegen 阶段丢失该信息。`EventDataKind::DragIntent` schema：x/y/dx/dy/startX/startY Float + pointerId Int。

**Lowering**：`drag_handler_ir(fns, name)` helper——fns 查找（找不到时语义 pass 报 V2）、`collect_ir_event_fields` 扫描 fn body 的 `e.field`、`scan_ir_event_methods` 扫描 preventDefault/stopPropagation。

**语义验证**：`validate_drag` pass（run_ir_passes 第 8 位）——walk_view 每元素回调，V1 @ref 检查、V2 handler 存在性、V3 arity ≤ 1。handler 名解析为 `{ x, y, dx, dy, pointerId, startX, startY }` 事件对象（报错信息内联 schema）。

## Alternatives Considered

### handler 存裸 String（brief 原案）

**Pros:**
* IR 更小、语义验证直接读名

**Cons:**
* codegen 事件对象发射需要 event_data + used_fields（emit_event_object_js 签名），String 形状丢失该信息，codegen 阶段需二次扫描或重新接线——事件发射路径的既有语义绑定基建（D1 apply 对齐）无法复用

### @drag 处理器进 el.events map（复用 @onX 事件管线）

**Pros:**
* 零新字段

**Cons:**
* events map 是 DOM 事件（validate_event_names 白名单验证 + codegen 逐事件发射），DragIntent 非 DOM 事件——会污染事件名验证并产生垃圾事件输出；且 @drag/@dragEnd 是成对意图，拆成两个独立事件丢失配对关系

### 语义验证阶段对 capturePointer 显式书写软提示（不硬错误）

**Pros:**
* 迁移平滑

**Cons:**
* 机制泄漏回潮（ADR-071 决策 3 明确保留硬错误 + 迁移提示）；软提示在 demo 改造前无人移除

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 将设计文档 §5 草案固化为语言语义，语义链实现为纯增量（~700 行含测试）。无新依赖、无新抽象；`DragBindingIR` 存 EventHandlerIR 是复用既有事件发射基建而非新增机制。

**Decision Challenge**:

**Proposed Decision**: "@drag/@dragEnd 可拖拽意图语义：4 条决策（意图语义/@ref 前置/capturePointer 降级/D3 演进）+ 6 项 Validation 清单"

**Necessity Assessment**: 8/10
- **Current need**: ADR-071 决策 3 的既定下一步（「需先完成意图语义设计」）；codegen Task 8/9/10 依赖语义定义
- **Future need**: Flutter 缺口收敛、demo 改造、spec §10.3 D3 演进全部以本语义为前置
- **Cost of waiting**: 设计文档 §5 是草案非决策——无 ADR 则 codegen 无实现依据
- **Evidence of need**: 设计文档 §5 草案 + ADR-071 决策 3 明确要求

**Complexity Assessment**: 3/10
- **Added complexity**: 语义链纯增量；ElementIR 加字段是既有模式（a11y/semantic_bindings 同型）
- **Maintenance burden**: 低——validate_drag 单 pass、构造点编译期强制
- **Learning curve**: 低——事件处理/语义验证既有基建直接复用
- **Dependencies introduced**: 零

**Alternative Analysis**: 已考虑三项（String 形状/events map 复用/软提示）——String 形状丢失 codegen 所需信息、events map 污染事件验证、软提示允许回潮。三者均劣于所采纳案。

**Simpler Alternative Proposal**: 无——brief 草案与设计文档 §5 已是最小语义；偏离仅一处（String → EventHandlerIR），是 codegen 发射路径的复用性修正而非范围扩张。

**Recommendation**: ✅ Approve decision

**Justification**: 设计文档 §5 既定草案 + ADR-071 决策 3 的正式化；实现纯增量、编译期强制完整性；codegen 三目标分 Task 独立实施各带门禁。

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 3/10
- **Ratio**: 0.375 *(Target: <1.5)*

**Overall Assessment**: 适当工程——既定草案正式化 + 最小语义链落地，无过度设计。

## Validation

* `cargo test -p omnilang-core`：1029 passed（含 21 个 drag 相关测试：parser 6 + lowering 1 + semantic 5 + 既有 motion/lexer drag 9）
* `cargo check -p omnilang`：通过（codegen 测试构造点全部补齐）
* parser 测试覆盖：`parses_drag_directive`（顺序无关对）、`drag_end_before_drag_is_order_independent`、`drag_end_without_drag_errors`、`duplicate_drag_errors`、`drag_value_must_be_bare_function_name`、`drag_does_not_collide_with_motion_drag_triggers`（@drag 与 @drag-horizontal 专用 token 不冲突）
* lowering 测试：`test_lower_element_drag_directive`（drag_binding 生成 + event_data=DragIntent）
* 语义验证测试：`drag_with_ref_and_existing_handlers_passes`、`drag_without_ref_errors`（V1）、`drag_handler_undefined_errors`（V2）、`drag_handler_with_too_many_params_errors`（V3）、`drag_inside_list_body_is_checked`（walk_view 深入 @for body）

## References

* [ADR-071-ref-semantic-compiler-responsibility.md](./ADR-071-ref-semantic-compiler-responsibility.md) — 决策 3（capturePointer 意图化）为触发上下文
* `docs/design/ref-high-frequency-direct-channel.md` §5 — @drag 语义草案与三目标实现方向
* `docs/design/ref-high-frequency-direct-channel.md` §6.4 — 方案 A（CSS 字符串模式解析）否决记录
* [ADR-070-ref-high-frequency-property-manipulation.md](./ADR-070-ref-high-frequency-property-manipulation.md) — 三目标机制契约与 D3 起源
* Task 7 brief：`.superpowers/sdd/2026-08-07-ref-high-frequency-direct-channel/task-7-brief.md`
