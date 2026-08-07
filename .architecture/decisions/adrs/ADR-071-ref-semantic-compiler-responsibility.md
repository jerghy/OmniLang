# ADR-071: ref 语义级语法的编译器责任模式 + 机制泄漏治理（capturePointer 意图化）

## Status

Accepted

## Context

用户追问 ref 语法的高频性能可行性时提出关键判断：**「ref 的语法应该是语义级语法吧？如果是的话，只需要编译器了解它到底想要实现什么前端功能，然后由编译器去实现就行，可行性应该还是比较大的吧」**。

这一判断需要固化为架构原则，因为它的推论直接决定未来实现路线：

1. `.omni` 的 `ref: box` + `style: { transform: expr }` + `@onPointerMove` 表达的是**前端功能意图**（「box 的 transform 恒等于该表达式；指针移动时更新 dragX」），而非任何目标的机制
2. 编译器责任模式（language expresses intent, compiler chooses implementation）在 `@for` 上已有先例：三目标实现完全不同（React map / Flutter ListView.builder / OmniView DOM 循环），源码一致
3. 若该模式适用于 ref，则 React 高频直写、Flutter 动态绑定**都不需要新语法**——只需各目标编译器把「style 动态绑定」语义翻译成各自地道的高频机制
4. 但该模式成立的前提是**语义在三目标都可定义**——ADR-070 实测暴露的反例：`capturePointer` 在 Flutter 渲染树没有「指针捕获」原语（move 事件靠命中链隐式路由），设计 D3 硬错误是语义对齐失败的诚实暴露，而非编译器偷懒

## Decision Drivers

* **语义级 DSL 哲学（ADR-053/054/056）**：语言层表达意图，实现层归编译器——ref 必须继承这一契约
* **「高频 = 简单直接」用户拍板（ADR-070）**：高频操控用底层原语，编译期无壳
* **编译器责任模式先例**：@for 三目标异构实现是模式成立的实证
* **机制泄漏治理**：语义 API 必须是三目标可定义的意图——capturePointer 是当前唯一机制泄漏成员
* **为 React 直写通道与 Flutter 路径提供设计基础**：避免「给 capturePointer 找 Flutter 等价」的错误方向（语义不等价论证困难），改走「语义上移」

## Decision

**1. 确认编译器责任模式为 ref 语义级语法的实现原则**：`.omni` 源码表达前端功能意图（可拖拽、可聚焦、属性恒等于表达式），三目标编译器从语义自行选路实现——可行性由语义完备性决定，而非目标机制。

**2. style 动态绑定语义定义**：「元素的该属性**恒等于**该表达式」。三目标实现策略（此为本决策给编译器的实现指引）：

| 目标 | 实现策略 | 性能语义 |
|---|---|---|
| OmniView | `el.style.transform = ...` 直接 DOM 写（现状） | 零 reconciliation |
| React | `useRef` + 事件回调/effect 直写 `el.style`——属性不进 style prop，避开重渲染覆盖 | 零 reconciliation（与 OmniView 同级） |
| Flutter | `ListenableBuilder` 包裹 + `Transform`——信号即 ValueNotifier，局部重建不整树 build | 零整树 diff |

**3. 机制泄漏治理——capturePointer 意图化**：定义高层「元素可拖拽/可捕获指针」意图语义（如 @drag 或高层绑定），capturePointer 降级为支持目标（OmniView/React）的编译器内部实现细节，demo 源码不显式写它。Flutter 缺口因此从「机制映射」（论证近似等价，难）变为「意图语义 + 各目标实现」（易）——设计 D3 从硬错误演进为目标差异注记（需先完成意图语义设计）。

**4. 实现路线（后续按此推进）**：
* React 直写通道：修订 v2「ref.style 写是编译错误」决策（高频用例是 React 官方 escape hatch 正当用例），codegen 将 ref 元素上引用信号的 style 动态属性发射为直写通道
* Flutter inline_style 表达式化：从「静态字符串直译」扩展到「表达式 → ListenableBuilder 包裹 + ValueNotifier 接线 + 多动态属性 Listenable.merge」
* 门禁同步：verify-react 验证直写通道无新错误类；flutter gate 从「过滤 D3 诊断」改为「验证新行为」
* IR type_ann 修复为编译器选路增强（typed 分派激活），非前置依赖

