# ADR-068: View 属性三态保真（class="x" / 裸布尔 / prop={expr} 动态属性）

## Status

Implemented

> 2026-08-06：7 任务 SDD + 真机验证 643/643 + tsc 0

## Context

Poiesis 设计画布对 view 元素属性的解析/发射存在三类数据破坏缺口（learn-omniview 真实使用量）：

| 语法 | 使用量 | 缺口 |
|------|--------|------|
| `class="x"` 属性形式 | simple.omni | 解析静默丢弃（parser-view STRING 分支跳过 class） |
| 裸布尔属性 `<input disabled>` | FormPage 等 8 处 | 非 EQUALS/COLON 即 **break 整个属性循环**（后续属性全丢） |
| `prop={expr}` 动态属性 | 遍布 30+ 文件 | 只吃单 IDENTIFIER 且**丢花括号** → `value={count}` 保存变 `value="count"`（动态→静态退化）、`value={fn(x)}` 解析失败 |

**根因**：`ViewElementData.props: Record<string, unknown>` 的值无法区分「静态文本」与「动态表达式」——解析时花括号必须丢弃（否则发射端会把 `{count}` 当字面文本输出）。

**三个关键代码事实**（决定设计空间）：
1. `applyStyles` 是空函数（collect-styles.ts:15）→ 发射端 `style.htmlAttrs` 分支是死代码，**props 路径（source-emitter.ts:618）就是真实路径**，只需改一处
2. UI 的 label 芯片编辑器已有既定约定：**props 字符串值中 `{expr}` = 动态表达式**（parseLabelSegments/serializeSegments）；`/` 信号插入已对所有属性输入框生效——**编辑能力已就绪，缺的只是解析/发射不认该约定**
3. `props` 值类型是 `unknown`——**天然容纳 string/boolean**，类型层面无需 schema 改动

## Decision Drivers

* 数据破坏级缺口优先修复：打开→保存即丢代码（不是「缺功能」是「损坏代码」）
* 与既有约定统一：`{expr}` 字符串约定是 label 芯片编辑器已验证的模式，推广而非发明
* 不打断思路：芯片编辑器对纯静态用户退化 = 普通输入框，创建表达式 = 打字（输入层零变化）
* 最小改动面：只改真实路径（applyStyles no-op 事实），不触碰死分支
* 测试即契约：解析与发射必须同时改同时测（杜绝 @for keyExpr 式解析有发射无的不对称）

## Decision

**props 值三态约定**（零 schema 改动，与 label 芯片既有约定统一）：

| 值形态 | 含义 | 发射 |
|--------|------|------|
| `"Enter name"` | 静态文本 | `placeholder="Enter name"` |
| `"{count}"`（trim 后 `/^\{(.+)\}$/s` 且 `$1` 非空） | 动态表达式（任意合法表达式） | `value={count}` |
| `true` / `false` | 裸布尔属性开 / 关 | `disabled` / 不发射 |

**三端同步实现**：
- **解析**（parser-view.ts）：BRACE 属性值配平花括号收集任意表达式（列距空格插回，@if 同款模式）存 `"{expr}"`；裸布尔属性 → `true` + **continue**（修复 break 丢后续属性 bug）；`class="x"` 与 `class: "a b"`（连带）统一走 className → classRefs 管线；`id={x}` 保持旧行为（D7：动态 id 零使用，el.id 承担节点身份）
- **发射**（source-emitter.ts:618 props 路径）：`{expr}` 形态 → `key={expr}`；boolean → 裸属性；静态 → 原样。`style.htmlAttrs` 死分支不动
- **UI**（PropsSection.tsx）：LabelEditor 参数化为 ChipEditor 推广所有属性行（id 行除外）；BoolPropRow 布尔行——开关 ↔ 表达式**互斥视图**（有表达式时开关隐藏显示紫色芯片，杜绝「开关和输入框并存该信谁」歧义）；`/` 插入信号 = 光标位置插裸信号名（I1）+ blur 竞态守卫（slashPendingRef，ChipEditor 同款）
- **BOOLEAN_ATTRS 共享常量**（codegen/boolean-attrs.ts，19 项）：emitter 原 18 项 + autocomplete（Rust parser 侧本就视其为布尔，element.rs 核对）——parser/emitter/UI 同源

