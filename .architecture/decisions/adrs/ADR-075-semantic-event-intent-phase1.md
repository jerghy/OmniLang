# ADR-075: 语义事件意图化阶段 1——首批 14 语义事件 + 一步到位切换（实现完成）

## Status

Implemented

## Context

2026-08-08 设计覆盖评估（`docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md`）确立：语义级 DSL 在机制级意图上成立（@drag/@ref 直写/三目标地道 codegen），但**机制残留半壁**（事件名/CSS/ARIA 透传非理解）与**功能设计级零词汇**（表单/路由/数据流/反馈）是「编译器理解前端功能该怎么设计」的缺口。用户拍板：**方向 3 分阶段全补**——先事件意图化，再功能设计级四域（表单+数据三态/反馈+模态/导航/响应式）。

用户新原则拍板：**语义化即切换，不留旧残留**——构造进入语义层后旧形态不留透传共存，硬诊断 + 迁移提示（capturePointer→@drag 先例），demo 与文档一次性迁移。

阶段 1（本 ADR）实现完成：13 commits（0cfd6acf..762be2b1），9 任务 subagent-driven + 逐任务审查 + 终审（opus）MERGE-READY。

## Decision Drivers

* **机制残留半壁收敛**：54 个 DOM 事件名原样透传、事件字段 DOM 形状（`e.x→clientX`）是「透传非理解」——Flutter 端靠诊断兜底，跨目标可移植性在这半壁失效
* **语义化即切换（用户拍板，spec 原则 8）**：一旦语义化，旧形态硬诊断 + 迁移提示，无新旧双轨期
* **取向 B 增量登记（用户拍板）**：首批收敛跨目标痛点最高 + 阶段 2 前置的事件，其余 40 个透传 + 登记册逐事件标注状态，随语料需求渐进上移
* **既有机制复用**：事件 schema 机制（EventDataKind + event_data_schema + used_fields 提取 + invalid_event_field lint）已存在且字段层已语义化——阶段 1 是「分级 + 收口 + 增强」而非从零建机制
* **YAGNI**：@onPointer* 的 dx/dy（相对上帧位移）移除——@drag 已覆盖增量语义，pointer 原始事件增量需跨事件快照状态、零消费者

## Decision

### 1. 语义事件分级（14 个事件名/9 语义族）

| 语义族 | 事件名 | schema 要点（编译器单一事实源 `ir/events.rs`） |
|---|---|---|
| press | `@onPress` | `{x, y}` 元素坐标（click/press/tap 别名收编） |
| input / change | `@onInput` / `@onChange` | `{value, checked}`（两事件不同语义，非旧名关系） |
| submit | `@onSubmit` | `{values}`（FormData 收集，StringMap） |
| key | `@onKeyDown` / `@onKeyUp` | `{key, code, ctrl, shift, alt, meta}`——key 语义键枚举（19 命名键 + 单字母/单数字，E0006 硬错误 + 近邻建议） |
| pointer | `@onPointerDown/Move/Up` | `{x, y, pointerId, buttons, …}`——**坐标基准 = 元素局部坐标**（D1：offsetX/offsetY / localPosition）；dx/dy YAGNI 移除 |
| scroll | `@onScroll` | `{x, y, progress}`（progress 0..1：scrollTop/(scrollHeight−clientHeight)，不可测 0 兜底） |
| focus | `@onFocus` / `@onBlur` | `{value}`（名不变，仅 schema 化） |
| enter/leave | `@onEnter` / `@onLeave` | `{x, y}`（mouseenter/mouseleave 上移；canonical 别名 enter→mouseenter） |

### 2. 一步到位切换（原则 8 落地）

`@onClick`/`@click`/`@onTap`/`@tap`（canonical 同值 click）→ E0006 硬错误「事件名 "@x" 已弃用——使用 "@onPress"」；`@onMouseEnter`/`@mouseenter`/`@onMouseLeave`/`@mouseleave` → 提示 @onEnter/@onLeave。命中判定 = canonical 同值（裸拼写/大小写变体均命中），`name != s` 守卫防语义名自命中（brief 缺陷，实施修正 + 穷举验证）。`@press` 是运动触发器指令（非事件名）不进事件校验。V5：DOM 形状字段名（clientX/ctrlKey/…）→ 无效字段硬错误 + 迁移提示（「可改用 e.x（元素局部坐标）/e.ctrl」）。

### 3. 关键实现事实

