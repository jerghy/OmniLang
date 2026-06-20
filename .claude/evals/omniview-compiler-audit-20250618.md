# 🔍 OmniView 编译器安全与质量审计报告

**日期**: 2026-06-18 | **范围**: `omni-compiler/src/` — ~22,689 行 Rust，29 源文件 | **基线**: `cargo test` 380 passed

**审计阵容**: 并行五代理 + 直接代码审计
- 🔐 security-reviewer: 注入/XSS/路径遍历
- 🦀 rust-reviewer: unsafe/unwrap/panic/死代码
- 🔇 silent-failure-hunter: 错误吞没/静默回退
- 📐 code-reviewer (codegen): 代码生成正确性/转义
- 🔬 code-reviewer (parser/lexer): 解析器/词法分析器健壮性
- 👁️ 直接审计: 手动审查全部 29 源文件

---

## 📊 漏洞总数: 60

| 级别 | 数量 | 主要分布 |
|------|------|---------|
| 🔴 CRITICAL | 8 | CSS注入、运行时名称冲突、@for模板eval、Dart/Dioxus语法错误、7-target ExprIR丢失、JS代码注入、**全局字符串`<`/`>`截断** |
| 🟠 HIGH | 14 | 递归限制、输入限制、不安全注释、名称污染、Vue v-if注入、HTML转义、关键字冲突、**整数溢出→0**、**Unicode转义数据损坏** |
| 🟡 MEDIUM | 24 | 错误吞没、不一致性、静默回退、死代码、性能、**无错误恢复**、**未闭合注释/字符串无诊断** |
| 🟢 LOW | 14 | 文档、测试覆盖、效率、可维护性 |

---

## 🔴 CRITICAL (7)

### CVE-1: CSS 值注入 — 无输出净化
- **文件**: `codegen/omniview.rs:1341-1351`, `react.rs:733`, `vue.rs:764`
- **描述**: CSS 值从 .omni 源逐字传递。可注入 `url("javascript:alert(document.cookie)")`。
- **利用**: 编写恶意 .omni → 编译加载 → JS 在浏览器中执行。
- **修复**: 阻止 CSS 中的 `javascript:`/`data:` URL；如发现则报错。

### CVE-2: 运行时名称冲突 — 用户代码覆盖 `_signal`/`_effect`/`_computed`
- **文件**: `codegen/omniview.rs:145-232`
- **描述**: 运行时函数为全局。Store 动作和模块级函数使用相同名称会覆盖运行时。
- **修复**: 使用 `__omni_` 前缀；在代码生成前验证名称不冲突。

### CVE-3: @for 模板插值 — 属性查找错误导致所有复合表达式静默失败
- **文件**: `codegen/omniview.rs:866-872`
- **严重性**: 所有 `{item.field}` 表达式在 @for 循环中静默显示 `undefined`。
- **根本原因**: `({ item })["item.name"]` 查找键 `"item.name"` 而非路径 `item.name`。
- **修复**: 使用 `new Function(item_var, "return " + expr)(item_var)` 或等效方案。

### CVE-4: 7/10 codegen 目标静默丢弃 MethodCallExpr/ArrayExpr/StructExpr/BlockExpr
- **文件**: `dart.rs:385`, `javascript.rs:420`, `rust.rs:402`, `dioxus.rs:594`, `react.rs:661`, `vue.rs:691`, `flutter.rs:638`
- **描述**: `_ => "null"` 通配符将数组、结构体、方法调用降级为 `null`。
- **修复**: 为每个缺失 variant 添加显式处理。

### CVE-5: Dart/Flutter 参数语法错误 (`name: Type` 应为 `Type name`)
- **文件**: `dart.rs:208-214,549-553`, `flutter.rs:558-562`
- **描述**: 生成 `int add(a: int)` — 无效的 Dart 语法。
- **修复**: 交换类型和名称顺序。

### CVE-6: Dioxus 空元素发射杂散 `{} div` 行
- **文件**: `dioxus.rs:320-327`
- **描述**: 调试遗留导致 `rsx!` 宏中产生文字 `{} div` 语法错误。
- **修复**: 删除第 324 行。

