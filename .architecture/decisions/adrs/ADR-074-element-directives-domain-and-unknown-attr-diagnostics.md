# ADR-074: 元素指令语义域登记与未知属性诊断策略（语法审计收口）

## Status

Implemented

> 全部决策已实现并测试通过：域 7 登记 + 脚本断言（三方对齐计划 0ed2adfb..78f3fbfb）；B2 修复 + minors 收口（遗留收口计划 eb651652..a2b1fa75）。

## Context

语义集登记册（`docs/superpowers/specs/semantic-universe.md`）维护约定：**新增语法必须先在本全集登记**——「这是不是前端功能」由语义域决定，实现进度单独在状态列跟踪，漂移即门禁失败（`verify-semantic-universe.py` 双向断言）。2026-08-01 后实现的大批新语法（@drag/@dragEnd 意图指令 ADR-073、style 直写通道、view 属性三态 ADR-068、语义绑定全集）未登记——登记册标注「@drag 尚非登记册成员，演进见后续 spec 同步」。

语法挖掘三方对齐审计（2026-08-07）发现两个真实缺陷：

1. **登记册缺口**：新语法不在 6 域（可动画属性/数组操作/事件/RefOp/design token/值绑定）覆盖内——`verify-demo-coverage.py` 审计无法覆盖这些成员（审计盲区永久存在）。
2. **B2 编译器 bug（零诊断静默退化）**：`<details open={isOpen}>` 中 `open` 不在 OmniView `emit_element_props` 已知属性白名单（~70 项）也不在 `HTML_BOOLEAN_ATTRS`（12 项）→ 落入 `_ =>` fallback → 静态发射 `data-open="{isOpen}"`，**零 `__omni_effect`、零诊断** → `<details>` 运行时恒关闭；React 目标正确透传。同一通道影响所有白名单外属性动态绑定。
3. **管线级缺陷（审计深挖发现）**：`build_file_output` 用 `CompileOutput::success`（warnings 恒空）→ **OmniView generator 全部警告（未知属性/未定义 class/未知 variant）在 CLI 管线被系统性丢弃**——连已有的警告都不显示，诊断契约名存实亡。

用户拍板：**文档对齐编译器（保守）**——审计期编译器 bug 只列清单不修（`tasks/omniview-syntax-audit-bugs.md`）；遗留收口计划期修复 B2（方向：增补 open + warning）。

## Decision Drivers

* **语义集维护约定**：新增语法先登记 → 才可进 BNF/文档/白名单；登记册与编译器事实双向锁定（`verify-semantic-universe.py` 断言）
* **「零诊断静默退化」模式治理**（[[omnilang-bugfixes]] 模式 A/B/C）：诊断是契约——静默丢弃绑定 = 用户无感知的语义丢失
* **用户拍板三选**（遗留收口澄清）：B2 = **增补 open + warning**（治标 + 治本）；deferred minors 全修；React Doctor 报告（并行会话 codeflow 工作面）不纳入
* **既有 ADR-068/071/073 配套**：域 7 成员是这些决策的语法产物，登记册需同步
* **「高频 = 简单直接」哲学（ADR-070）**：修复保持最小——open 一行入册 + warning 一个分支 + 转发 3 行

## Decision

### 决策 1：语义集新增域 7「元素指令与绑定」——一次全收 8 成员

| 成员 | 语义 | 事实源 | 状态 |
|------|------|--------|------|
| drag | 拖拽意图指令（指针捕获编译器选路） | `parse_event_prop` drag 分派 + `validate_drag.rs` + `DragBindingIR` | 支持（三目标） |
| dragEnd | 拖拽结束回调 | 同上 | 支持 |
| ref | 元素引用绑定（`@ref={name}` / `ref: name` 等价） | `ref_binding` + `RefDeclIR` | 支持 |
| focus | 声明式焦点绑定（`focus={sig}`） | `SEMANTIC_BINDINGS` 常量 | 支持 |
| scroll-x / scroll-y | 声明式滚动绑定 | 同上 | 支持 |
| style 动态绑定 | `style: { prop: sig }` 动态条目 → D1/D2 直写通道 | `codegen/shared/direct_write.rs` | 支持（三目标） |
| 布尔属性 | 裸布尔属性形态（parser 15 关键字） | `peek_is_boolean_attr_keyword` + `HTML_BOOLEAN_ATTRS` | 支持 |