* **map_event_data_kind 接线**（Task 3 fix 1）：@onEnter/@onLeave 必须同时登记 `map_event_data_kind`（PointerEvent 臂）——否则 lowering event_data=None → `emit_event_object` 走 `{raw}.detail` 兜底 → 语义字段三端静默失效（审查捕获的 Critical 计划缺口，闭环）
* **Flutter pointerId 命名差异**：Flutter PointerEvent 无 `pointerId` 属性（SDK `gestures/events.dart:301` 实证只有 `pointer`）——生成 `final pointerId = e.pointer;`（语言面字段名不变，提取源映射 + 注释）
* **@empty 运行时 bug 登记**（pre-existing，b6bff980）：@empty 多子节点只物化 firstElementChild（`<slot>` shadow DOM 外不渲染）→「No notifications」永不出现——HIGH，测试改断言可观测行为 + 根因注释，修复列入阶段 2 候选

### 4. 透传维持（40 个）

`load/error/play/ended/copy/paste/dragstart/…`（54 − 14）维持透传 + Flutter 诊断；semantic-universe 域 3 逐事件标注「语义化/透传」状态（14 + 40 = 54 全分类，脚本双向断言 + parser 常量锚定）。

## Consequences

### Positive

* **跨目标可移植性半壁闭合**：事件层 14 个高频族从「透传」变为「编译器理解 + schema 契约 + 三目标地道映射」；@onInput/@onSubmit/@onKeyDown 是阶段 2（表单/导航）的前置词汇
* **一步到位切换落地**：learn-omniview 零旧名（grep 实证）、Rust 夹具全迁移、登记册/文档/spec 三方同步——无新旧双轨期
* **验证网全绿**：workspace 2000 passed、clippy EXIT=0（5 处预存漂移收口）、verify-react GATE PASSED、Playwright react 46+1 / omniview 44+1、docs-gate 251/0、注入语料 10 passed（新 13 点 + 5 后端 × 7 legacy 切换逐字消息断言）、verify-semantic-universe 0 problems
* **审查捕获的缺陷全部闭环**：Task 2 brief 自命中缺陷、Task 3 Critical 缺口（map_event_data_kind）、Task 9 归属叙述失实——SDD 审查机制实证有效

### Negative

* **破坏性变更历史负担**：@onClick/@onMouseEnter/@onMouseLeave/@onTap 变硬错误——外部 .omni 代码需迁移（迁移提示已内建，docs 同步）
* **docs 同步未全量**：00-cheatsheet/05/06/migration-v1-to-v2 仍教 @onClick 合法（12 处）——spec §7 明示全量同步属阶段 4（docs 重构），已登记
* **V6 未实现**：字段类型检查警告（value 类型与控件匹配）——v0 范围显式 defer（spec §3.5 自标）
* **dioxus 一致性债务**：PointerEvent x/y 仍视口坐标（非默认后端，登记）

### Neutral

* `is_semantic_event` 目前零消费者（计划声明的未来接口——登记册断言用 rust_str_array 直提取，未用该函数）
* @onScroll Flutter 端无端到端 wiring（既有缺口对称延伸，demo 已记录现状）

## Implementation

**13 commits**（0cfd6acf..762be2b1，9 任务 TDD + 修复轮）：

| Task | 内容 | 关键 commit |
|---|---|---|
| 1 | 语义事件分级常量 + 旧名映射 | 533cb677 |
| 2 | 旧名硬诊断 + 迁移提示 | 43f93b38 |
| 3 | @onEnter/@onLeave 别名 + map_event_data_kind 接线 | c3ba52f0/7f01417c |
| — | （Task 2 遗留）omnilang 夹具旧名迁移 | d14f593a |
| 4 | PointerEvent schema +pointerId/buttons + 坐标基准 | 1bd85b59 |
| 5 | KeyEvent key 语义键枚举验证 | 675a07a4 |
| 6 | ScrollEvent +progress | bbe04d64 |
| 7 | Flutter 字段映射补齐 | 9ed05f9d |
| 8 | learn-omniview 事件用点一步到位迁移 | f389d3f1 |
| 9 | 登记册重分类 + 文档 + 注入语料 + clippy 收口 + 终审 Minor | 9bd3bf74/50249fcd/762be2b1 |

## Alternatives Considered

### @onPointer* 保留 dx/dy（相对上帧位移）

**否决（YAGNI）**：@drag 已覆盖增量位移语义（相对按下点）；pointer 原始事件的跨事件增量需编译器生成快照状态（React useRef/全局变量），零消费者。最终 schema `{x, y, pointerId, buttons}`。

