# 批次 3 响应式 + 样式修复 设计 spec（2026-08-09-semantic-batch3-responsive-styling-design.md）

**日期**: 2026-08-09
**状态**: 详细级设计草稿（待 writing-plans 生成实施计划）
**上游**: `2026-08-09-semantic-capability-baseline-design.md` §4 批次 3 + §8 治理；车票 `08-stage-scope-responsive-breakpoints.md`、`10-stage-scope-styling-selectors.md`；研究 `02-feedback-and-style.md` 样式簇
**配套计划路径**: `.superpowers/sdd/semantic-batch-specs-plans/`（writing-plans 产出批次 3 计划）

---

## 1. 背景与范围

本批次合并两个已裁决域（响应式断点 + 样式选择器缺陷修复）为一个实施批次，另含 @media 映射策略定案与容器查询候选记录。能力项清单与上游裁决引用：

| # | 能力项 | 上游裁决（基线 §4/§2 + 车票） | 本批次处置 |
|---|--------|-------------------------------|-----------|
| 1 | `bp("md")` 逻辑断点条件 | 🔜 进语言——design 域 `breakpoint md: 768` token + `@if bp("md")` 条件表达式 → React useMediaQuery/matchMedia / Flutter MediaQuery / OmniView matchMedia（基线 §4；车票 08） | 定稿语法形态 + BNF + Validation + 三目标映射 |
| 2 | breakpoint token | 🔜 进语言——design 域扩展，编译期 token 表 → 各目标断点常量（车票 08） | 定稿 14 类设计声明形态（不扩 token_type 名单） |
| 3 | 选择器静默跳过零诊断（真实缺陷，`parser/view/style.rs` ~:166） | 🔨 缺陷修复定案：诊断先行——未支持选择器部分（兄弟/属性/:nth-child 等）发 **warning 硬诊断**（含选择器文本 + 截断说明），诚实降级（ADR-054），绝不静默；支持补齐随语料需求再议（车票 10） | 定稿诊断行为 + 机制（parser 无 warning 通道实证 → 设计选型） |
| 4 | 用户级 @media | 🏞 机制语法面——CSS 透传（web 通道）+ Flutter 映射策略（MediaQuery 分支）批次 spec 细化（基线 §4；车票 08「bp() 是跨目标主通道」） | 定稿语法形态 + 映射策略（降级粒度列入未决问题 §9） |
| 5 | 容器查询 | 🔜 进语言候选（低优先）——web @container / Flutter LayoutBuilder，形态批次 spec 细化（车票 08） | 记录形态草案 + 触发条件（本批次不实施） |
| 6 | 深色模式 / 样式作用域 | ✅ 已支持（@themes + 组件级作用域）——无动作（车票 10） | 仅登记册/文档一致性注记 |
| 7 | 复杂选择器语法面 | 🏞 机制语法面（CSS 透传）；Flutter 端组件级样式（表达通道不同，非功能缺失）——注记（车票 10） | 域 14「留生态」登记行 |

---

## 2. 语义意图定义

### 2.1 `breakpoint` 断点声明（design 域扩展）

**意图一句话**: 「给一组命名断点数值，供条件表达式引用，实现跨目标响应式分支」。

**语法形态**（详细级定稿）——design 块内新增专用声明（**不改动 `token_type` 13 名单**，理由见 §2.3 冲突检查）：

```
design LearnDesign {
    @themes { light, dark }
    @default-theme light;

    breakpoint sm: 640;
    breakpoint md: 768;
    breakpoint lg: 1024;

    token primary: color = "#6366f1" dark "#a5b4fc";
    // ...既有 token/class/variant 不变
}
```

- 声明关键字 `breakpoint`，形式 `breakpoint <名>: <无单位数字>;`——与上游所有文档字面（`breakpoint md: 768`）一致
- 值 = 无单位数字，语义为 CSS px（`min-width` 语义；与 Tailwind `theme.screens` 断点体系同构）。Flutter 端逻辑像素与 px 1:1，无换算
- **无主题覆盖**（与既有非 color token 一致——`10-design-system.md` 约定）：断点不随主题变化，不允许 theme 值
- 断点名命名空间 = design 块内标识符空间，与同块 token/class/variant **互斥**（重名 = 硬错误 E-402，见 §3）
- 编译期静态表：breakpoint 声明产出「断点常量表」（name → number），供 `bp()` 编译期查表

**BNF 产生式草案**（`design_block` 现有产生式基础上追加）：

