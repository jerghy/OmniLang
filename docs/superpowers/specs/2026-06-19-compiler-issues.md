# OmniLang 编译器问题汇总

> 来源：learn-omniview 全 7 阶段教学项目实战（21 文件，~89 特性，380 测试）
> 日期：2026-06-19
> 基准：omni-compiler @ main 分支

---

## ❌ 不支持的特性（13 个）

| # | 问题 | 阶段 | 错误信息 |
|---|------|------|---------|
| 1 | 文件顶层 `const` | P6 | `Unexpected token at top level: Const` — 仅 `component { }` 内可用 |
| 2 | `pub const` | P6 | `Expected fn, struct, or type after 'pub'` |
| 3 | `$prefixed const` 无法引用 | P1/P6 | 解析通过但引用时报 `Unexpected token in expression: Dollar` |
| 4 | `computed` 在 `store` 内 | P6 | `Unexpected token in store: Computed` — store 仅支持 `signal`/`getter`/`action` |
| 5 | `?:` 三元运算符 | P2 | 完全不存在，用 `if/else` 表达式替代 |
| 6 | `@init` 生命周期 | P3 | 不存在，仅 `@mount`/`@unmount`/`@effect(deps)` |
| 7 | Store 在 `fn` 体内不可访问 | P2/P5 | Store action/field 只能从 view/内联事件回调调用 |
| 8 | Store 在 `computed` 内不可访问 | P5 | `computed` 只追踪本地 signal |
| 9 | `let` 在循环体内 | P4 | `Undeclared variable` — 仅函数顶层可用 |
| 10 | 链式访问 `obj.method().field` | P1 | View 插值不支持 |
| 11 | JS 风格命名空间导入 `import * as X from` | P7 | `expected import name found Star` — OmniLang 用 `import "path" as X;` |
| 12 | `const name: Type = val` 带类型标注 | P6 | `expected '=' found Colon` — const 不能带类型 |
| 13 | `char_at` 方法 | P4 | String 无字符级操作 |

---

## ⚠️ 解析器 Bug / 意外行为（11 个）

| # | 问题 | 阶段 | 现象 |
|---|------|------|------|
| 14 | CSS hex 科学记数法 | P1 | 含 `e` 的 hex（`#e5e7eb`）被当科学记数法解析 |
| 15 | View 文本禁字 `[` `=` `$` | P1 | 这三个字符导致解析错误 |
| 16 | View 文本禁字 `>` | P3 | `Unexpected token in view: Gt` — 被当标签闭合 |
| 17 | View 文本禁字 `&` | P4 | `unterminated HTML comment` — 被当 HTML 实体起始 |
| 18 | inline `style:` 中 hyphenated CSS 属性 | P6 | `border-bottom` 报 `expected ':' found Minus` — CSS `style {}` 块中正常 |
| 19 | `class:` map 中 hyphenated class 名 | P3 | `modal-content` 解析失败 |
| 20 | `arr[i]` 在 view 插值中 | P1 | 函数中正常，view 插值不工作 |
| 21 | `theme` 关键字无代码生成 | P6 | 解析通过但 codegen 是 no-op |
| 22 | Boolean attr 字符串化 | P3 | `disabled={false}` → HTML 输出 `disabled="false"` |
| 23 | 自定义 attr 被加 `data-` 前缀 | P3 | `aria-label` → `data-aria-label` |
| 24 | Lexer 回归：中文断裂 | P1 | 未提交 `is_id_start()` 修改破坏中文（已 stash） |

---

## 🔧 需规避的写法（6 个）

| # | 问题 | 阶段 | 正确写法 |
|---|------|------|---------|
| 25 | `let` 跨分支重复声明 | P5 | 顶层 `let mut x = default;` 再在分支中赋值 |
| 26 | Map 字面量 key 须加引号 | P4 | `{"key": val}` 而非 `{key: val}` |
| 27 | Map 访问用 `.get()` | P4 | `map.get("key")` 而非 `map["key"]` |
| 28 | Closure 语法 | P4 | `\|p1, p2\| body` 而非 JS `=>` |
| 29 | `async action` 语法顺序 | P5 | `action name(p: T): Ret async { }` — `async` 在返回类型之后 |
| 30 | `<input />` 自闭合 | P1 | 用 `<input></input>`；`<img />` `<br />` `<hr />` 正常 |

---

## 按发现阶段

| 阶段 | 新增问题 | 累计 |
|------|---------|------|
| Phase 1 (类型+逻辑) | #3, #10, #14, #15, #20, #24, #30 | 7 |
| Phase 2 (Store+Slot) | #5, #7 | 9 |
| Phase 3 (表单+表格) | #6, #16, #17, #19, #22, #23 | 15 |
| Phase 4 (循环+闭包) | #9, #13, #26, #27, #28 | 20 |
| Phase 5 (FilterBar+TodoStore) | #8, #11, #25, #29 | 24 |
| Phase 6 (Theme 系统) | #1, #2, #4, #12, #18, #21 | 30 |
| Phase 7 (模块系统) | — (无新增) | 30 |

---

## 按优先级（修复建议）

**P0 — 阻碍日常开发：** #14 (hex bug), #15-#17 (view 文本禁字), #24 (lexer 回归), #18 (inline style hyphenated)

**P1 — 限制关键能力：** #1, #2 (顶层 const), #7, #8 (store 访问限制), #3, #4, #5, #6

**P2 — 影响开发体验：** #9-#13, #19-#23, #25-#30

**已由 parser 测试覆盖但语义未完成：** tagged union 变体构造/导出