### 完整事件集一次收敛（取向 A）

**否决（用户拍板取向 B）**：54 全收敛里一半（媒体/资源事件）真实跨目标需求未被语料证明，一次收敛有过度设计风险；增量登记制 + 「上移一个切换一个」保证契约完整演进。

### 旧名保持透传共存（过渡期）

**否决（用户拍板原则 8）**：透传共存 = 新旧双轨 + 迁移动力归零（@drag 先例：capturePointer 保留硬错误后 demo 才迁移）。硬诊断 + 迁移提示是唯一切换形态。

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 低-中——阶段 1 是「既有 schema 机制上收口」（分级 + 诊断 + schema 增强），非新架构；唯一新概念是语义事件分级与旧名映射（静态表 + 查询函数）；YAGNI 裁剪（dx/dy 移除）与机制复用（used_fields/emit_event_object/canonical 表）结构性降低复杂度。

**Necessity Assessment**: 8/10
- **Current need**: 事件层 54 名透传是「源码表达意图」契约的最后一公里；@onInput/@onSubmit/@onKeyDown 是阶段 2（表单/导航）的硬前置
- **Future need**: 阶段 2 四域全部建立在本阶段事件词汇之上
- **Cost of waiting**: 每个后续域都必须在透传事件上打补丁
- **Evidence of need**: 用户拍板方向 3/取向 B/原则 8；spec 实证三结论

**Complexity Assessment**: 3/10
- **Added complexity**: 语义事件分级表 + 旧名映射 + 一个验证 pass（validate_semantic_keys）+ schema 3 处扩展——全部复用既有机制
- **Maintenance burden**: 登记册与脚本双向断言强制新增事件先登记（ADR-074 纪律延续）
- **Dependencies introduced**: 零

**Alternative Analysis**: 三项备选（dx/dy 保留/取向 A/透传共存）均有明确否决理由（YAGNI/语料未证/双轨退化），用户拍板先行。

**Recommendation**: ✅ Approve

**Pragmatic Score**: Necessity 8, Complexity 3, Ratio 0.375（target <1.5）

**Overall Assessment**: 适当工程——机制收口 + 前置词汇 + YAGNI 裁剪，SDD 审查闭环捕获并修复 3 个真实缺陷（brief 自命中/接线缺口/归属叙述）。

## Validation

- **终审（opus）MERGE-READY**：12 commits 链路闭合（常量→诊断→别名→schema→登记册）、spec §3 逐项有落点、破坏性变更链条无断点、deferred minors 无必须 merge 前修项
- **门禁终态**：workspace 2000 passed / clippy EXIT=0（5 处预存漂移收口）/ verify-react GATE PASSED（tsc 75 cap 余量 2 + eslint 5）/ Playwright react 46+1 · omniview 44+1 / docs-gate 251 blocks 0 failed / verify-semantic-universe 0 problems / 注入语料 10 passed（含 5 后端 × 7 legacy 切换逐字消息断言）
- **零旧名实证**：全仓 .omni `@onClick/@onMouseEnter/@onMouseLeave/@click/@onTap/@tap` grep 零命中；@onChange 判别正确（select/checkbox 保留、文本输入转 @onInput）
- **登记册真锚定**：verify-semantic-universe.py 实跑 0 problems（rust_str_array 提取 parser 常量 + canonical_alias_map 别名表解析 + 14/40 双向断言）
- **审查闭环**：3 个计划缺陷被审查捕获并修复（Task 2 自命中守卫、Task 3 map_event_data_kind 接线、Task 9 clippy 归属叙述）

## References

- spec：`docs/superpowers/specs/2026-08-08-semantic-design-coverage-design.md`（阶段 1 契约 + 原则 8）
- 计划：`docs/superpowers/plans/2026-08-08-semantic-event-intent-phase1.md`（9 任务）
- SDD ledger：`.superpowers/sdd/2026-08-08-semantic-event-intent-phase1/progress.md`（逐任务审查 + deferred minors 汇总）
- 上游：ADR-052/053/054（语义 DSL 时代决策链）、ADR-070/071/073（ref/编译器责任/@drag 意图先例）、ADR-074（语义域登记 + 脚本断言）
- 遗留登记：@empty 运行时 bug（HIGH，阶段 2 候选）；V6 deferral（spec §3.5）；dioxus 一致性债务；阶段 4 docs 全量同步清单（00/05/06/migration）
