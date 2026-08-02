# ADR-055: OmniView @for 渲染与事件委派运行时契约

## Status

Accepted

## Context

ADR-052/053 定义了语义级 DSL，ADR-054 确立了「运行时正确性由真实执行验证」原则。2026-08-01 首次浏览器全量测试（commit 09c18415）实现了 @for 事件委派初版（data-event-*/data-emit-* 属性 + 委托监听器 + `__omni_forItemVar` 注入）。2026-08-02 第二轮浏览器全量测试（learn-omniview 9 页 + `document.getAnimations()` 动画验证）发现初版机制存在**连锁性运行时缺陷**——keyed 重渲染丢项、emit 参数错误、委派闭包过期等，全部为静默错误（无编译期诊断、无运行时报错、界面数据悄悄错误）。

共 4 个 commit（a448b623 → 7330e783 → 242a5e53 → d0ed59e5）修复 15 个编译器 bug + 34 处 demo 问题。本 ADR 记录其中沉淀的**运行时契约**（非纯 bug 修复的架构性决策），防止后续评审/修改重新引入同类缺陷。

关键失败模式（浏览器复现）：

1. **keyed @for 重渲染丢全部已有项**：clear loop 无条件移除 tpl 与 ctr 之间的所有克隆，但 keyed 路径 append 仅限 isNew → 每次重渲染（如添加第二个 todo）已有项全部消失；`TodoStore.remove({})` 场景下计数与列表显示矛盾。
2. **emit 事件参数错误**：委派监听器对 `data-emit-*`（内联子组件 CustomEvent）也调用 `fn(_extractEvent(...))` 返回的 `{}` → `TodoList_handleDelete({})` → `filter(item.id !== {})` 永不命中——删除静默失效。
3. **委派监听器闭包过期**：空状态早退使监听器只在**首次非空渲染**挂载，闭包 `data` 永远是该快照 → 第二次删除用错 item id、rebind 回写错误文本。
4. **nid 后缀累加**：keyed 复用克隆每次渲染 `assignUniqueNids` 追加 `_idx`（`el-x_1_0_0`）→ 精确 `[data-nid="base_idx"]` 绑定查找失败 → 复用克隆文本不更新。
5. **空列表清理缺失**：无 `@empty` 的非 keyed 列表在数据清空时早退不清理 DOM（NotificationsPanel Clear 后旧项残留）。
6. **FnRef handler 的 `e.stopPropagation()` 失效**：只有 InlineFn 走 `scan_event_methods`，FnRef 既不发射 listener 级标志、handler 又收到 `{}` → `{}.stopPropagation()` TypeError（Modal 内容点击误关的根因）。
7. **函数局部名被 store 信号重命名遮蔽**：参数 `items` 解析为 `TodoStore_items`（validate_all 迭代错对象）；局部 `let mut` 数组 `.push` 恒走 `__omni_subs.get("local")` 信号路径（一旦被调用即 TypeError）。
8. **后代选择器截断**：`.note-list li` 被 parser 静默截断为 `.note-list`，规则降级作用于祖先（lexer 丢弃空白导致无法区分复合 `.a.b` 与后代 `.a .b`）。

## Decision Drivers

* **静默错误是头号敌人（ADR-054 原则 3 延续）**：上述 8 类缺陷全部无诊断无声失效——契约必须让「看起来对」与「实际对」一致
* **keyed 身份追踪语义**：`key={expr}` 的承诺是克隆复用（状态保留、FLIP 动画、enter 不重放）——重渲染必须保持该承诺
* **委派监听器一次挂载 vs 数据永远新鲜**：监听器挂在容器上只 attach 一次（性能），但 item 解析必须用最新渲染数据
* **语言语义决定编译形态**：`e.stopPropagation()` 是编译期指令（→ listener 级标志），不是运行时对象方法调用（handler 只收到类型化事件数据对象）
* **局部遮蔽是语言基本语义**：函数参数/let 声明必须遮蔽同名 store 信号引用

## Decision

### D1. keyed @for 克隆复用契约（list.rs）

- 顶层 clear loop 仅对**非 keyed** 列表发射；keyed 列表的存活克隆保留在 DOM（身份由 `ctr._forKeys` Map 追踪），append 仅限 isNew
- 数据变空时：空分支先清理存活克隆再插入 `data-for-empty` 克隆；数据从空变非空时移除残留 empty 克隆
- 消失 key 由 `oldKeys - newKeys` 清理（`__omni_cleanupElement` + remove）
- **契约**：keyed 重渲染 = 新增克隆追加 + 消失克隆移除 + 存活克隆原地保留（文本/类/属性绑定仍每渲染重跑）

