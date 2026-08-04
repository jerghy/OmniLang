# ADR-057: CodeFlowCanvas 语法节点扩展（语句级语法节点）

## Status

Implemented

## Context

CodeFlowCanvas（节点编辑器）基于 ReactFlow 实现，以节点编排 OmniLang 函数体逻辑，编译成 `.omni` 源码（`compileCodeFlowGraph` + `@pid` 注释 round-trip）。原设计（2026-07-10 spec）仅 4 种节点：`fn` / `code-block` / `condition` / `return`——`let`、赋值、store 调用等语句全部落入 `code-block` 自由文本，需双击打开 Monaco 手写，无法在画布上以结构化方式表达。

用户需求（2026-08-04）："节点编辑器加上常用的界面开发时会用的代码……不是一整块代码，而是一个语法。比如设置变量，修改变量或状态或 store"——即新增**语句级语法节点**。素材来源 `test/projects/learn-omniview/src` 真实统计：`let` 68 处、`let mut` 21 处、store 动作调用 6+ 处、`emit` 4 处。

约束（用户确认）：
- 一个语法一个节点（非整块模板、非 Monaco snippet）
- 右侧 FlowInspector 结构化表单编辑（与 fn/return 节点一致）
- **保存与解析严格符合现有设计**（`// @pid:xxx` round-trip 机制零改动）

## Decision Drivers

* **语句级可视化诉求**：设置变量/修改状态/store 是界面开发最高频语句，应作为结构化节点而非自由文本
* **复用现有 round-trip 机制**：`@pid` 注释 + GraphCompiler/GraphParser 双向管线已稳定，新节点必须无缝接入，保存重开节点形态不丢
* **单一代码来源**：props → 代码文本的格式逻辑只写一处（`formatSyntaxNode`），GraphCompiler 与 UI 预览共用，杜绝双实现漂移
* **语言合法性优先**：生成源码必须能通过 OmniLang 语言验证（嵌套 fn 声明非法——E0001，实现中发现并根治）

## Decision

新增 4 种语句级语法节点，贯穿全链路：

| tag | 节点 | 生成代码 | props |
|-----|------|---------|-------|
| `let` | 变量声明 | `let [mut] name[: 类型] = 值;` | `{ name, mutable, varType?, value }` |
| `assign` | 赋值/改状态 | `target op 表达式;` | `{ target, operator, value }`（`= += -= *= /= %=`） |
| `store-call` | store 调用 | `TodoStore.add(args);` | `{ storeName, action, args }` |
| `call` | 调用/emit | `close();` / `emit onSelect(x);` | `{ fnName, args, emit? }` |

**Architectural Components Affected:**
* `poiesis/src/services/codegen/syntax-nodes.ts`（新增）——`formatSyntaxNode` / `defaultSyntaxProps` / `ASSIGN_OPERATORS` 纯函数单一来源
* `poiesis/src/services/codegen/graph-compiler.ts`——emitNode 4 case（复用 formatSyntaxNode，`@pid` 注释格式不变）
* `poiesis/src/services/codegen/graph-parser.ts`——`detectSegmentType` 语句分类（8 种）+ `parseSyntaxProps` 正则提取（round-trip 互逆）
* `poiesis/src/components/canvas/code/`——flowConstants 菜单分类、SyntaxNode 组件（4 色系）、FlowInspector 4 表单
* `poiesis/src/services/codegen/parser.ts`——store action 解析侧 fn 元素合成（action 签名 → fn 入口，替代嵌套 fn 载体）

**Interface Changes:**
* `parseCodeFlowGraph` / `compileCodeFlowGraph` 签名不变，支持 8 种 tag
* 新节点 props 存 `CodeblockElementData.props`，`source` 为派生缓存（解析时 = `formatSyntaxNode(tag, props)`）
* 新节点**禁用双击 Monaco 编辑**（编辑器改 source 不生效，props 为权威）

## Consequences

### Positive

* **round-trip 双向稳定**：保存重开节点类型、props、连线完整保持（compile→parse→compile 幂等测试）
* **覆盖两条代码流**：组件 fn（pipeline.ts）与 store action（source-emitter.ts）共用同一实现，一处修改两处生效
* **统一语法识别**：手写 `let`/赋值/调用语句保存重开自动升级为对应语法节点（语义无损）
* **语言合法性强制**：store action 体不再含嵌套 fn（E0001 根治），生成源码可过 WASM 语义验证
* **端到端真实验证**：Victauri 实测画布添加/编辑/连线/保存/重进全链路 + 磁盘产物核验

