# 语法生命周期（Syntax Lifecycle）

**日期**: 2026-08-05
**状态**: Active —— 设计 P6 机制（v3 修复计划 Phase 7，Task 7.4）
**依据**: filter/map/count 教训（v2 域 8）+ v2 设计-验证循环先例（§4.1）+ 设计文档 §2 原则（语义纯度/单名/floor 硬诊断）
**审计**: `scripts/verify-syntax-lifecycle.py`（已批准语法必含「提案 → 试点 → 批准」三段记录 + 试点文件存在性，docs-gate 同批）

> 本制度回答一个问题：**新语法凭什么进 BNF**。
> 新语法进 BNF 前必须：demo 原型 + 预期使用基准 + 收口评审记录——filter/map/count 0 消费者类不再可能（P6 机制）。
> 与登记册（`vocabulary-registry.md`，P1——「词从哪来」）、语义全集（`semantic-universe.md`，P3——「功能有哪些、支持哪些」）互补：登记册管词汇来源，语义全集管功能全集与实现状态，**本制度管时间序**（提案 → spike → 试点 → 评审 → 批准——「新功能怎么进」）。

## 动因（设计 P6——filter/map/count 教训）

v2 域 8 新增语言方法 `filter`/`map`/`count` **直接进 BNF**：learn-omniview 语料（90+ 个 .omni 文件，v2 终验期 93 个）**零消费者**——demo 全部走 stdlib `array_filter`/`array_map`，直到 v3 Task 4.6 prove-or-retire 才迁移 4 处 demo（TodoList/DataTable/FilterBar/array_ext）到语言方法。删除有纪律（已删除语法全部硬诊断 + 迁移提示——floor 外硬诊断原则，`@paused`/auto-bind/`value={sig}` 先例），**新增没有**：「新语法必须带 demo + 使用基准 + 收口评审」的制度不存在——语法可以零消费者进入 BNF，再靠事后的 prove-or-retire（4.6）、floor 门（4.8）追认。

本制度把新增纳入与删除对称的纪律：**先进语料，再进 BNF**。

---

## 四阶段生命周期

```
提案（Proposal）→ spike（原型）→ 语料试点（Corpus Pilot）→ 收口评审（Closure Review）→ BNF 批准
```

### 阶段 1: 提案（Proposal）

新语法的动机记录——回答设计原则 1「编译器因此知道了什么前端功能」：

- **语义意图**：一句话说明前端功能（与平台无关）
- **前置检查（先查再提案）**：
  - 登记册（P1）：候选词汇是否已在册？是 → 用登记册语义名；否 → 是否平台旗标名/合成事件名（借词黑名单）→ 违规
  - 语义全集（P3）：功能是否已登记？语义集扩充（新条目）先在本阶段写进全集，实现状态保持「未支持」
  - 单名（P2）：同一语义是否已有名字（stdlib/方法调用层）？有 → prove-or-retire 论证或收口评审裁决
- **预期使用基准**：预估使用面——哪些 demo/语料/场景会用、预期调用处数量级（≤1 / 1-5 / 5+）
- **产出**：提案记录（动机 + 语义域 + 预期使用基准 + 前置检查结论）——入登记册「提案记录」列

### 阶段 2: Spike（原型）

v2 设计-验证循环先例（设计文档 §4.1）：`设计定稿 → spike 实现（parser + semantic + ≥1 后端 + 测试）→ 门禁 → 浏览器行为验证 → 发现矛盾 → 修订设计 → BNF spec 同步`。

- 原型范围：parser 规则 + 语义校验 + ≥1 目标后端发射 + 单元测试（先于全目标实现）
- 门禁初验：`cargo test` 相关测试绿；原型缺陷/语义矛盾在此阶段暴露（v2 每域 spike 先例：8 触发器/19 属性/keyed @for 等均在 spike 期发现并修订设计）
- **产出**：spike 记录（原型范围 + 测试证据 + 设计修订）——入登记册或设计文档

### 阶段 3: 语料试点（Corpus Pilot）