### D2. 委派监听器数据新鲜度（list.rs）

- 每渲染 `ctr._forData = data`（在空检查**之前**）；`_forHandle` 用 `(ctr._forData || data)[data-for-item]` 解析 item——闭包 `data` 仅作回退
- **契约**：监听器一次挂载，但 item 解析永远基于最新渲染的数据快照

### D3. 事件参数契约（list.rs / lower/view.rs / expr.rs）

- `data-emit-*`（内联子组件 emit）→ `fn(e.detail)`；`data-event-*`（元素事件）→ `fn(_extractEvent(...))`
- **FnRef 与 InlineFn 同等扫描**：`e.stopPropagation()`/`e.preventDefault()` 在函数体内被 `scan_ir_event_methods`（IR 层）识别 → `needs_stop_propagation/needs_prevent_default` 标志 → listener 级对真实事件执行
- handler 体内 `e.stopPropagation()`/`e.preventDefault()` 编译为 **no-op**（listener 级已执行；handler 收到的是类型化事件数据对象，无这些方法）
- **契约**：事件方法调用 = 编译期指令；handler 永远只收到提取的事件数据

### D4. nid 唯一性与绑定查找（list.rs）

- `assignUniqueNids` 仅对 **isNew** 克隆执行（复用克隆保持创建时 nid，不累加后缀）
- 绑定查找：精确 `[data-nid="base_idx"]` 命中失败时回退前缀 `[data-nid^="base_"]`（复用克隆的后缀来自创建时 idx，与当前渲染 idx 无关）
- **契约**：克隆 nid 在生命周期内稳定；查找同时兼容新克隆（精确）与复用克隆（前缀）

### D5. 局部遮蔽与局部可变（scope.rs / store.rs / stmt.rs / expr.rs）

- `ComponentScope.fn_locals`：当前函数的参数 + `let` 声明名；`IdentifierExpr` 最先检查——命中时仅接受非信号 rename（保留 sanitize 映射如 `default → default_`）
- `MutationIntent::Push/Pop` 与 `DirectAssign` 同样检查 `is_signal`：信号走响应式路径（`.value.push` + `slice()` re-set），局部变量走普通 JS push/pop
- **契约**：函数局部名遮蔽任何 store 信号重命名；可变语句的目标是信号还是局部由解析结果决定

### D6. 后代选择器（parser/view/style.rs）

- lexer 丢弃空白 → 复合（`.a.b`）与后代（`.a .b`）通过 **token span** 区分：`prev.span.end < next.span.start` 即有空白 = 后代段
- 后代段支持 class/tag + 连字符 + 伪类（`.note-list li`、`.child-box strong:hover`）
- 仍不支持的组合器（`>`/`+`/`~`、`:checked` 等状态伪类）保持跳过逻辑
- **契约**：选择器链不再被静默截断；span 是空白判定的唯一依据

## Consequences

### Positive

* 8 类静默错误全部消除，浏览器 0 runtime errors
* keyed 列表的身份追踪承诺兑现（Todo 增删改全链路、中间项删除正确）
* 事件委派从「碰巧能跑」变为「契约明确」——`__omni_forItemVar`/`ctr._forData`/e.detail 三者职责分离
* 后代选择器让 demo 样式意图真正生效（`.note-list li` 的 margin 作用在 li 而非 ul）
* 回归测试固化：65 个 tests_regression 断言 + 全工作区 748+ 测试通过

### Negative

* `ctr._forData` 是运行时扩展属性（非 `data-*` 属性）——编辑器/WASM 桥接层需知道该约定
* 前缀 nid 查找（`^=`）假设克隆内同一 base nid 唯一——嵌套 @for 时需保持生成 nid 唯一性
* 事件方法 no-op 依赖 listener 级标志扫描的完备性——若未来出现未经扫描的调用路径（如新的事件类型），会静默无效
* 后代选择器 span 判定假设 lexer 不合并相邻标识符——若未来 lexer 改变空白处理需重审

### Neutral

* fn_locals 是 per-function 状态（emit_function_named / store action 发射时填充清空）——ComponentScope 自动纳入内联隔离
* 后代选择器支持范围限定 class/tag 段，复杂组合器仍是 deferral

## Implementation Strategy

### Blast Radius

**Impact Scope**: OmniView 后端 codegen（list.rs/expr.rs/stmt.rs/scope.rs/store.rs/element.rs）+ parser/view/style.rs + lower/view.rs；影响所有使用 `@for`、事件、store、style 块的 OmniView 输出。