**机制级条目例外（脚本断言架构）**：6 个词法级条目（drag/dragEnd/ref/focus/scroll-x/scroll-y）走通用 `assert_set`（白名单 == 编译器词集合）+ `assert_matrix`（状态矩阵 ↔ 白名单）；2 个机制级条目（style 动态绑定/布尔属性）**不走**通用矩阵子集规则——其语义集条目名非词法级（是机制/形态名），无法进词集合比对，走**存在性断言**（`direct_write.rs` 分类函数存在 + `peek_is_boolean_attr_keyword` 关键字非空）。这是 `verify-semantic-universe.py` 的 per-domain 例外设计——机制级条目状态 ∈ {支持, 部分} 才执行存在性检查（未支持条目不报「declared 支持」误导消息，Task 2 收口）。

### 决策 2：B2 修复——open 入册 + 未知裸属性动态绑定 warning

**2a. `open` 增补**：`HTML_BOOLEAN_ATTRS` 12→13（+open）+ 已知属性白名单 match 增补 → 走既有布尔属性 DOM property 分支 `el.open = !!({js_access})`（`__omni_effect` 内）。布尔属性必须走 DOM property 而非 setAttribute（`setAttribute("open", "false")` 中 "false" truthy 无法取消——HTML 中 open 属性存在即 true）。

**2b. fallback warning（治本——零诊断 → 诊断）**：`_ =>` 分支对**裸未知属性名 + 动态值**（非 `LiteralExpr`）生成 warning：「unknown attribute '{name}' with dynamic binding: OmniView emits it as static data-{name} (not reactive); add the attribute to the known-attribute whitelist or use a known attribute」。**data-*/aria-* 前缀保持静默降级**——用户自定义透传通道（`data-tag={count}` 是用户的显式通道，非语义丢失），有意设计。

**2c. CLI 管线修复（深挖发现的配套缺陷）**：`build_file_output` 转发 `gen.warnings` → `CompileOutput.warnings`（原 `CompileOutput::success` 恒空——OmniView generator 全部警告在 CLI 系统性丢弃）。核验：React（`react/mod.rs`）/Flutter（`flutter/mod.rs`）早已接线，**OmniView 是唯一仍丢警告的警告产生后端**；legacy dart/js/rust 从不收集警告（零 warnings 引用）无需动。修复后 CLI 警告经既有通道（`CompileOutput.warnings` → Diagnostic → stderr，`commands/common.rs`）输出且**不使构建失败**（既有测试 `cmd_build_surfaces_warnings_without_failing` 保障）。

## Architectural Components Affected

* **语义集登记册**：`docs/superpowers/specs/semantic-universe.md` 域 7（三节结构：语义域定义/语义集条目/已实现白名单）
* **门禁脚本**：`scripts/verify-semantic-universe.py`（`DOMAIN_TITLES` 7 + `range(1,8)` + 提取函数 `drag_binding_words`/`semantic_binding_words`/`boolean_attr_keywords` + 机制级存在性断言）；`scripts/verify-demo-coverage.py`（域 7 usage 分支 + EXEMPT 更新——**drag 减法为纯负项必须删除**：`@drag\s*=` 因 `-` 拦截永不匹配 scrubbing 形态，减法减去从未计入项致 drag×-1 假阴性进 shown）
* **OmniView codegen**：`omnilang/src/codegen/omniview/view/element.rs`（`HTML_BOOLEAN_ATTRS` 13 + 白名单 match + fallback warning）
* **CLI 管线**：`omnilang/src/codegen/omniview/utils.rs`（`build_file_output` warnings 转发）
* **展示项目**：`test/projects/learn-omniview/src/pages/ElementRefDemo.omni`（scroll-x 语义绑定演示 + 文件头清单）

**Interface Changes**：
* CLI 行为：OmniView 目标的未知属性动态绑定从「零诊断静默」变为「warning + data-* 静态降级」
* 登记册 API：域 7 条目进入 `verify-semantic-universe.py`/`verify-demo-coverage.py` 断言范围（新语法未登记 = 门禁失败）

## Consequences

### Positive

* **零诊断 → 诊断**：未知属性动态绑定现在有 warning（B2 治本）；CLI 全部 generator 警告可见（管线修复）
* **open 可用**：`<details open={isOpen}>` 正确生成 `el.open = !!()` effect（B2 治标）
* **登记册与编译器事实双向锁定**：域 7 8 成员被脚本断言保护——实现漂移即门禁失败；demo 覆盖审计覆盖新语法（scroll-x 等缺口被发现并补齐）
* **CLI 警告可见性**：未定义 class/未知 variant 等既有警告从「系统性丢弃」变为「正常输出」

### Negative