```
design_stmt      ::= themes_decl | default_theme_decl | token_decl
                   | breakpoint_decl | class_decl | variant_decl
breakpoint_decl  ::= "breakpoint" ident ":" NUMBER ";"
```

与既有语法的关系/冲突检查：

| 检查项 | 结果 |
|--------|------|
| 与 `token_type` 13 名单（color/size/.../zIndex） | 不冲突——breakpoint 是**独立声明关键字**，不进入 token_type 名单。理由：token_type 是语义集锁定面（域 5 白名单 + verify 脚本双向断言）；breakpoint 语义（离散命名数值、无主题值、无单位、编译期查表）与 theme token 正交，扩名单会牵动全量断言且语义混叠。登记册 域 12 新域承载（§5） |
| 与 `@themes` / `@default-theme` | 不冲突——并行声明；断点无主题维度 |
| 与 token 名重名 | **E-402 硬错误**（design 块内标识符互斥，消息见 §3） |
| 与 CSS 属性名 `breakpoint`（不存在） | 无冲突 |

### 2.2 `bp("name")` 断点条件表达式

**意图一句话**: 「当前视口宽度 ≥ 命名断点值时条件成立」。

**语法形态**（详细级定稿）——函数调用形式，布尔表达式：

```
component TodoList {
    view {
        @if bp("md") {
            <div class: DesktopLayout> … </div>
        }
        @if {!bp("md")} {
            <div class: MobileLayout> … </div>
        }
        <div class: {bp("md") ? "desktop-grid" : "mobile-list"}> … </div>
    }
}
```

- 参数必须是**字符串字面量**（编译期静态查表）；动态表达式参数 = 硬错误 E-405
- 语义 = `min-width`（宽度 ≥ 断点值），与 CSS `@media (min-width: 768px)` 对齐
- 使用面：任何布尔表达式位置——`@if` 条件、`{}` 表达式绑定、三元运算（与既有表达式系统正交，无新指令）
- 引用未声明断点名 = 硬错误 E-401（照 `2026-08-08-semantic-design-coverage-design.md` §4.4「未声明引用 → 硬错误 + 建议」）

**BNF 产生式草案**（表达式 grammar 的 primary/调用位置追加）：

```
bp_call   ::= "bp" "(" STRING_LITERAL ")"
```

与既有语法的关系/冲突检查：`bp` 非既有保留名/事件名/指令名；`bp(...)` 是表达式层函数调用形态，与 `token(...)` 取值调用、`format(...)` 等 stdlib 形态同族（语义函数先例）；无词法白名单常量——登记册按**机制级条目**处理（存在性断言，域 8/9 先例，§5）。

### 2.3 选择器静默跳过缺陷修复（warning 硬诊断）

**意图一句话**: 「样式规则选择器含暂不支持部分时，绝不静默截断——发 warning 诊断（含选择器文本 + 截断说明）」。

**缺陷实证**（`crates/omnilang-core/src/parser/view/style.rs` ~:166-174）：

```rust
// Skip unsupported remaining selector parts (e.g., "input:checked + span")
// until we find the opening brace
while *self.peek_kind() != TokenKind::LBrace
    && *self.peek_kind() != TokenKind::RBrace
    && *self.peek_kind() != TokenKind::Eof
{
    self.advance();
}
self.expect_token(TokenKind::LBrace, "expected '{'")?;
```

选择器解析链（已核实）：`.` 消费 → class ident → 连字符名 → 伪类 loop（`:hover` 等单冒号 + ident）→ compound loop（仅当 span 相接 `.a.b`）→ descendant loop（span 空隙 `.a .b`）→ **静默跳过（缺陷）** → `expect_token(LBrace)`。触发静默跳过的未支持部分类别：兄弟 `+`/`~`、子 `>`、属性 `[attr]`、带参伪类 `:nth-child(...)`/`:not(...)`、伪元素 `::before`、`:has()`、选择器列表 `,`。

**诊断行为定稿**：

| 项 | 定稿 |
|----|------|
| 严重级别 | warning（构建不失败——诚实降级非阻断；ADR-054「诚实降级 > 静默错误」） |
| 消息内容 | 选择器**完整原文** + 截断点 + 未支持部分类别 + 影响说明（web 透传/Flutter 不生效）+ 注册表指引 |
| 触发条件 | 每次解析到静默跳过处即发（含 text span 定位到规则行/列） |