### CVE-7: ViewTree::Text `{...}` 回退 — JS 代码注入
- **文件**: `codegen/omniview.rs:1107-1123`
- **描述**: `element_to_html` 回退路径将原始文本注入 `_effect()` 回调体。如果 `ViewTree::Text("{evil_code()}")` 到达此处 → 完整的 JS 代码注入。
- **修复**: 移除此回退（`Interpolation` 节点已正确处理），或添加 `debug_assert!` 并转义。

### CVE-8: `<` 和 `>` 字符在所有字符串字面量中全局截断 — 静默数据丢失
- **文件**: `lexer/mod.rs:380-383, 406-409`
- **描述**: `read_string` 和 `read_single_quoted_string` 在遇到 `<` 或 `>` 字符时无条件中断，在**所有上下文中**（不限于视图模板）。例如 `let msg = "value < threshold";` 被词法分析为 `StringLiteral("value ")`，其后所有内容丢失。`<` 字符被作为单独的 `Lt` token 处理，导致级联解析错误。代码注释说这是"视图文本中的 HTML 边界"，但词法分析器在所有上下文中全局应用此规则。
- **利用场景**: 任何使用 `<` 或 `>` 比较运算符的 .omni 字符串字面量都会导致静默数据损坏。
- **修复**: 从通用字符串读取器中移除 `<`/`>` 截断。如果需要视图文本边界检测，应将其移到视图解析器中（视图解析器已有用于收集元素间文本的 `is_text_token` 逻辑）。

---

## 🟠 HIGH (12)

### H-1: 递归下降解析器无深度限制 → 栈溢出
- **文件**: `parser/expr.rs:40-52`, `parser/view.rs`
- **修复**: 最大深度计数器（256），超限返回 `ParseError`。

### H-2: 词法分析器无输入大小限制 → OOM
- **文件**: `lexer/mod.rs:22-29`
- **修复**: 10MB 限制，处理前检查。

### H-3: 不安全块缺少 `// SAFETY:` 注释
- **文件**: `lib.rs:590,594,622,759,770,777` — 6 个不安全块
- **修复**: 为每个块添加内联 SAFETY 注释。

### H-4: LSP 中未检查的 `.unwrap()` → 错误请求时 panic
- **文件**: `lsp.rs:605`
- **修复**: 用 `match` 替换，不存在文件返回空结果。

### H-5: Vue v-if/v-else-if 双引号属性注入
- **文件**: `codegen/vue.rs:406,420-421`
- **严重性**: 字符串字面量中的 `"` 字符会破坏 `v-if="..."` 属性边界。
- **利用**: `@if name == "Alice" { ... }` → 生成的 HTML 格式错误 `v-if="name === "Alice""`
- **修复**: 将嵌入属性中的 `"` 转义为 `&quot;`。

### H-6: 42 处符号声明错误被静默丢弃
- **文件**: `semantic/mod.rs:95,107,119,131,143,175,187,...`
- **描述**: `symbols.declare(...).ok()` — 重复符号无诊断
- **修复**: 传播错误或发出诊断。

### H-7: React/Vue codegen 无 HTML 转义
- **文件**: `react.rs:283`, `vue.rs:463`
- **描述**: .omni 源中的 `<script>alert(1)</script>` 逐字进入 JSX → XSS。
- **修复**: 添加 HTML 实体编码。

### H-8: 无目标语言关键字冲突检查
- **文件**: 全部 9 个 codegen 文件
- **修复**: 添加关键字集合，自动重命名冲突标识符。

### H-9: 空十六进制/八进制/二进制字面量静默返回 0
- **文件**: `lexer/mod.rs:516,531,546`
- **修复**: 空字面量发出 error token。

### H-10: NaN/Infinity 静默变为 0
- **文件**: `ir/mod.rs:555`
- **修复**: 解析时拒绝 NaN/Infinity。