* **parser 15 关键字 vs codegen 13 项集合不对称**：parser `peek_is_boolean_attr_keyword` 含 hidden/autocomplete/autoplay 而无 open；codegen `HTML_BOOLEAN_ATTRS` 含 open 而无前三者——历史遗留（非本次引入），文档双源如实标注；若未来做布尔集合一致性门禁需统一
* **编辑器 canvas 门控行为变化**：`resolve_styles`/`resolve_styles_multi`（lib.rs）以 `gen_output.warnings.is_empty()` 门控 success——新 warning 使含未知属性动态绑定的项目在编辑器画布从 success=true 变 false（pre-existing 模式，方向符合诊断目标，warnings 数组同时返回可供展示；Poiesis 侧观察清单）
* **warning 无 source span**（file/line/col = None/0，`-:0:0` 惯例）——与既有 codegen 警告一致，但定位体验弱于语义诊断

### Neutral

* **② 未实施**（fallback 对 data-*/aria-* 生成 setAttribute）——有意设计：用户自定义透传通道保持静默
* **legacy dart/js/rust 目标**不收集警告——无需改动（零 warnings 引用）
* **minors 收口**（deferred minors 8 项）：脚本注释/断言消息/死检查/表述对齐——零行为变化（除 M-2 门控语义）

## Implementation Strategy

**Blast Radius**：OmniView codegen 属性发射（全部元素属性路径）+ CLI 警告管线（全部 OmniView 构建）+ 门禁脚本（登记册/展示项目双向）。React/Flutter codegen 不受影响（不同文件、已独立接线）。回滚 = revert 3 个 commit（eb651652/80b0279c/375deac6）——纯增量无迁移。

**Reversibility**：High——增量改动、无数据迁移、门禁脚本可整体回退。

**Sequencing**：域 7 登记 → 脚本断言扩展 → demo 审计接入 → 文档核对 → demo 补齐 → B2 修复 → minors 收口。登记册先行（维护约定顺序），B2 修复独立 commit（codegen 与脚本无耦合）。

**Readiness**：Ready（已实现）。

## Implementation

**Phase 1：三方对齐（8 commits，0ed2adfb..78f3fbfb）**
* 域 7 登记 + spec 事实源修正（属性三态双源：parser 15 关键字 + codegen 12 项——「19 项」实为 Poiesis TS 侧常量，事实源描述错误被审计发现）
* `verify-semantic-universe.py` 域 7 断言 + `verify-demo-coverage.py` 域 7 审计 + EXEMPT 更新
* 文档核对（16 份零失实；`<details open>` 编译通过 → B1 关闭）+ bug 清单建立
* ElementRefDemo scroll-x 补齐（审计缺口唯一项）

**Phase 2：遗留收口（4 commits，eb651652..a2b1fa75）**
* B2 修复（open 13 + warning + 管线转发 + 4 测试）
* minors 8 项收口（脚本 5 + demo 2 + 清单 1 + 登记册 12→13 同步）
* 游离副本归位（tasks/task-5-report.md → ref 计划 workspace）
* bug 清单状态闭合（B2 待批 → 已修复注记）

## Alternatives Considered

### B2 修复方向

**仅增补 open**：最小改动（open 可用），但白名单外属性仍零诊断静默——治标不治本，同类 bug 复发（borderRadius/fontSize 白名单教训：白名单随实现修正而变动）。**否决**。

**通用 fallback setAttribute**：fallback 对所有 `{expr}` 动态值生成 setAttribute 效果 + 未知属性 lint——全面但改动面大（@for 模板路径/URL 消毒/布尔语义交叉），跨目标回归风险高，与「高频 = 简单直接」哲学冲突。**否决**（列为 ② 未实施方向，触发条件：白名单外属性动态绑定成为实际需求）。

**继续搁置**：保守延续「只列不修」——但用户拍板修复，且零诊断静默退化是项目明确定义的缺陷模式（[[omnilang-bugfixes]]）。**否决**。

### 域 7 边界

**拆分登记**（指令/绑定组 vs 动态 style/属性三态分开）：脚本改两遍、审计盲区永久存在（不收的两项无登记册域）。**否决**——一次全收（用户拍板）。

**仅词法级条目登记**（6 成员，机制级不登记）：动态 style/布尔属性无登记册域 → `verify-demo-coverage.py` 永远覆盖不到（审计盲区）。**否决**。

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 低复杂度——修复核心是 3 处小改动（open 入册一行、warning 一个分支、转发 3 行）+ 文档登记。机制级存在性断言例外是脚本架构的唯一新增概念，必要性由「条目名非词法级」的客观结构决定。