### Negative

* **code-block 内容会被重分类**：现有 code-block 内含 `let`/函数调用的，保存重开会变为语法节点（预期行为变化，Task 3 已更新受影响测试）
* **store-call 节点在 store action 流中无法编译**：跨 store 调用语言不支持（WASM 测试已文档化），用户保存时仅见 `Undeclared variable 'XxxStore'` 诊断
* **多行手写语句的解析折中**：多行调用合并为单行匹配；无法匹配时保留原文（响亮失败留给 WASM 验证），不静默重写

### Neutral

* 遗留 operations/SlotValue 体系（CodeCard v3 废弃机制）未触碰，保留原状
* `// @<type>` 手写类型注释未新增（`detectSegmentType` 从代码文本分类已足够）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中等——限定在 poiesis 前端（codegen 服务 + 画布组件），编译器（omnilang crates）零改动

**Affected Components**:
- `poiesis/src/services/codegen/`（graph-compiler/graph-parser/parser/source-emitter/pipeline）— 节点类型联合扩展
- `poiesis/src/components/canvas/code/`（FlowCanvas/FlowInspector/SyntaxNode/flowConstants）— UI 层
- `poiesis/src/hooks/useSaveProject.ts` — store 保存同步（修复 3）

**User Impact**: 画布右键菜单新增 2 分类 4 节点；保存重开后既有 code-block 中 let/调用语句形态变化（语义不变）

**Risk Mitigation**: 全链路逐任务 TDD + 独立审查 + 真实 WASM 语义验证 + Victauri 端到端

### Reversibility

**Reversibility Level**: High

**Rollback Feasibility**: 代码级回退（回退合并即可）；已保存项目的 `@pid` 注释格式未变，旧代码仍可解析（新节点语句会回到 code-block 或按旧规则分类）

**Migration Paths**:
- **Forward Migration**: 新节点即新语法，无迁移
- **Rollback Migration**: `git revert` 合并 commit
- **Evolution Path**: for/while 循环节点（2026-07-10 spec 后续迭代）；`@<type>` 注释扩展

**Commitments Made**: `formatSyntaxNode` 为语法节点代码唯一来源；`@pid` round-trip 契约继续作为 CodeFlowCanvas 稳定性基础

### Sequencing & Timing

**Prerequisites**:
- [x] Task 1-7 全部完成（15 commits，逐任务审查 + 2 轮返工 + 最终修复波）
- [x] 232/232 tests 全绿 + tsc 零新增错误
- [x] Victauri 端到端验证（组件 + store 两条代码流）

**Readiness Assessment**: Ready to implement —— 已实现并合并 main（0e15baa7）

### Social Cost

**Learning Curve**: Low
- 4 种新节点的 props 结构一目了然；`formatSyntaxNode` 单一来源概念简单
- 表单与既有 FnInspector 模式一致

**Cognitive Load**: 低——语法节点是"一个语法一个节点"的直接映射，无新抽象

**Documentation Needs**:
- [x] 设计文档 `docs/superpowers/specs/2026-08-04-codeflow-syntax-nodes-design.md`
- [x] 实现计划 `docs/superpowers/plans/2026-08-04-codeflow-syntax-nodes.md`

## Alternatives Considered

### 方案 B：复用遗留 operations 机制（拒绝）

扩展 code-block 的 `operations` 字段（CodeCard v3 遗留的 DeclareVarOp/SetVarOp/CallFnOp + 补 StoreCallOp）。
**拒绝原因**：operations 是已废弃积木编辑器的遗留（SlotValue 体系复杂）；不满足"一个语法一个节点"诉求；需重建 SlotValue 表单 UI，工作量反而更大。

### 方案 C：语法节点但不做 round-trip（拒绝）

节点 + 表单 + 编译全做，GraphParser 不识别新语句（重开退化 code-block）。
**拒绝原因**：保存重开后节点形态丢失，与 fn/condition/return 的 round-trip 体验不一致。