**机制选型（实证支撑）**: parser 层当前**无 warning 发射通道**（`crates/omnilang-core/src/parser/` 全域 grep `warnings|emit_warning` 零命中）；warning 通道现存在于语义/生成层（`analyse.rs` `AnalyseOutput.warnings: Vec<Diagnostic>`、`codegen/writer.rs` `CodegenWarning`、`lib.rs` `CompileOutput.warnings` → CLI stderr，ADR-074 B2 管线，不使构建失败）。**推荐机制**：parser 在静默跳过处把「规则起始 token 到跳过起点」的 span 记录到 `StyleRule` AST 新字段（如 `unsupported_selector: Option<Span>`），由语义层新增验证 pass（`analyse.rs` IR 管线第 8 pass，`validate_style_selectors`）读取 span 切片源码发射 `Severity::Warning` 诊断 → 既有 warnings 管线直通 CLI。备选机制（parser 层新增 warnings 槽）改动面大（Parser 公共 API 全链路），不推荐；两者取舍列入 §9 未决问题 2。

**BNF 影响**: 无——选择器语法面不变（支持集合不变，仅补诊断）。

### 2.4 用户级 @media 块（web 透传通道）

**意图一句话**: 「组件 style 块内书写 CSS 媒体查询样式；web 目标原样透传，Flutter 目标诚实降级」。

**语法形态**（详细级定稿）——组件 style 块内 at-rule 块：

```
style {
    .card { padding: 16; }
    @media (min-width: 768px) {
        .card { padding: 32; }
    }
    @media (prefers-color-scheme: dark) {
        .card { background: #111827; }
    }
}
```

- 媒体条件文本 = **透传不解析**（parser 从 `(` 起按括号配对消费到块起始 `{`，原文保存——机制语法面，编译器不解析条件内部结构）
- 块内规则 = 既有 style rule 语法（class/tag/伪类/descendant/compound 同现行）
- web 目标：`@media (...) { ... }` 原样输出 CSS（透传通道）
- **Flutter 目标：warning W-302 + 块内规则跳过（不生效）**，指引「响应式意图用 `bp()` 表达（跨目标主通道）」。MediaQuery 分支映射（LayoutBuilder 包裹 + 条件样式覆盖）首版成本高、收益低，列为增强候选（触发条件见 §6）。**降级粒度取舍列入 §9 未决问题 1**（本 spec 推荐诚实降级，不擅自推翻基线「映射策略细化」的两种解读）
- 与 design 域 `@themes` 生成的 `@media (prefers-color-scheme: dark)`（3-layer CSS 生成物）**无冲突**——生成物在 CSS 输出层，用户级 @media 在组件 style 块语法层，命名空间分离；深色模式意图仍走 @themes（✅已支持，车票 10 无动作）

**BNF 产生式草案**：

```
style_block    ::= "style" "{" style_rule* media_block* "}"
media_block    ::= "@media" media_condition "{" style_rule* "}"
media_condition::= RAW  // 括号配对消费原文保存（透传不解析）
```

### 2.5 容器查询（低优先候选记录，本批次不实施）

- **形态草案**（候选，随语料需求再议）：style 块内 `@container (min-width: 400px) { .card { … } }` → web `@container` 透传 / Flutter `LayoutBuilder` 映射候选
- 现有 `container-type: inline-size`（design.omni 已用）是**容器声明侧**（CSS 属性透传已支持）；@container 是**查询侧**——本批次不实施查询块
- 本批次遇 `@container` 字面 → **硬错误 E-404**（未实施功能明确拒绝 + 指引，绝不静默；「语义化即切换」与 ADR-054 一致）
- 触发条件（§6）：组件级（非视口级）自适应样式语料出现

### 2.6 命名冲突表（新词汇 × 既有词汇全表）

| 新词汇/形态 | 既有词汇 | 冲突？ | 处置 |
|-------------|----------|--------|------|
| `breakpoint` | design 域 13 token_type 名 | 无 | 独立声明关键字（§2.1） |
| `breakpoint` | `token`/`class`/`variant` 声明名 | 名空间共享 | E-402 互斥硬错误 |
| `breakpoint` | 元素指令（@in-view/@drag/@form/@load/@modal…）、事件名（@onPress…） | 无 | 不同形态面 |
| `bp` | 保留名/事件名/Flutter 旗标名/React 合成事件名 | 无 | 表达式函数形态（§2.2） |
| `@media` | `@themes`（生成物 @media） | 无 | 语法层 vs 输出层分离（§2.4） |
| `@container` | `container-type` 属性 | 无 | 查询侧 vs 声明侧（§2.5） |
| `@media`/`@container` | kebab 指令族（@in-view 等） | 无 | at-rule 形态不同（冒号后接条件文本，非指令参数） |