**≥1 个 demo 真实使用**（learn-omniview demo 或参数化 fixture 语料——`test/projects/learn-omniview/src/**/*.omni` / `omnilang/tests/fixtures/**`）：

- 试点 demo 用真语法写真实场景（不是教学片段）；语料试点即测试——编译/行为证据（门禁/运行断言）随 demo 落地
- 试点失败（demo 抗拒：语义矛盾/跨目标不等价）→ 回退提案修订或撤回（prove-or-retire 的 retire 路径先例，v3 Task 4.6）
- **产出**：语料试点记录（demo 文件路径 + 调用处 + 编译/行为证据）——入登记册「语料试点」列（审计脚本强制引用存在的文件）

### 阶段 4: 收口评审（Closure Review）

- **三目标门禁**：React（`tsc --strict` 0 错误 + eslint react-hooks 0 告警）/ Flutter（`dart analyze` 0 错误）/ OmniView（浏览器套件全测或编译级断言）——v2 终验口径（设计文档 §4.2）
- **使用基准对账**：预估 vs 实际（试点 demo 数量/调用处）
- 评审未通过 → **defer-with-trigger** 入 `.architecture/deferrals.md` 台账（#NN，带触发条件）——推迟不是废弃（台账跟踪约定）
- **产出**：收口评审记录（门禁结果 + 基准对账 + 处置：批准 / 修订重试 / defer / retire）——入登记册「批准记录」列

### BNF 批准

四阶段全部通过后，语法进入 BNF：

1. BNF spec（`2026-07-31-semantic-level-dsl-syntax-spec.md`）与设计文档（`2026-08-02-semantic-syntax-v2-design.md`）同步——文档/BNF/parser 三方一致（v2 先例）
2. 语义全集（P3）状态列更新（未支持 → 支持/部分）+ 登记册（P1）词汇登记
3. floor 矩阵（P5，`omnilang/tests/floor_matrix.rs`）挂机器验证——新增/删除 floor 条目必须先挂机器验证
4. 本登记册「批准记录」列补批准行——审计脚本强制三段记录完整

---

## 批准标准

1. **≥1 demo 使用（语料试点）**——过滤器：0 消费者语法不允许进 BNF；试点 demo 文件存在且被引用（审计脚本检查路径可解析）
2. **三目标门禁**——React（tsc --strict + eslint react-hooks）/ Flutter（dart analyze）/ OmniView（浏览器或编译级断言）全绿
3. **使用基准预估 + 对账**——提案期预估（阶段 1）与试点期实际（阶段 3）在收口评审（阶段 4）对账

---

## 登记册（已批准语法——v2 一次性补档）

> **补档豁免说明**：v2 现有语法按「已批准」补登记（豁免——一次性补档，仅限本表；新语法不适用）。补档方法：**从 7.1 登记册 + 7.2 语义全集 + v2 设计文档 §5 逐域验证清单反向登记**——每行「提案 → 试点 → 批准」三段记录均为 v2 时代既有事实：
> - 提案 = v2 设计文档 §5 逐域设计（compilability 评审 13 专家 + ADR-052/053/054 决策）
> - 试点 = 逐域 demo 迁移（92 文件分批）+ 验证清单勾选项（各域 [x] 项）
> - 批准 = 2026-08-03 终验（learn-omniview 三目标全量绿 + 9 标签页浏览器 0 runtime error + 232 块文档 + 855 注入元组）+ floor 矩阵（`floor_matrix.rs` 25 行全挂机器验证）
>
> 补档表按 v2 设计文档 §5 逐域（10 个编号域 + Lifecycle 域（floor 19）——域 1-10 + Lifecycle）+ 顶层结构；白名单类语法组与 7.1/7.2 的权威白名单一致（19 可动画属性 / filter/map/count / 54 事件名 / 10 RefOp / 13 token 类型）。**注意**：filter/map/count 的 0/93 消费者历史如实保留在动因节——补档批准依据是 v3 Task 4.6 试点补齐（4 处 demo 迁移 + 三目标门禁）+ 4.8 floor 门 + 终验，按豁免登记为已批准。