### H-11: `data-bind-text` 属性值无 HTML 转义
- **文件**: `codegen/omniview.rs:659,1113`
- **修复**: 用 `escape_html()` 包裹。

### H-12: Rust codegen 对非 Option 类型用 `None` 初始化 props
- **文件**: `rust.rs:116-118`
- **修复**: 用 `Option<T>` 包裹 prop 类型或生成合适的构造函数。

### H-13: 整数溢出静默产生零 — 静默数据损坏
- **文件**: `lexer/mod.rs:516,531,546,605`
- **严重性**: 超过 `i64::MAX` 的字面量（例如 `99999999999999999999`、`0xFFFFFFFFFFFFFFFFFFF`）静默变为 `0`，无错误。
- **修复**: 将 `unwrap_or(0)` 替换为显式溢出检测；溢出时发出 error token。

### H-14: `\u{...}` Unicode 转义 — 单引号字符串无条件 advance 破坏数据
- **文件**: `lexer/mod.rs:369`
- **严重性**: `'\u{FFx'` — 十六进制循环在 `x` 处停止，无条件 `self.advance()` 消耗 `x`，破坏数据。双引号版本（第 442 行）正确检查 `}`，但单引号版本不检查。
- **修复**: 添加 `if self.peek() == Some('}')` 守卫，与双引号实现对齐。

---

## 🟡 MEDIUM (21)

1. **Break/Continue/Assign/EventEmit 静默变为 Null IR** — `ir/mod.rs:970-975`
2. **词法分析器 `advance().unwrap()` x12 — 安全但脆弱** — `lexer/mod.rs:440+`
3. **`emit_store` 名称重命名污染** — `omniview.rs:246-308`
4. **全部 target 静默丢弃 WhileStmtIR/ForInStmtIR** — `python.rs:390`, `js.rs:359` 等
5. **JavaScript codegen 静默丢弃 push/pop 变更** — `javascript.rs:338-341`
6. **所有 codegen 目标中 store 信号默认值静默变为 null** — 全部 target
7. **Unicode 转义解析不一致（单引号有 `_`，双引号无）** — `lexer/mod.rs:357-375 vs 434-450`
8. **无效 `\u{}` 被静默忽略** — `lexer/mod.rs:370-374,445-449`
9. **模板字符串无转义序列（无 `\n`/`\t`/`\u{...}`）** — `lexer/mod.rs:466-498`
10. **`sanitize_module_name` 路径遍历不完整（无 `:` 拒绝）** — `codegen/mod.rs:169-176`
11. **`CodeWriter::dedent` 静默下溢** — `codegen/mod.rs:55-58`
12. **重复模块静默覆盖** — `lib.rs:515-528`
13. **BlockExpr 副作用语句静默丢弃** — `omniview.rs:1299-1303`
14. **CSS 数值无单位 → 无效 CSS（例如 `width: 16`）** — `omniview.rs:1346`, `react.rs:734`
15. **React store hooks 不暴露 setter 函数** — `react.rs:237-265`
16. **`e.detail` 无条件传递 → 原生 DOM 事件接收错误数据** — `omniview.rs:947,956,975`
17. **`serde_value_to_js` 无 `\0` 处理** — `omniview.rs:1365-1366`
18. **组件 map 存储 `ComponentIR` 完全克隆 — 大项目内存膨胀** — `omniview.rs:67`
19. **TypeMismatch lint 规则为无操作** — `lints.rs:182-189`
20. **FFI 中 `from_utf8` 失败时静默返回 `"{}"`** — `lib.rs:623,762,771,780`
21. **LSP 跳过 store 符号（StoreDef 缺少 span）** — `lsp.rs:381-383`
22. **未闭合块注释静默消耗到 EOF 无错误** — `lexer/mod.rs:316-325`
23. **未闭合字符串静默产生截断内容无错误** — `lexer/mod.rs:343,400,473`
24. **解析器零错误恢复 — 首个错误终止整个解析** — 所有 parser 文件

---

## 🟢 LOW (12)