---

## 3. Validation 清单

### 硬错误（解析/语义）

| # | 消息草案（中文） | 触发 |
|---|------------------|------|
| E-401 | `bp("md") 引用了未声明的断点名 md——请在 design 块声明：breakpoint md: <数值>;（建议 768）` | `bp()` 字符串参数不在断点常量表 |
| E-402 | `design 块中 breakpoint 名 md 与 token/class/variant 名冲突——breakpoint 名须与同块标识符互斥` | 同块重名 |
| E-403 | `breakpoint md 的值必须为正数（当前 0）` | 值 ≤ 0 |
| E-404 | `不支持的 at-rule @container——容器查询为低优先候选（语义级未实施），响应式意图请用 bp() 或 @media 表达` | style 块内遇 @container |
| E-405 | `bp() 的参数必须是字符串字面量（编译期静态查表），当前为动态表达式` | 非字面量参数 |

### 警告（诚实降级，构建不失败）

| # | 消息草案（中文） | 触发 |
|---|------------------|------|
| W-301 | `样式规则选择器「input:checked + span」包含暂不支持的语法部分（兄弟选择器），已截断到「input:checked」继续解析——该部分 web 端按 CSS 透传处理、Flutter 端不生效；复杂选择器为留生态（semantic-universe 域 14 注记），支持随语料需求再议` | style.rs 静默跳过触发（类别词按实际：兄弟 `+`/`~`、子 `>`、属性 `[attr]`、带参伪类、伪元素、选择器列表） |
| W-302 | `@media 块在 Flutter 目标不生效（降级跳过）——响应式意图请用 bp() 表达（跨目标主通道）` | Flutter 目标 codegen 遇 @media 块 |

### 语义化即切换检查（ADR-075 原则 8）

本批次**无旧形态残留**：bp()/breakpoint/@media 均为全新语法面，无旧等价形态需硬诊断切换；@media 透传行为是新增能力（此前 parser 不支持用户级 @media，遇之报错），无双轨期。

---

## 4. 三目标映射表

### 4.1 `breakpoint` 声明 → 各目标断点常量

| 目标 | 生成形态 | 地道性依据 | 降级注记 |
|------|----------|-----------|----------|
| React | 编译期断点常量表（`const BP = { sm: 640, md: 768, lg: 1024 }`），随组件模块输出，供 useMediaQuery 引用 | Tailwind `theme.screens` 同构——断点即命名常量表，事实标准 | 无（纯编译期静态数据） |
| Flutter | 断点常量（`static const bpSm = 640.0; ...`），供 MediaQuery 分支引用 | Flutter 无常量体系惯例，static const 为标准做法；与 Tailwind 语义一致 | 无 |
| OmniView | 编译期常量表注入运行时（matchMedia 调用参数） | OmniView 运行时 JS 直通 | 无 |

### 4.2 `bp("md")` 条件表达式 → 各目标

| 目标 | 生成形态 | 地道性依据 | 降级注记 |
|------|----------|-----------|----------|
| React | `useMediaQuery('(min-width: 768px)')`（响应式 hook；`@if bp("md")` → 组件顶层 hook 调用 + 条件渲染；`{}` 绑定/三元同理） | React 官方推荐响应式写法——`useMediaQuery`（react-use / MUI 事实标准，底层 matchMedia） | 无；SSR 场景（无 matchMedia）以视口缺省分支渲染，注记 |
| Flutter | `MediaQuery.sizeOf(context).width >= 768`（构建期布尔；`@if bp("md")` → 条件 Widget 分支；`{}` 绑定同理） | Flutter 官方响应式惯用法——MediaQuery.sizeOf | 无 |
| OmniView | `matchMedia('(min-width: 768px)').matches`（表达式求值） | 浏览器原生 API 直通 | 无 |

### 4.3 用户级 @media 块 → 各目标

| 目标 | 生成形态 | 地道性依据 | 降级注记 |
|------|----------|-----------|----------|
| React | CSS 输出层 `@media (...) { ... }` 原样透传（3-layer CSS 生成物同级） | CSS 媒体查询是 web 原生表达通道（机制语法面） | 无 |
| Flutter | warning W-302 + 块内规则跳过（不生效）；指引 bp() | — | **诚实降级**（ADR-054）；MediaQuery 分支映射为增强候选（§9 未决问题 1） |
| OmniView | CSS 透传同 web（OmniView 运行时 CSS 直通） | 同 web | 无 |