| # | 语法组 | BNF 事实源 | 提案记录 | 语料试点 | 批准记录 | 使用基准 |
|---|--------|-----------|----------|----------|----------|----------|
| 1 | Ref 系统（`ref name: Element` 声明 + @ref 绑定 + 10 个 RefOp 语义操作 + `bind:value` 唯一双向绑定） | 设计文档 §5 域 1；spec §2（D2 值绑定收敛决策） | v2 设计 §5 域 1（compilability 评审 + ADR-052/053；D2 删除 `value={sig}` 单向绑定形式） | `test/projects/learn-omniview/src/pages/ElementRefDemo.omni`（ref 声明 + @ref 绑定 + 语义操作）、`test/projects/learn-omniview/src/components/ValidationInput.omni`、`test/projects/learn-omniview/src/components/FormPage.omni`、`test/projects/learn-omniview/src/components/Modal.omni`（bind:value）、`omnilang/tests/fixtures/parameterized/refs.omni` | v2 域 1 验证清单勾选（Task 2.2/2.3）+ 终验（Playwright 输入回写实证 T2.3）+ floor 矩阵 ref 1-5 行（`floor_matrix.rs`） | bind:value 覆盖全部表单控件 demo；10 RefOp 三目标（语义全集 域 4 白名单同载） |
| 2 | 动画系统（`@motion` 定义 + 内联体、8 触发器、19 可动画属性、播放参数 loop/count/direction/delay/fill/duration/easing、multi-stop keyframes） | 设计文档 §5 域 2；spec §3（19 属性 = BNF `semantic_anim_prop`） | v2 设计 §5 域 2（ADR-052/053；触发器参数 scroll/drag 维度 + @focus 接线决策） | `test/projects/learn-omniview/src/pages/MotionDemo.omni`、`test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni`、`test/projects/learn-omniview/src/pages/KeyframeAnimationDemo.omni`、`test/projects/learn-omniview/src/pages/EasingFunctionsDemo.omni`、`test/projects/learn-omniview/src/pages/AnimPropertiesDemo.omni`、`test/projects/learn-omniview/src/components/AdvancedPseudoDemo.omni`、`omnilang/tests/fixtures/parameterized/animation.omni`、`omnilang/tests/fixtures/parameterized/motion_semantic_v2.omni`、`omnilang/tests/fixtures/react/motion_enter.omni`、`omnilang/tests/fixtures/react/motion_drag.omni` | v2 域 2 验证清单勾选（Task 2.10 相等守卫/2.9 motion slice/3.4 in-view/focus/5.1 drag 默认 extent）+ 终验（node @scroll 实证 + Playwright @drag 300px T5.1/T7.1）+ floor 矩阵 6-9 行 | 8 触发器 × 多个动画 demo；19 属性三目标（语义全集 域 1 白名单同载） |
| 3 | Design Tokens + 主题（`design {}` 块、13 token 类型、class 两种形式、variant 两种形式、`@themes`/`@theme`/`@default-theme`、动态 token） | 设计文档 §5 域 3；spec §4（13 类型 = BNF `token_type`） | v2 设计 §5 域 3（D1 主题切换决策：@themes 声明 + @theme 信号；ADR-047/049 动态 token） | `test/projects/learn-omniview/src/components/DesignTokenPanel.omni`、`test/projects/learn-omniview/src/pages/DesignTheme.omni`、`test/projects/learn-omniview/src/pages/DynamicTokens.omni`、`test/projects/learn-omniview/src/components/ThemeSwitcher.omni`、`test/projects/learn-omniview/src/components/ThemeSourceDemo.omni`、`test/projects/learn-omniview/src/components/ThemeCssVarsDemo.omni`、`test/projects/learn-omniview/src/components/VariantShowcase.omni`、`omnilang/tests/fixtures/parameterized/design_tokens.omni`、`omnilang/tests/fixtures/themes.omni` | v2 域 3 验证清单勾选（Task 2.7 切片/3.6 multi-theme/4.2 主题选择器/4.7 default-theme 一致性）+ 终验（theme-wiring Playwright T4.2/T7.1）+ floor 矩阵 10-14 行 | 13 token 类型全覆盖；主题切换贯穿多个 demo（ThemeStore/ThemeSwitcher/DesignTheme） |
| 4 | Slots（default + named + fallback；D4 删除类型约束） | 设计文档 §5 域 4；spec §5 | v2 设计 §5 域 4（D4 决策：typed slots 删除——写 `@slot name: Type` → 编译错误 + 提示） | `test/projects/learn-omniview/src/components/SlotDefaultDemo.omni`、`test/projects/learn-omniview/src/components/Modal.omni`、`test/projects/learn-omniview/src/components/TabBar.omni`、`test/projects/learn-omniview/src/components/TabPanel.omni`、`omnilang/tests/fixtures/parameterized/slots.omni`、`omnilang/tests/fixtures/parameterized/slots_body.omni`、`omnilang/tests/fixtures/parameterized/slots_three_forms.omni`、`omnilang/tests/fixtures/parameterized/component_slots.omni` | v2 域 4 验证清单勾选（Task 2.9 slot 三目标对等）+ 终验 + floor 矩阵 15 行 | default/named/fallback 三形态 × 参数化 fixture 三后端 + 3 个组件 demo |
| 5 | Stores（`store {}`：signal/getter/action/effect、`Store.field` 访问、@store 绑定） | 设计文档 §5 域 5；spec §6 | v2 设计 §5 域 5（ADR-052/053；内联 store 硬诊断 v3 2.2） | `test/projects/learn-omniview/src/stores/TodoStore.omni`、`test/projects/learn-omniview/src/stores/ThemeStore.omni`、`test/projects/learn-omniview/src/stores/NotificationStore.omni`、`test/projects/learn-omniview/src/stores/DynamicTokenStore.omni`、`test/projects/learn-omniview/src/components/TodoList.omni`、`test/projects/learn-omniview/src/components/DataTable.omni`、`omnilang/tests/fixtures/parameterized/store.omni`、`omnilang/tests/fixtures/effect_deps.omni` | v2 域 5 验证清单勾选（Task 2.7 切片订阅/2.9 字面量 hoist/2.10 motion slice/2.6 effect deps）+ 终验 + floor 矩阵 18 行 | store 贯穿全部状态化 demo（4 个 store 文件 + 多组件消费）；effect deps 三形态 |
| 6 | Control Flow（`@if` 三元 + transition_mode fade/wait；`@for` key_expr + empty_body + [scroll]） | 设计文档 §5 域 6；spec §7 | v2 设计 §5 域 6（ADR-052/053；mode 过渡决策 + keyed @for） | `omnilang/tests/fixtures/parameterized/control_flow.omni`、`test/projects/learn-omniview/src/components/ForDirectiveKeyDemo.omni`、`test/projects/learn-omniview/src/components/TodoList.omni`（@if 分支 + @for）、`test/projects/learn-omniview/src/components/FeatureList.omni`、`test/projects/learn-omniview/src/components/PageSwitcherDemo.omni` | v2 域 6 验证清单勾选（Task 0.8 参数化 fixture/5.5 @for 契约）+ 终验（Playwright keyed 保状态 T7.1，BUG-1/BUG-3 回归闭环）+ floor 矩阵 16-17 行 | @if/@for 双指令 × 参数化 fixture 三后端 + 多个页面级 demo |
| 7 | Events（54 canonical 事件名 `@onXxx`、FnRef 内联 fn、事件字段映射） | 设计文档 §5 域 7；spec §2.4/§10（54 canonical = `event_names.rs` 单表） | v2 设计 §5 域 7（ADR-052/053；dblclick 类单表复用 + 字段诊断化决策） | `test/projects/learn-omniview/src/components/EventsDemo.omni`、`test/projects/learn-omniview/src/components/EmitEventDemo.omni`、`test/projects/learn-omniview/src/components/EmitKeywordDemo.omni`、`test/projects/learn-omniview/src/components/EventShorthand.omni`、`test/projects/learn-omniview/src/components/FormPage.omni`、`test/projects/learn-omniview/src/components/TabBar.omni` | v2 域 7 验证清单勾选（Task 2.4 单表 + 字段桩全诊断化 + FnRef 回归）+ 终验 + 语义全集 域 3（54 全支持） | 事件绑定遍布全部交互 demo；54 canonical 名三目标（语义全集 域 3 白名单同载） |
| 8 | Expressions & Statements（管道闭包——唯一通用闭包形式、语言方法 filter/map/count、match、block_expr、optional chain、spread、compound assignment、await、`$signal`/`$field` 样式通道） | 设计文档 §5 域 8；spec §9（filter/map/count = `ARRAY_METHODS`） | v2 设计 §5 域 8（D-E prove-or-retire 决策 + 闭包管道唯一性）——**filter/map/count 是本制度动因：0/93 消费者直接进 BNF（v2），试点由 v3 Task 4.6 补** | `test/projects/learn-omniview/src/lib/array_ext.omni`、`test/projects/learn-omniview/src/components/TodoList.omni`、`test/projects/learn-omniview/src/components/DataTable.omni`、`test/projects/learn-omniview/src/components/FilterBar.omni`（4 处 stdlib→语言方法迁移，v3 Task 4.6）、`test/projects/learn-omniview/src/components/OptionalChainDemo.omni`、`test/projects/learn-omniview/src/components/BlockExprDemo.omni`、`test/projects/learn-omniview/src/components/TernaryExprDemo.omni`、`test/projects/learn-omniview/src/components/TypeSystemDemo.omni`、`test/projects/learn-omniview/src/pages/DataFetching.omni`（await）、`omnilang/tests/fixtures/parameterized/array_methods.omni` | v2 域 8 验证清单勾选（Task 1.1 闭包唯一/1.2 数组方法三目标/1.3 $field 通道/1.4 match）+ v3 4.6 试点补齐（4 demo 迁移 + 三目标门禁）+ 4.8 floor 门（E_CODEGEN）+ 终验 + floor 矩阵 20-25 行 | filter/map/count：4 处 demo 调用 + 参数化 fixture（语义全集 域 2 白名单同载）；闭包/match/block/await 各 ≥1 demo |
| 9 | Styling（`class:` 两种形式 + `style: {}` 内联 + 类名字符集校验 + 样式值消毒） | 设计文档 §5 域 9；spec §4.4 | v2 设计 §5 域 9（注入安全硬约束：类名校验 + 转义双保险 + 消毒单源） | 全部 demo（class:/style: 遍布 `test/projects/learn-omniview/src/components/`）、`test/projects/learn-omniview/src/components/StylePseudoDemo.omni`、`test/projects/learn-omniview/src/components/CssVarsDemo.omni`、`test/projects/learn-omniview/src/components/CssUnitsDemo.omni` | v2 域 9 验证清单勾选（Task 3.1 注入语料 + 3.2 消毒三份同源——5 后端 × 9 插入点 × 19 载荷 = 855 元组全绿）+ 终验 | class:/style: 为最高频表面（每个组件 demo 必用） |
| 10 | a11y（`a11y: {}` 块：label/role/hidden/live/alt） | 设计文档 §5 域 10；spec §11 | v2 设计 §5 域 10（评审 MEDIUM-2 + Task 4.3 词汇 ARIA 化——Flutter 旗标名借词教训入登记册 域 2） | `test/projects/learn-omniview/src/components/A11yDemo.omni`、`test/projects/learn-omniview/src/components/FormPage.omni`（8 处 a11y 属性迁移，v2 Task 4.5）、`omnilang/tests/fixtures/parameterized/a11y.omni` | v2 域 10 验证清单勾选（Task 4.3 ARIA 化/4.4 aria-* 删除/4.5 三目标语义树断言 + 不可映射 role warning）+ 终验 + floor 矩阵（a11y 加入 floor 表 Task 4.5） | a11y 块 × 表单/图标 demo；5 语义原语三目标 |
| 11 | Lifecycle（`@mount`/`@unmount`/`@effect` + deps） | 设计文档 §5（域 5/8 联动）；spec §8 | v2 设计（ADR-052/053；effect deps 从 body 分析决策） | `test/projects/learn-omniview/src/components/Timer.omni`、`test/projects/learn-omniview/src/components/NotificationsPanel.omni`、`test/projects/learn-omniview/src/components/StatsDashboard.omni`、`test/projects/learn-omniview/src/components/AddTodo.omni`、`omnilang/tests/fixtures/effect_deps.omni` | v2 验证清单勾选（Task 2.6 cleanup 依赖收集 + _M 提升/2.9 effect deps）+ 终验 + floor 矩阵 19 行 | @effect 贯穿全部副作用 demo；mount/unmount ≥1 demo |
| 12 | 组件与顶层结构（`component`/`import`/`@slot` 声明体/17 属性白名单/bind: 事件缩写等 v2 基础表面） | 设计文档 §5 各域；spec §1/§13 | ADR-052/053（17 属性白名单 + 语义 ref 系统 + 顶层结构） | `test/projects/learn-omniview/src/main.omni`、`test/projects/learn-omniview/src/lib.omni`、`test/projects/learn-omniview/src/lib/api.omni`、`test/projects/learn-omniview/src/lib/transform.omni`（全部 96 个 .omni demo 文件的结构面） | ADR-052/053 + v2 终验（232 块文档全量 + 三目标全量编译 + 9 标签页浏览器） | 每个 .omni 文件均消费组件结构面（使用基准 = 语料全体） |