**Architectural Components Affected:**
- `poiesis/src/services/codegen/parser-view.ts`（属性解析）
- `poiesis/src/services/codegen/source-emitter.ts`（属性发射）
- `poiesis/src/services/codegen/boolean-attrs.ts`（新建共享常量）
- `poiesis/src/services/codegen/parser-types.ts`（attrs 类型加宽 `string | boolean`）
- `poiesis/src/components/panels/inspector/PropsSection.tsx`（芯片编辑器推广 + 布尔行）

**Interface Changes:**
- `ViewElementAttrs.attrs: Record<string, string>` → `Record<string, string | boolean>`（三态约定所需最小加宽，消费端已核兼容）
- `BOOLEAN_ATTRS` 从 source-emitter 私有静态提升为共享导出

## Consequences

### Positive

* **打开→保存不再丢代码**：class="x"、裸布尔、动态属性三缺口闭合（learn-omniview 对应文件往返保真）
* **动态属性可编辑**：任何属性行可输 `{expr}`（回车自动拆成紫色芯片），布尔属性有开关
* **顺带修复两个隐藏 bug**：裸布尔 break 截断后续属性；`class: "a b"` 字符串类名丢失（ViewAttributesDemo 3 处）
* **UI 输入层零变化**：芯片只是渲染层视觉区分，创建表达式 = 打字（与 label 芯片既有交互一致）
* **发射端单点修改**：只改 props 真实路径（applyStyles no-op 事实）

### Negative

* **`{expr}` 字符串约定需两端同步维护**：解析/发射/UI 三端必须共享同一形态判定（`/^\{(.+)\}$/s`）——违反则产生新不对称（评审流程已加「三端对称」核查项）
* **`class="x"` 输出改写为 `class: x`**：语义等价但保存后 diff 可见（格式重排本已存在，PID 注释体系已接受）
* **slashPendingRef 怪癖扩散**：ChipEditor 既有「Escape 后闩锁残留吞一次提交」行为随推广扩散至所有属性行（既有模式缺陷的镜像）
* **WASM 门禁拦截未声明信号**：`value={count}`（count 未声明）保存被拦截——正确防御但用户可能困惑（无 UI 提示）

### Neutral

* `value={expr}` 与 `bind:value` 可并存发射（用户输入组合，编译器为最终闸门——D10 范围外）
* 芯片 × 删除键盘不可达（onKeyDown delete 分支死代码，ChipEditor 同款既有）

## Implementation Strategy

> 已实现（2026-08-06，SDD 7 任务 + 真机验证）。以下为执行期事实记录。

### Blast Radius

**Impact Scope**: 低——改动集中在属性解析/发射/UI 三个文件；发射只改 props 路径（死分支 htmlAttrs 不动）；`attrs` 类型加宽经消费端全链核验（variantRaw `as string` 断言、cleanAttrs 进 `Record<string, unknown>` props 槽位均无 string-only 假设）

**Affected Components**:
- parser-view.ts - 属性分支重写（BRACE 配平/裸布尔/class），既有测试 279→292 全量回归
- source-emitter.ts - props 循环加三态分发，bind-value/roundtrip 回归
- PropsSection.tsx - 芯片推广 + 布尔行，label 行为逐字保留（评审逐行比对确认）

**User Impact**: 画布保存后属性不再丢失/退化；属性面板出现芯片与开关（视觉增强）

**Risk Mitigation**:
- 测试即契约：9 行往返矩阵（解析中间态 + 发射 + 闭环 parse→emit）
- 评审强制 tsc 门（T2 教训：vitest 走 esbuild 不做类型检查，TS2322 曾全绿掩盖）
- 真机验证（Victauri + demo2）全链路：开关三态/芯片编辑/保存落盘/WASM 门禁

### Reversibility