### 4.4 选择器诊断（无生成形态——纯诊断）

| 目标 | 行为 | 降级注记 |
|------|------|----------|
| 全部 | CLI stderr 输出 W-301 warning（构建不失败），含选择器原文/截断点/类别/指引 | 既有 warnings 管线（ADR-074 B2）直通 |

---

## 5. 登记册扩展

### 5.1 semantic-universe 域 12「响应式」新域（登记先行，ADR-074）

**语义域定义**: 视口级响应式能力——命名断点声明 + 断点条件表达式 + 媒体查询透传。

**语义集条目表**（域 12 草案）：

| 条目 | 语义 | 实现状态 | 诊断行为 |
|------|------|----------|----------|
| breakpoint 声明 | 命名断点数值表（design 块） | 支持（本批次） | 值 ≤ 0 → E-403；同块重名 → E-402 |
| bp() 条件 | 视口宽度 ≥ 断点值 | 支持（本批次） | 未声明引用 → E-401；非字面量参数 → E-405 |
| @media 块 | 媒体查询样式（web 透传） | 部分（web 生效 / Flutter 降级 W-302） | Flutter 目标 → W-302 |
| @container 块 | 容器查询（查询侧） | 未支持（低优先候选，触发条件见 §6） | 遇之 → E-404 硬错误 + 指引 |
| 深色模式 | @themes → prefers-color-scheme 生成物 | 支持（既有，无动作） | — |

**已实现白名单表**（机制级条目——存在性断言，域 8/9 先例：bp 为函数调用形态、无词法白名单常量）：

| 条目 | 编译器事实源锚点 |
|------|------------------|
| breakpoint 声明 | parser design 块 breakpoint_decl 识别；语义层断点常量表产出 |
| bp() 条件 | 语义层 bp_call 识别 + 断点常量表查表；codegen 三目标锚点（React useMediaQuery / Flutter MediaQuery.sizeOf / OmniView matchMedia） |
| @media 块 | parser media_block 识别 + web codegen 透传输出 |
| 选择器诊断 | analyse.rs validate_style_selectors pass 发射 W-301 |

### 5.2 域 14 样式「留生态」登记行（状态值域新增，基线 §8 治理）

| 条目 | 语义 | 实现状态 | 语言边界注记（触发条件 + 目标生态映射指引） |
|------|------|----------|---------------------------------------------|
| 复杂选择器（兄弟/子/属性/带参伪类/伪元素/选择器列表） | 样式选择器表达 | 留生态 | 机制语法面（CSS 透传通道）；Flutter 以组件级样式表达同等意图（表达通道差异，非功能缺失）；诊断先行已落地（W-301）；支持补齐触发条件 = 语料需求 |
| @container 查询块 | 容器级自适应 | 留生态（候选） | 低优先候选；web @container / Flutter LayoutBuilder；触发条件 = 组件级自适应样式语料出现 |
| @media 块 | 视口级媒体查询 | 部分（域 12 同载） | web 透传生效；Flutter 降级（W-302）→ 增强候选（§9 未决问题 1） |

### 5.3 vocabulary-registry 新词汇（四字段）

| 语义词汇 | 来源域 | 目标映射 | 排除词汇 |
|----------|--------|----------|----------|
| bp | 12 响应式 | React useMediaQuery / Flutter MediaQuery / OmniView matchMedia | `useMediaQuery`（React 借词禁——语义词汇不得为合成事件名/旗标名/库 API 名） |
| breakpoint | 12 响应式 | 断点常量表 → 各目标常量 | — |

> `@media` / `@container` 为机制透传关键字（CSS 原生词汇），**不登记**为语义词汇（域 7 at-rule 形态面；语义集条目见 §5.1）。

### 5.4 verify 脚本锚点方案

- `verify-semantic-universe.py`：域 12 新增条目双向断言（声明/使用/诊断各锚点存在性）；域 14 留生态行状态值域校验（新增「留生态」状态值）
- `verify-vocabulary.py`：bp/breakpoint 两行四字段完整性 + 排除词汇禁借断言
- 注入语料（855 元组扩展）锚点见 §7

---

## 6. 语言边界与触发条件

本批次**未进语言项**记录（留生态/后议，语义化即切换纪律——绝不静默，均有明确诊断或注记）：

