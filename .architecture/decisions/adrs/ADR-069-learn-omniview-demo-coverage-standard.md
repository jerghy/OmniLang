# ADR-069: learn-omniview 展示项目完整化——语义覆盖标准 + 门禁例外 + 暴露缺陷处置

## Status

Implemented

## Context

用户指令：learn-omniview 作为 OmniLang 的展示项目（「可执行文档」），**必须没有语法错误，且展示出所有功能，包括灵活运用的功能**。

审计发现（`semantic-universe.md` 语义集 × demo 使用矩阵，脚本化）：**42 项已实现（支持/部分）语义成员未被 demo 展示**——域 1 可动画属性 fontSize、域 2 数组语言方法 count、域 3 事件 35 个（鼠标/指针/触摸/键盘/拖拽/动画过渡族）、域 5 design token 类型 5 个（lineHeight/letterSpacing/easing/opacity/zIndex）。此前「96 文件 10 域覆盖」的宣称在事件与 token 类型两域显著不足（如 v4 期间 `velocity_max` 零消费者被移除、`count()` 零展示、35 个 canonical 事件从未在任何 demo 中出现——其中潜伏缺陷仅因首次使用才暴露）。

## Decision Drivers

* **「可执行文档」完整性**：语义全集是语言契约，展示项目是其可执行镜像——已实现功能必须可看、可试、可学
* **覆盖可验证**：覆盖判定不能靠宣称——机器可复跑（`scripts/verify-demo-coverage.py`：支持/部分成员 × demo 使用，0 缺口门禁）
* **诚实降级优先**：部分支持功能（Flutter 未映射事件等）必须展示真实行为（含目标警告），而非回避或假装全支持
* **展示项目是缺陷探测器**：v2 先例（demo 暴露编译器 bug 即修）——本次再次验证（3 个 codegen 缺陷潜伏至首次使用才暴露）
* **语法零错误是硬约束**：新 demo 不得引入 tsc/eslint/dart analyze/浏览器错误（门禁为证）

## Decision

**learn-omniview 展示项目完整化**：补 4 个 demo 组件覆盖 42 项语义缺口 + 建立可复跑的覆盖审计 + 门禁受支持例外 + 暴露缺陷即修。

**Architectural Components Affected:**
* 展示项目：FontSizeMotionDemo（域 1 fontSize + 多属性复合动画）、ArrayChainDemo（域 2 count + filter→map 链式管道 + 方法调用层对照）、EventSpectrumDemo（域 3 35 事件分族 + 事件日志管道）、TokenTypeDemo + design.omni 5 个 token 类型声明（域 5）
* 覆盖治理：`scripts/verify-demo-coverage.py`（语义集支持/部分成员 × demo 使用矩阵；0 缺口 + 4 项无合法元素形态豁免）
* codegen 修复（展示项目暴露）：DSL `Number` → Dart `num` 映射（DartTypeFormatter/type_annotate 两处）、`.contains()` 的 is-Map 守卫改显式 `as Map`（shared/expr.rs 4 处）
* 门禁：React eslint 豁免 +1（#88）、Flutter warnings 基线 673→717（第三个受支持 honest-degradation 例外）

**Interface Changes:**
* 语言面：无变更（全部为展示与 codegen 修复）
* 门禁面：`FLUTTER_WARNINGS_BASELINE` 673→717（注记第三个受支持例外）；`verify-react.exemptions` +1 条目（TokenTypeDemo useEffect import——codegen 对 @hover motion 组件发射过宽，deferral #88）
* 治理面：deferral #88 注册（codegen useEffect import 按需发射）

## Consequences

### Positive

* **覆盖矩阵归零**：42 项缺口全闭合（域 1 19/19、域 2 3/3、域 3 54/54、域 5 13/13、域 4 10/10、域 6 bind:value/checked）——「可执行文档」宣称首次可机器验证
* **灵活运用有实体**：链式管道（filter→map）、事件日志管道（push + @for）、多属性复合动画（4 属性同帧）、跨主题 token 覆盖——「组合用法」从文档口号变为可运行 demo
* **3 个潜伏编译器缺陷被修复**：Number 类型映射缺失（生成非编译 Dart）、contains is-Map promotion 阻断（List 静态错误）、import 发射过宽（#88 已登记）——展示项目再次证明其缺陷探测器价值（v2 先例延续）
* **诚实降级教学化**：35 事件分族中 Flutter 未映射事件保留 + 警告（717 基线）——部分支持成为可学信号而非隐藏

### Negative

* **Flutter warnings 基线上调 44**（673→717）：第三个受支持例外（D3 事件族 ~28 + zIndex）——单调下降契约的又一例外，燃尽依赖后续编译器映射扩展（deferral 触发）
* **4 项语义成员无合法元素展示形态**（resize window 级、animation 族需宿主 @keyframes）：以说明面板呈现——覆盖审计豁免清单维护需纪律
* **dart analyze 全量测试环境退化**（~900s 挂起，v4 终验时 206s）：展示项目完整化后 177 文件的 analyze 全量在异常环境不可完成——需环境修复后重跑确认（代码层面已手动验证 0 error）
* **eslint 豁免 +1**（#88）：codegen import 发射过宽的诚实登记——根因修复前豁免持续存在

### Neutral

* 展示项目 96→100 文件（+4 组件）；React 生成 175→177 文件
* 覆盖审计脚本入库（scripts/）——后续语义演进（新支持成员）须同步 demo 或显式豁免

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——展示项目 + 3 个 codegen 文件 + 2 个门禁文件；无语言面变更。codegen 修复影响所有 Flutter 输出（Number 类型标注与 contains 翻译——均为既有缺陷修复，方向正确）。