1. `escape_html` 不转义单引号 (`omniview.rs:1480`)
2. `serde_value_to_js` 无 NaN/Infinity 处理 (`omniview.rs:1363`)
3. 测试覆盖率较低 (380 tests / 9 targets / ~23K 行)
4. 运行时注释误导性地提及 "WASM" (`omniview.rs:148`)
5. 模板字面量转义效率低下 — 3 次连续 `.replace()` (`omniview.rs:621`)
6. 词法分析器无 BOM 处理 (`lexer/mod.rs`)
7. 死代码: `capitalize_first` (vue.rs), `format_type_ir_dart` (dart.rs), `format_type_ir_rust` (rust.rs)
8. `to_kebab` 中未检查的 `.unwrap_or()` (`omniview.rs:1457`)
9. 无法识别的二元运算符在所有 target 中静默通过 (`map_binary_op`)
10. 自动输入绑定找不到合适信号时静默失败 (`ir/mod.rs:532-547`)
11. Debug HTML 输出中无 Content-Security-Policy (`omniview.rs:1497`)
12. `format_source` 对越界 span 的 `unwrap_or("")` 回退可能损坏格式 (`lsp.rs:1487-1494`)

---

## 📈 安全态势摘要

| 维度 | 评级 | CRITICAL | HIGH | MEDIUM | LOW |
|------|------|----------|------|--------|-----|
| 注入防护 (XSS/HTML/CSS) | ❌ 弱 | 3 | 4 | 2 | 1 |
| 内存安全 (unsafe/unwrap) | ✅ 良好 | 0 | 2 | 1 | 1 |
| 错误处理 (静默失败) | ⚠️ 一般 | 0 | 2 | 6 | 2 |
| 代码生成正确性 | ❌ 差 | 4 | 3 | 5 | 2 |
| 解析器/词法分析器健壮性 | ⚠️ 一般 | 0 | 3 | 4 | 3 |
| 资源限制 | ❌ 缺失 | 0 | 2 | 0 | 0 |
| 测试与文档 | ⚠️ 一般 | 0 | 0 | 0 | 3 |

### ⚠️ 总体评级: 需要注意

编译器适用于**受信任的开发环境**（本地工具链）。在未进行额外的安全加固（特别是注入防护和资源限制）之前，不适合用于面向公众的编译服务。

---

## 🗓️ 修复路线图

### Sprint 1 — 立即 (P0) — 6 项
1. **CVE-1**: CSS 注入 — 阻止 `javascript:` URL
2. **CVE-2**: 运行时名称冲突 — `__omni_` 前缀
3. **CVE-3**: @for 模板 eval 错误 — 正确的表达式求值
4. **CVE-4**: 7-target ExprIR 变体 — 为所有缺失变体添加处理
5. **CVE-5**: Dart 参数语法 — 交换类型/名称顺序
6. **CVE-6**: Dioxus 杂散行 — 删除调试代码
7. **CVE-7**: ViewTree::Text `{...}` 回退 — 移除或保护

### Sprint 2 — 本周 (P1) — 8 项
8. **H-1**: 递归深度限制
9. **H-2**: 输入大小限制
10. **H-3**: 不安全块 SAFETY 注释
11. **H-4**: LSP unwrap 修复
12. **H-5**: Vue v-if 双引号转义
13. **H-6**: 符号声明错误传播
14. **H-7**: React/Vue HTML 转义
15. **H-8**: 关键字冲突检查

### Sprint 3 — 本月 (P2) — 12 项
16. **H-9**: 空字面量错误
17. **H-10**: NaN/Infinity 拒绝
18. **H-11**: data-bind-text 转义
19. **H-12**: Rust None props
20. **M-1** 到 **M-8**: 关键的 MEDIUM 项

### Backlog (P3) — 19 项
21. **M-9** 到 **M-21**: 剩余的 MEDIUM 项
22. **L-1** 到 **L-12**: 所有 LOW 项

---

*五代理并行审计于 2026-06-18 完成。每个文件 ≥2 次审查，安全关键路径 ≥3 次审查。*