| 项 | 边界类别 | 现状处置 | 触发条件（语料/环境变化时重评） |
|----|----------|----------|--------------------------------|
| 复杂选择器（兄弟/子/属性/带参伪类/伪元素/选择器列表） | 🏞 机制语法面（CSS 透传） | W-301 诊断 + web 透传（诊断先行） | 语料中复杂选择器需求出现（如第三方设计系统移植） |
| @media 块 Flutter 映射（MediaQuery 分支） | 🏞 编译器职责候选 | 首版 W-302 诚实降级（§9 未决问题 1 待拍板） | 移动端响应式样式语料出现；首版后复盘收益 |
| 容器查询 @container | ⏳ 低优先候选 | E-404 硬错误 + 指引 | 组件级（非视口级）自适应样式语料出现 |
| 深色模式 / 样式作用域 | ✅ 已支持 | 无动作（登记册注记） | — |

---

## 7. 门禁与验证（五层门禁逐项）

| 门禁 | 本批次断言 |
|------|-----------|
| tsc | React 生成物编译通过：useMediaQuery hook 调用 + 条件渲染类型正确（含 bp() 三态分支组件） |
| dart analyze | Flutter 生成物零告警：MediaQuery.sizeOf 分支 + W-302 降级路径（无 @media 残留语法） |
| Playwright | 浏览器行为断言（清单见下）+ CLI warning 文本断言（stderr 级） |
| docs-gate | semantic-universe 域 12 + 域 14 留生态行 + vocabulary-registry 两行 + 10-design-system.md breakpoint 说明 + 09-styling.md @media 注记，verify 双脚本绿 |
| 注入语料 | 855 元组扩展：① breakpoint 声明（sm/md/lg）② bp() 在 @if/{} 绑定/三元三位置 ③ @media 块（min-width + prefers-color-scheme）④ 未支持选择器（`input:checked + span`）→ W-301 ⑤ bp 未声明引用 → E-401 ⑥ @container → E-404 ⑦ Flutter 目标 @media → W-302 |

**浏览器行为断言清单**（learn-omniview ResponsiveDemo）：

1. 视口宽 ≥768（如 1024）：DesktopLayout 分支可见；`bp("md") ? "desktop-grid" : "mobile-list"` 绑定为 desktop-grid
2. 视口宽 <768（如 500，resize 至 767/768 边界各一次）：MobileLayout 分支可见；绑定为 mobile-list
3. 边界精度：767px 走 mobile、768px 走 desktop（min-width 语义，无 off-by-one）
4. @media 块规则在 web 生效（如 768px 以上 .card padding 变化）
5. CLI 编译含 W-301 的 .omni：退出码 0（构建不失败）+ stderr 含选择器原文与类别词

---

## 8. demo 迁移清单（learn-omniview）

| 位置 | 动作 |
|------|------|
| `src/design.omni` | 新增 `breakpoint sm: 640; breakpoint md: 768; breakpoint lg: 1024;` 三行声明 |
| 新组件 `src/components/ResponsiveDemo.omni` | @if bp("md") 双分支布局 + `{}` 三元绑定 + @media 块（min-width 768px 样式增强），演示三使用面 |
| `src/pages/`（UiComponents 页或新建） | ResponsiveDemo 接线（照 LoadDemo 先例：页面组件 + 注册入口） |
| `src/components/ThemeSourceDemo.omni` | 注记：展示文本中 `@media (prefers-color-scheme: dark)` 为**主题生成物**，用户级 @media 为独立语法面（区别说明） |
| `src/main.omni` | 页面入口 wiring |

---

## 9. 未决问题（需用户/主 agent 拍板）

1. **@media 块 Flutter 映射粒度**：基线「Flutter 映射策略（MediaQuery 分支）批次 spec 细化」存在两种解读——(a) 本 spec 推荐：首版**诚实降级**（W-302 + 跳过 + 指引 bp()，跨目标主通道），MediaQuery 分支映射（LayoutBuilder 包裹 + 条件样式覆盖）列为增强候选；(b) 首版即做 MediaQuery 分支映射（成本高：style 条件求值 + 动态样式覆盖，收益低：bp() 已覆盖响应式意图）。**推荐 (a)**，待拍板。
2. **选择器诊断机制选型**：推荐 = parser 记录 span 到 `StyleRule.unsupported_selector`（AST 新字段）+ analyse.rs 第 8 pass 发射 warning（不动 Parser 公共 API，warnings 管线零改动）；备选 = parser 层新增 warnings 槽（改动面大）。**推荐前者**，待拍板。
3. **breakpoint 值域上限**：是否设最大断点值（如 ≤ 10000）防笔误——本 spec 暂只限正数（E-403），上限留实施期定。