**Reversibility Level**: High——值三态约定是数据形态而非结构变更；回退只需还原 5 个文件（无 schema/存储迁移）

**Rollback Feasibility**: 高——git 提交链完整（f540e36d..02026d4d + d42573da），单 commit 回退粒度

## Alternatives Considered

* **值对象包装**（`{kind: "static"|"expr", value: string}`）：schema 改动大（props 全量迁移）、与 label 既有 `{expr}` 字符串约定不一致、UI/发射端全部适配——**拒绝**（过度）
* **表达式 AST 建模**（解析时表达式结构化为节点）：Poiesis 无表达式 AST 基础设施（expression-lowering 只是字符串拼接），成本高且与「编译器为最终闸门」哲学冲突——**拒绝**
* **前缀标记**（`$expr$count` 等转义）：脆弱（与用户输入冲突面大）、无既有先例——**拒绝**
* **文本 fallback 解析**（发射时探测 `{...}` 形态）：本设计已采用（判定在发射端正则）——但解析端同时存 `"{expr}"` 原文是必要前提（否则花括号信息已丢，发射无从探测）

## Validation

* 解析往返矩阵 9 行 + 闭环 2 行（`<input disabled placeholder="x">`、`value={count}` parse→emit）全绿
* 全量 643/643 vitest + `tsc --noEmit` exit 0
* 真机（Victauri + demo2）：布尔开关三态、表达式芯片互斥视图、芯片点击编辑（bug 修复后）、`disabled={newSignal}`/`value={newSignal}` 保存落盘、WASM 门禁拦截未声明变量
* 真机抓到 1 个单测盲区 bug（BoolPropRow `isExpr` 屏蔽 `editingExpr`，芯片点击编辑不可达）→ 修复 + 回归用例
* learn-omniview 全项目保存被 WASM 门禁拦截（第一类缺口 @motion/@slot/@effect 未支持）→ 门禁保护项目零污染——**全项目保存无损验收依赖第一类缺口修复，记入后续计划**

## References

* Spec: `docs/superpowers/specs/2026-08-06-view-attributes-design.md`（D1-D10）
* Plan: `docs/superpowers/plans/2026-08-06-view-attributes.md`
* 研究文档: `docs/2026-08-06-view-syntax-usage-guide.md`
* SDD workspace: `.superpowers/sdd/2026-08-06-view-attributes/`（含真机验证记录）

---

## Pragmatic Enforcer Analysis

**Necessity Assessment (0-10)**: **9**——三类缺口均为数据破坏级（打开→保存即丢代码/退化），真实使用量明确（learn-omniview 遍布）；不做则编辑器对含动态属性的项目保存即损坏。当前需要 9 / 未来需要 9（只会更多文件使用）/ 等待成本高（每保存一次损坏一次）/ 证据充分（研究文档逐文件盘点）

**Complexity Assessment (0-10)**: **3**——核心是「值三态约定 + 三端各一小段逻辑」；零 schema 改动、无新依赖、无新抽象；主要复杂度在 BoolPropRow 交互（互斥视图 + slash 守卫），且全部镜像既有 ChipEditor 模式

**Alternative Analysis**: 值对象包装/表达式 AST/前缀标记均已评估并拒绝（见 Alternatives）；本设计选择的「`{expr}` 字符串约定」是**复用既有 label 芯片模式**——不是新发明，是推广已验证的约定

**Simpler Alternative Proposal**: 更简方案「只修解析不修 UI」（保真但不给编辑入口）——被拒：D9 前置（布尔属性不在白名单）证明无 UI 则控件不可达，编辑能力是本次用户明确需求（AskUserQuestion 确认芯片推广 + 开关）

**Recommendation**: **Approve**（本设计本身就是 pragmatic 的选择：零 schema、复用约定、最小改动面）

**Pragmatic Score**: Necessity 9 / Complexity 3 / **Ratio 0.33**（< 1.5 目标 ✓）

**Overall Assessment**: Appropriate engineering——数据破坏级缺口用最小面修复，无过度设计