### 实现期替代：嵌套 fn 作为 store action 载体（尝试后否决）

source-emitter 曾把完整 fn 定义 emit 进 action 体（round-trip 载体），被 WASM 语义验证否决——OmniLang 语句解析器不支持嵌套 fn 声明（E0001 'fn'）。
**终选**：emit 侧提取 fn 内部 body 只发语句（与 pipeline.ts 组件侧一致），解析侧从 action 签名**合成 fn 元素**（`<actionPid>_fn`）作为入口节点，round-trip 保持。

## Validation

- 232/232 vitest 全绿（新增 46 测试：syntax-nodes 8、graph-compiler 3、graph-parser-syntax 10、roundtrip 3、UI 3、回归 19）
- `store-action-wasm-validity.test.ts`：真实 WASM 模块编译 store 产物，零 E0001，负控制（嵌套 fn）仍被拒
- Victauri 端到端：右键菜单新分类 → 4 节点添加与表单编辑 → 连线 → 保存 → 重进类型保持 → `.omni` 含 `@pid` 注释与正确语句 → 真实编译通过；store action 跨重启 fn 入口保留
- 最终全分支审查（fable）：0 Critical，2 Important（round-trip 不对称：let 无值丢名/多行截断）+ 5 Minor → 修复波 5/5 ADDRESSED，无新破坏
- 磁盘产物核验：`TestCodeFlow.omni` / `demostore.omni` 的 `@pid` id 与验证报告逐字吻合（uid 随机 id 不可伪造）

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 用户明确需求（"一个语法一个节点"）+ learn-omniview 使用频率证据（let 68×/store 6+/emit 4×）驱动；复杂度来自与既有 round-trip 管线的正确融合（正则互逆、语言合法性），而非抽象膨胀。

**Proposed Decision**: "新增 4 种语句级语法节点（let/assign/store-call/call），formatSyntaxNode 单一来源，复用 @pid round-trip 机制"

**Necessity Assessment**: 8/10
- **Current need**: 用户明确提出（节点太少、需要设置/修改变量、状态、store 的语法节点）；learn-omniview 高频使用统计佐证
- **Future need**: 语句级可视化是代码流编辑器的自然演进方向（for/while 节点已在 spec 排队）
- **Cost of waiting**: 当前只能以 code-block 自由文本表达这些语句，无法结构化编辑
- **Evidence of need**: 用户需求确认 + learn-omniview 68×/21×/6+/4× 使用统计

**Complexity Assessment**: 3/10
- **Added complexity**: `formatSyntaxNode` 单一来源消除重复；GraphCompiler/GraphParser 扩展是既有模式的线性延伸（emit case + 分类分支）
- **Maintenance burden**: 低——语法节点核心集中在 syntax-nodes.ts 一个模块 + 两个管线函数的扩展
- **Learning curve**: 4 个 props 结构 + 1 个单一来源函数，与既有 FnInspector 模式同构
- **Dependencies introduced**: 零新依赖

**Alternative Analysis**: 方案 B（复用废弃 operations）被拒——复用遗留体系反而增加复杂度；方案 C（无 round-trip）以不完整体验换省事，被拒。所选方案是三者中最小完整实现。

**Simpler Alternative Proposal**: 只做 Monaco snippet（方案 B 变体）可省管线改动——但不满足"一个语法一个节点"的核心诉求，且 snippet 无法在画布上结构化表达。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 8/10
- **Complexity**: 3/10
- **Ratio**: 0.375 *(Target: <1.5)*

**Overall Assessment**: 适当的工程——用户需求明确、使用频率有证据，实现复用既有 round-trip 机制（零架构改动），核心逻辑单一来源，无过度设计。

## References

- 设计文档：`docs/superpowers/specs/2026-08-04-codeflow-syntax-nodes-design.md`
- 实现计划：`docs/superpowers/plans/2026-08-04-codeflow-syntax-nodes.md`
- 上游设计：2026-07-10 CodeFlowCanvas 代码生成 spec（节点体系基础）
- 遗留项（独立任务）：`validateSources` 未适配 ebd9cacd 信封（保存门禁静默失效，HIGH）；`Cargo.lock` 治理；store-call 在 store action 流中的菜单禁用或诊断映射