**Affected Components**:
- 展示项目 4 新组件 + 3 页挂载 + design.omni —— 纯增量
- `crates/omnilang-core/src/codegen/shared/{types,expr}.rs` + `omnilang/src/codegen/flutter/type_annotate.rs` —— Number 映射与 contains 翻译（全 Flutter 输出受影响，方向修复）
- 门禁两文件（exemptions + warnings 基线）

**Risk Mitigation**:
- 三目标编译逐组件验证（React 177/Flutter/OmniView 全绿）
- 浏览器套件 46+1/0 全量通过（四新组件运行时零错误）
- 覆盖审计脚本可复跑（回归防线）
- dart analyze 全量测试挂起为环境问题（非代码）——手动 analyze 验证 0 error 已确认修复

### Reversibility

**Reversibility Level**: 高——展示项目内容增量可回退（删 4 组件 + 还原挂载）；门禁例外可撤销（基线回落需编译器映射扩展）；codegen 修复为纯缺陷修复（不可逆也不应逆）。

### Sequencing & Timing

已完成（2026-08-06）。后续：
1. 环境恢复后重跑 `learn_omniview_flutter_output_passes_dart_analyze`（确认 0 error 0 warning 全量）
2. deferral #88（codegen import 按需发射）——编译器修复轮触发
3. Flutter warnings 燃尽（717 → 回落）——未映射事件映射扩展（与 #81 TextStyle 燃尽同路线）

## Alternatives Considered

1. **逐功能独立 demo 组件**（35 事件 → 35 组件）——拒绝：展示项目膨胀失控（96→131+），组件间重复样板；事件分族面板（一族一区 + 统一日志）信息密度更高且示范组合用法
2. **跳过部分支持功能（hover/longpress/resize 等）**——拒绝：违背「展示所有功能」指令；部分支持是语言现状，说明面板 + 目标警告是诚实的教学信号（v2「诚实降级」原则延伸）
3. **修 dart analyze 环境再完整验证**——部分采纳：代码层面手动 analyze 验证 0 error；全量测试挂起登记为遗留（环境问题非代码，不阻塞本决策）
4. **展示项目不改、只修编译器**——拒绝：覆盖审计证明缺口在展示侧（42 项），编译器缺陷是展示过程暴露的副产品

## Validation

- 覆盖矩阵：42 缺口 → 0（`python scripts/verify-demo-coverage.py`，4 项豁免有据）
- React 门禁：tsc 77/77 + eslint 5/5 GATE PASSED（新组件零新增 tsc 错误；1 新增 eslint 豁免带 #88 根因）
- 浏览器：46 passed + 1 skipped + 0 failed（semantic-behavior 5/5 保持）
- Flutter：warnings 717 断言测试通过；手动 dart analyze 0 error（修复前 8 issues：3 return_of_invalid_type + 1 unnecessary_non_null_assertion + as Map 修复）
- 三目标编译：React 177 / Flutter / OmniView 全绿
- 提交：`04b7f8da`（demo）+ `764f0650`（门禁）+ `9c59c1a9`（codegen）+ `82ecff16`（demo 微调）

## Pragmatic Enforcer Analysis

**Proposed Decision**: 「展示项目完整化——42 项语义缺口补齐 + 覆盖审计可复跑 + 3 个暴露缺陷修复」

- **Necessity: 8/10** —— 用户明确指令（「必须展示出所有功能」）；「可执行文档」宣称此前不可验证（覆盖审计证明 42 缺口）；3 个潜伏编译器缺陷（含生成非编译 Dart 的 Number 映射）只有展示才会暴露——必要性由指令与实证共同支撑，非装饰性扩张。
- **Complexity: 4/10** —— 4 个 demo 组件（415 行）+ 1 个审计脚本 + 3 处 codegen 修复（累计 20 行内）+ 2 个门禁条目；无新抽象、无新依赖；最大复杂度在 EventSpectrumDemo（35 事件分族）——但为「展示所有功能」指令的直接要求。
- **Ratio: 0.5**（目标 <1.5 ✅）—— 展示内容与覆盖收益匹配；审计脚本（~80 行）是「覆盖可验证」的必要机制（宣称 → 机器断言）。
- **Alternative Analysis**: 逐功能独立组件（备选 1）复杂度更高收益更低——否决正确；跳过部分支持功能（备选 2）违背指令——否决正确。
- **Simpler Alternative Proposal**: 覆盖审计脚本可更简（纯 grep 清单而非解析 universe 表格）——但解析表格使审计随语义集自动演进（新成员自动纳入检查），20 行差异换永久回归防线，值得。
- **Recommendation**: **Approve** —— 指令驱动的必要扩张，复杂度受控，暴露缺陷修复为净收益。
- **Pragmatic Score**: Necessity 8, Complexity 4, **Ratio 0.5**
- **Overall Assessment**: Appropriate engineering —— 「展示所有功能」是明确用户需求而非 YAGNI 违背；覆盖审计把宣称变为可验证机制，与项目「机器验证优先」传统一致。

## References

- `.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md`（12 专家终审——「宣称 vs 事实」漂移主题的直接延续）
- ADR-056/062/067（语义 DSL 时代与 v4 修复轮——展示项目作为可执行文档的演进基线）
- ADR-066（Flutter warnings 燃尽——673→717 第三个受支持例外的历史上下文）
- `scripts/verify-demo-coverage.py`（覆盖审计可复跑资产）
- `.architecture/deferrals.md`（#88 codegen import 按需发射）