**Architectural Components Affected:**
* codegen：react/expr.rs（style 动态属性发射策略）、flutter/view.rs（inline_style 表达式化）、shared/expr.rs（语义判定）
* 语义层：capturePointer 意图化设计（新语义成员或绑定形式）
* 决策面：v2「ref.style 写是编译错误」修订、spec §10.3 设计 D3 演进
* 门禁：verify-react、generated_code_validation.rs

**Interface Changes:**
* 语言面：**无**——全部为编译器实现策略与语义定义，.omni 源码不变（意图化若引入新语义成员则另行 ADR）

## Consequences

### Positive

* **三目标高频性能统一由语义推导**：React 直写后与 OmniView 同为零 reconciliation；Flutter 局部重建零整树 diff——「高频 = 高性能」承诺在语义层成立，不再依赖目标巧合
* **无需新语法**：哲学确认直接兑现为编译器工作，符合「少而精」语义 DSL 原则
* **Flutter 缺口正解明确**：语义上移（意图化）替代机制映射（近似等价论证），路径可行且简化
* **跨目标契约可复用**：未来目标（vue/dioxus）实现同一语义契约有明确基准（ADR-070 三目标机制契约 + 本决策实现指引）

### Negative

* **语义定义先行的工作**：capturePointer 意图化需要新的语义设计（@drag 或等效绑定）——设计决策在实现之前
* **v2 决策修订有跨目标回归风险**：「ref.style 写是编译错误」→ 直写通道，涉及 React 输出全体变化，需门禁与浏览器双验证
* **IR type_ann 恒 None 缺陷限制选路**：编译器想用类型信息选路但拿不到（collect/lower scope 断裂）——修复后编译器能力增强，属内部债非哲学障碍

### Neutral

* ADR-070 三目标机制契约成为实现基准（行为不变量）
* Flutter 门禁语义从「预期错误过滤」变为「行为验证」

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——决策本身零代码改动（Accepted，待实现）；实施后影响 React/Flutter codegen 输出全体。语义层意图化是纯增量。

**Affected Components**:
- react codegen（直写通道发射）——React 生成代码全体变化风险，verify-react 门禁守护
- flutter codegen（inline_style 表达式化）——widget 树生成策略变化，flutter gate 重写
- 语义层新成员设计——纯增量

**Risk Mitigation**:
- React 先行（成本低、门禁现成、社区先例强），Flutter 待意图语义设计完成后动
- 每目标独立 commit + 门禁验证，可回退
- IR type_ann 修复保持独立（跨 target 回归门槛，见 [[ir-type-ann-attachment-defect]]）

### Reversibility

**Reversibility Level**: 高——实施阶段每目标独立可回退；决策本身（Accepted）不锁死实现，语义定义可再演进。

### Sequencing & Timing

分阶段（未排期）：
1. React 直写通道（修订 v2 决策 + codegen + 门禁）——优先
2. capturePointer 意图化语义设计（新 ADR 或本 ADR 修订）
3. Flutter inline_style 表达式化（依赖 2 的语义定义）

## Implementation

**Phase 1：React 直写通道**
* 修订 v2「ref.style 写是编译错误」决策（ADR 记录修订理由：高频 escape hatch 是 React 官方正当用例）
* codegen：ref 元素上引用信号的 style 动态属性 → useRef + 直写（属性不进 style prop）
* 验证：verify-react 无新错误类 + 浏览器实测拖拽 60fps + react 测试套件

**Phase 2：capturePointer 意图化语义设计**
* 设计「可拖拽」意图语义（@drag 或等效绑定）——独立 ADR
* capturePointer 降级为编译器内部实现细节；spec §10.3 D3 演进为目标差异注记

**Phase 3：Flutter inline_style 表达式化**
* 动态 style 值 → ListenableBuilder 包裹 + ValueNotifier 接线
* flutter gate 从过滤 D3 诊断改为验证新行为

## Alternatives Considered

### 维持现状（React useState 重渲染 / Flutter 静默跳过）

**Pros:**
* 零改动

**Cons:**
* 「高频 = 高性能」承诺在 React/Flutter 不成立（ADR-070 实测）；Flutter 拖拽位移根本不渲染——展示项目「可执行文档」失信

### 给 capturePointer 找 Flutter 等价（机制映射）

**Pros:**
* 不动语义层

**Cons:**
* Flutter 无指针捕获原语（命中链隐式路由），需论证「近似等价」——语义不等价论证困难，且产生行为漂移风险；设计 D3 当初选硬错误的理由依然成立

### 每目标新增语法（React 专属 / Flutter 专属标记）

**Pros:**
* 目标最优化