**Affected Components**:
- `omnilang/src/codegen/omniview/list.rs` — D1-D4 全部
- `omnilang/src/codegen/omniview/expr.rs` — D5 IdentifierExpr / e 方法 no-op / map.contains
- `omnilang/src/codegen/omniview/scope.rs` — D5 fn_locals
- `omnilang/src/codegen/omniview/stmt.rs` — D5 Push/Pop is_signal
- `crates/omnilang-core/src/ir/lower/view.rs` — D3 FnRef 扫描 / D5
- `crates/omnilang-core/src/parser/view/style.rs` — D6
- 展示项目 learn-omniview（demo 源码修复，非编译器）

**User Impact**: 全部为正确性修复——用户可见行为从「错误但安静」变为「正确」；无 API 变化。

**Risk Mitigation**:
- 每项决策均有浏览器复现 → 修复 → 回归验证闭环
- 65 个 tests_regression 断言固化生成代码形状
- 全工作区测试（748+）在每次改动后运行

### Reversibility

**Reversibility Level**: High（编译器内部契约，无公共 API；回退 = revert commit）

**Rollback Feasibility**:
- 4 个 commit 各自独立可 revert
- 无数据迁移、无外部依赖
- demo 源码修复与编译器修复解耦（demo 修复回退不影响编译器）

**Options Preserved**:
- D4 前缀查找为未来「克隆 nid 完全稳定化」保留空间
- D6 的 span 判定为未来支持 `>`/`+` 组合器留了入口（段解析模式）

**Commitments Made**:
- keyed 重渲染语义：克隆复用 + isNew-only append（不重排存活克隆——排序变更需另行决策）
- 事件参数契约：emit 传 detail、元素事件传提取对象、方法调用为编译期指令

### Sequencing & Timing

**Prerequisites**:
- [x] 浏览器测试环境（omnic run + Playwright MCP + localhost HTTP server）
- [x] 回归测试固化（tests_regression/list.rs）
- [x] 全工作区测试基线

**Readiness Assessment**: Ready to implement（已实现并验证）

### Social Cost

**Learning Curve**: Medium——`ctr._forData`、`fn_locals`、isNew-only append 三个机制需要 codegen 开发者理解；文档见本 ADR 与 [[omnilang-codegen]] 记忆。

**Cognitive Load**: 低——机制数量少且职责单一（数据新鲜度、遮蔽、标志扫描各一个关注点）。

**Clarity Assessment**:
- **Will this help more than confuse?**: Yes
- **Documentation Needs**: 本 ADR + omnilang-codegen 记忆更新（@for 修复需补充记录）

### Confidence Assessment

**Model Correctness Confidence**: High——8 类缺陷全部浏览器复现后修复，回归断言覆盖生成代码形状。

**Assumptions**:
1. keyed 列表的 item 身份由 key 唯一决定（demo 均满足）— **Validation**: TodoStore nextId 自增不复用
2. 克隆内 base nid 唯一（前缀查找安全）— **Validation**: 生成器 nid 分配保证
3. 事件参数名约定为 `e` — **Validation**: 所有 demo handler 均用 `e`；扫描用函数实际参数名

**Uncertainty Areas**:
- 嵌套 @for + keyed + emit 组合（demo 无此场景）
- keyed 列表排序变更（DOM 不重排，仅新增/删除）
- 事件方法 no-op 若遇未扫描调用路径

**Edge Cases**:
- `data-for-empty` 克隆在嵌套 @for 中的清理
- 空数据早退与 `_forKeys` 残留（同 key 复用 detached 克隆，状态保留）
- store action 内参数遮蔽（emit_store 发射路径已覆盖）

## Implementation

已实现（4 个 commit）：

**Phase 1（a448b623）— @for 渲染/委派正确性**：D1 clear 门控 + 空分支清理 + empty 克隆移除；D2 `ctr._forData`；D3 emit e.detail；D4 isNew-only 后缀 + 前缀查找；空列表清理；bind:value 顺序（element.rs）；map.contains；动态 token `.value`

**Phase 2（7330e783 + 242a5e53）— 语义与遮蔽**：D3 FnRef 扫描 + e 方法 no-op；D5 fn_locals + Push/Pop is_signal；demo 源码修复（Modal/ThemeStore/FeatureCard/FormControls 等）

**Phase 3（d0ed59e5）— 语言能力**：D6 后代选择器；Todo 过滤接线；死代码/文案清理

## Alternatives Considered

### 始终重渲染 keyed 列表（放弃复用）

每次重渲染全部重新 clone，克隆不复用。

**Pros:** 实现简单，无 nid 累加问题
**Cons:** 丢失 keyed 身份追踪承诺（FLIP/enter/状态保留全失效）；与 ADR-032 动画机制冲突