---

## 新语法申请流程（检查清单）

新增 BNF 生产式时逐项勾选（未勾齐不得进 BNF）：

- [ ] **提案记录**：动机（语义意图 + 语义域）+ 登记册/语义全集/单名前置检查结论 + 预期使用基准
- [ ] **spike 记录**：原型范围（parser + semantic + ≥1 后端 + 测试）与测试证据
- [ ] **语料试点**：≥1 个 demo 真实使用（文件路径 + 调用处 + 编译/行为证据）——0 消费者语法不允许进 BNF
- [ ] **收口评审记录**：三目标门禁结果 + 使用基准对账 + 处置（批准/修订/defer/retire）
- [ ] **BNF 批准**：BNF spec + 设计文档同步；语义全集状态列更新（P3）；登记册词汇登记（P1）；floor 矩阵挂机器验证（P5）；本登记册「批准记录」列补行
- [ ] **审计脚本绿**：`python scripts/verify-syntax-lifecycle.py --check` exit 0

## 与 deferrals 台账的关系

- 收口评审未通过 → defer-with-trigger 入 `.architecture/deferrals.md`（#NN，带触发条件；台账跟踪约定：「推迟不是废弃」）
- 已登记语法删除（零使用量 + 硬诊断，对称纪律）——删除后从本登记册移除并标注原因
- 本制度的模板引用已登记于 `.architecture/deferrals.md`（2026-08-05 状态更新）

## References

- `docs/superpowers/plans/2026-08-03-semantic-syntax-v3-fixes.md`（Task 7.4；设计 P6——v2 域 8 filter/map/count 教训）
- `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（v2 设计：§4 验证协议、§5 逐域 BNF 与验证清单——补档证据源）
- `docs/superpowers/specs/semantic-universe.md`（设计 P3——语义集 vs 实现状态；补档反向登记源之一）
- `docs/superpowers/specs/vocabulary-registry.md`（设计 P1——词汇来源与借词黑名单；补档反向登记源之一）
- `.architecture/deferrals.md`（收口评审未通过的下游；模板引用登记）
- `omnilang/tests/floor_matrix.rs`（设计 P5——floor 条目机器验证）