**Decision Challenge**: "B2 修复 = open 入册 + fallback warning + CLI 管线转发"

**Necessity Assessment**: 9/10
- **Current need**: B2 是真实静默退化（`<details open>` 恒关闭零诊断）；管线缺陷使全部 OmniView 警告不可见——当前用户可感知
- **Future need**: 白名单外属性动态绑定是持续风险面（任何新属性都走同一通道）
- **Cost of waiting**: 用户写 `<details open={isOpen}>` 得到静默错误行为——每多一天多一份误导；诊断契约是项目级原则
- **Evidence of need**: 实证链完整（产物 data-open 静态发射/运行时恒关闭/零诊断；React 透传对照）

**Complexity Assessment**: 2/10
- **Added complexity**: 一个分支条件（3 个谓词）+ 一个常量项 + 一个字段转发——无新抽象
- **Maintenance burden**: 低——warning 文案与白名单随属性演进，与既有 70 项白名单同维护面
- **Learning curve**: 无（既有模式复用）
- **Dependencies introduced**: 零

**Alternative Analysis**: 三方向（仅 open / 通用 setAttribute / 搁置）均被考虑并有明确否决理由（治标不治本 / 改动面大 / 用户拍板修复）。无「do nothing」缺口——搁置即被讨论且否决。「最小 change」选项（仅 open）被否定是因为 warning 是治本目标（零诊断→诊断）的核心——两者组合是「最小可行修复 + 诊断契约」的平衡。

**Simpler Alternative Proposal**: 无——当前方案已是每个缺陷的最小修复（open 一行、warning 一分支、转发 3 行）。域 7 登记的机制级例外是脚本断言的结构性必需（条目名非词法级无法进词集合），非过度设计。

**Recommendation**: ✅ Approve decision

**Justification**: 必要性 9/10（真实缺陷 + 诊断契约 + 用户拍板），复杂度 2/10（无新抽象、复用既有模式）。B2 修复的每一部分都有实证支撑；域 7 登记是维护约定的强制执行。ratio 0.22 远低于 1.5 阈值——这是「当前问题的当前修复」，无投机性设计。②（通用 setAttribute）被明确搁置为触发式扩展，符合 YAGNI。

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 2/10
- **Ratio**: 0.22 *(Target: <1.5 for balanced mode)*

**Overall Assessment**: 适当工程——修复针对实证缺陷、复杂度最小、无投机抽象；搁置项（② setAttribute、布尔集合一致性门禁）有明确触发条件。

## Validation

**Acceptance Criteria:**
- [x] `open` 动态绑定生成 `el.open = !!()` effect（测试 + release 实证产物）
- [x] 无 `data-open` 发射（测试断言 + 产物 grep 0 出现）
- [x] 裸未知属性动态绑定输出 warning（测试 + CLI stderr 实证）；data-*/aria-* 前缀静默（测试）
- [x] CLI 警告可见（`CompileOutput.warnings` 转发守卫测试 + `cmd_build_surfaces_warnings_without_failing`）
- [x] 域 7 8 成员登记 + 脚本断言（`verify-semantic-universe.py` 0 problem）；demo 8/8 展示（缺口 0）
- [x] 全量回归：cargo test 432/432 + omnic 15/15 + 5 门禁全绿

**Testing Approach:**
* 4 新测试（omnilang codegen：el.open 产物/无 data-open、裸未知属性 warning、data- 前缀静默、warnings 转发守卫）——RED 2/3 → GREEN
* 脚本断言（verify-semantic-universe.py 域 7 交叉断言 + verify-demo-coverage.py 域 7 审计）
* release 二进制实证（`cargo build --release -p omnic`——omnic CLI 是独立 crate）+ omnic build 产物 grep + CLI stderr 检查

## References

* [ADR-068](ADR-068-view-attributes-three-state-fidelity.md) / [ADR-071](ADR-071-ref-semantic-compiler-responsibility.md) / [ADR-073](ADR-073-drag-intent-semantics.md)——域 7 成员的语法来源
* 设计文档 `docs/superpowers/specs/2026-08-07-omniview-syntax-three-way-alignment-design.md`
* 计划 `docs/superpowers/plans/2026-08-07-omniview-syntax-three-way-alignment.md` / `2026-08-07-omniview-leftovers-fix.md`
* bug 清单 `tasks/omniview-syntax-audit-bugs.md`（B1 关闭 / B2 已修复注记）
* SDD ledgers：`.superpowers/sdd/2026-08-07-omniview-syntax-three-way-alignment/` / `.superpowers/sdd/2026-08-07-omniview-leftovers-fix/`