### 监听器闭包每次重渲染重新挂载（放弃 data 缓存）

`data` 闭包改为每次渲染重新 addEventListener（remove 旧的）。

**Pros:** 无 `ctr._forData` 扩展属性
**Cons:** 事件监听器反复挂载/卸载（性能 + 时序风险）；`ctr.dataset.forEventsAttached` 守卫失效

### handler 接收真实事件对象（放弃提取对象）

`e.stopPropagation()` 直接可用，无需 no-op 特判。

**Pros:** 语义直观
**Cons:** 破坏「handler 只收到类型化事件数据」的语义契约（ADR-052 原则 1）；跨目标（React/Flutter）无法统一

### 选择器用空白重建 token（放弃 span 判定）

lexer 在 style 上下文保留空白 token。

**Pros:** 语义显式
**Cons:** lexer 侵入式改动影响全部语法域；span 判定零成本且已验证

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 6 项契约均为**已发生故障的修复**，非投机设计——每项都有浏览器复现的具体失败场景。复杂度来自问题本身的连锁性（事件委派机制的多层交互），而非抽象膨胀。

**Decision Challenge**:

**Proposed Decision**: "keyed @for 克隆复用 + 委派数据新鲜度 + 事件参数契约 + 局部遮蔽 + 后代选择器"

**Necessity Assessment**: 9/10
- **Current need**: 8 类缺陷全部在浏览器中复现（Todo 删除失效、Modal 误关、Notifications 残留、过滤失效等）——当前、具体、用户可见
- **Future need**: 契约本身是防御性的，但问题已发生
- **Cost of waiting**: 每个缺陷都是静默数据错误——等待意味着错误状态持续存在于所有 OmniView 输出
- **Evidence of need**: 4 轮浏览器全量测试 + 65 个回归断言

**Complexity Assessment**: 4/10
- **Added complexity**: 每项契约 ≤ 30 行核心改动；总净增 ~150 行编译器代码
- **Maintenance burden**: 低——契约集中在 list.rs/expr.rs 两处，其余为数据传递
- **Learning curve**: 3 个新概念（ctr._forData、fn_locals、isNew-only append）
- **Dependencies introduced**: 零新依赖

**Alternative Analysis**: 四个替代方案均评估——「始终重渲染」与「重新挂载监听器」被否决（破坏 keyed 语义/性能）；「真实事件对象」与「空白 token」被否决（破坏语义契约/侵入 lexer）。最小改动方案（仅修症状不立契约）不可行——同类缺陷已在两轮测试中反复出现，需要机制级修复。

**Simpler Alternative Proposal**: 无——每项决策已是问题域的最小正确解；D6 是唯一可延后的（仅 3 个 demo 样式受影响），但实现成本低（~40 行）且消除一类静默降级。

**Recommendation**: ✅ Approve decision

**Justification**: 全部为已复现缺陷的机制级修复；复杂度/必要性比 0.44，远低于 1.5 阈值；无投机设计（无新抽象、无新依赖、无未来特性预留）。

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 4/10
- **Ratio**: 0.44 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——修复真实故障并固化契约，复杂度与问题严重性相称，无过度设计。

## Validation

**Acceptance Criteria:**
- [x] Todo 增删改全链路（含中间项删除、删空显示 empty state）
- [x] Modal 内容点击不误关、overlay 点击关闭、Escape 关闭
- [x] Notifications Add/Remove/Clear 列表同步
- [x] Todo 过滤（filterStatus 驱动 visibleItems）
- [x] 后代选择器样式作用在正确元素（li margin 4px / ul 0px）
- [x] 浏览器 console 0 errors
- [x] 全工作区测试通过（omnilang-core 748 + omnilang 257 + tests_regression 65）
- [x] 65 个 tests_regression 断言覆盖生成代码形状（keyed 无顶层 clear、empty 分支清理、`ctr._forData`、`_emitAttr && !_evtAttr ? e.detail : ...`、`data-for-empty` 移除）

**Testing Approach:**
* 浏览器全量测试（omnic run + Playwright MCP）：9 页交互 + `document.getAnimations()` 动画验证
* 回归断言：tests_regression/list.rs（keyed/非 keyed/事件委派/嵌套）
* 编译产物检查：grep 断言生成代码形状

## References

* ADR-054 OmniView 语义 DSL 时代设计理念（原则 3：诚实降级 / 原则 4：运行时验证）
* ADR-052 三层语义架构（事件数据契约）
* commit a448b623 / 7330e783 / 242a5e53 / d0ed59e5
* session-2026-08-02-browser-test-round2（记忆）