**Cons:**
* 违反语义级 DSL 哲学（ADR-054）——语言层泄漏目标差异，.omni 源码失去跨目标可移植性；三份语法三份维护

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 本决策是**哲学确认 + 实现指引**，零代码改动（Accepted 待实施）——确认既有设计原则（语义级 DSL）适用于 ref 域，并治理一个已暴露的设计缺陷（机制泄漏）。无新抽象、无新依赖。

**Decision Challenge**:

**Proposed Decision**: "确认 ref 语义级语法的编译器责任模式：源码表达意图、编译器自行选路；capturePointer 意图化治理机制泄漏"

**Necessity Assessment**: 9/10
- **Current need**: 用户哲学确认（「只需要编译器了解它想要实现什么前端功能」）需要正式化——否则未来实现方向分歧（机制映射 vs 语义上移）无裁决依据
- **Future need**: React/Flutter 高频路径的全部后续工作以此为前提
- **Cost of waiting**: 不确认则 Flutter 路径可能走上机制映射错误方向（论证困难、行为漂移）
- **Evidence of need**: 用户明确拍板 + ADR-070 三目标实测证据 + @for 编译器责任先例

**Complexity Assessment**: 2/10
- **Added complexity**: 零实现；语义定义（style 动态绑定「恒等于」语义）是澄清而非新增
- **Maintenance burden**: 低——意图化减少机制 API 泄漏面，语义层反而更简
- **Learning curve**: 低——继承 ADR-053/054 既有哲学
- **Dependencies introduced**: 零

**Alternative Analysis**: 已考虑三项（维持现状/机制映射/每目标语法）——「不做」失信于「可执行文档」承诺；机制映射论证困难且行为漂移；每目标语法违背既有 DSL 哲学。语义上移是三者中最简且符合既有原则的。

**Simpler Alternative Proposal**: 无更简路径——「确认既有原则适用」本身即最小决策；实施阶段（React 直写/Flutter 表达式化）保持分阶段、每阶段门禁验证，不做一次性大改。

**Recommendation**: ✅ Approve decision

**Justification**: 用户拍板 + 既有哲学（ADR-053/054）的领域延展 + 零实现复杂度 + 治理已暴露缺陷。实施被明确分阶段并各带门禁，无过度设计风险。

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 2/10
- **Ratio**: 0.22 *(Target: <1.5)*

**Overall Assessment**: 适当工程——确认原则 + 治理缺陷 + 给出实现指引，实施分阶段可回退。无过度设计。

## Validation

**Acceptance Criteria:**
- [x] （Phase 1）React 拖拽生成代码无 useState 重渲染路径（useRef + 直写），verify-react 无新错误类
- [x] （Phase 1）浏览器实测拖拽与 OmniView 同级流畅（零 reconciliation）
- [ ] （Phase 2）意图语义设计 ADR 评审通过；demo 源码不再显式写 capturePointer
- [ ] （Phase 3）Flutter 拖拽位移正确渲染（ListenableBuilder + Transform）；flutter gate 行为验证替代 D3 过滤

**Testing Approach:**
* 生成代码产物审计（三目标各阶段）
* verify-react.sh + flutter gate（generated_code_validation.rs）门禁
* 浏览器实测（拖拽跟随/位移渲染）
* 覆盖审计（verify-demo-coverage.py）随语义成员变化复跑

## References

* [ADR-070 ref 高频属性操控——三目标机制契约](ADR-070-ref-high-frequency-property-manipulation.md) — 三目标机制实测基准（本决策的实现指引前提）
* [ADR-054 语义级 DSL 时代设计哲学](ADR-054-omniview-semantic-dsl-era-design-philosophy.md) — 5 原则：语言表达意图、编译器负责实现
* [ADR-053 语义级 DSL 语法形式化](ADR-053-semantic-level-dsl-syntax-formalization.md) — 语义语法设计基线
* [docs/design/element-ref-system.md](docs/design/element-ref-system.md) — v2 ref 系统设计（含「ref.style 写是编译错误」决策，Phase 1 修订目标）
* [spec §10.3 设计 D3](docs/) — capturePointer 硬错误（Phase 2 演进目标）
* [memory: session-2026-08-07-ref-flexible-ops](memory://session-2026-08-07-ref-flexible-ops) — 会话实况与三目标实测证据
* [memory: ir-type-ann-attachment-defect](memory://ir-type-ann-attachment-defect) — 编译器选路能力内部债（typed 分派死代码）
