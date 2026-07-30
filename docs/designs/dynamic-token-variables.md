# Dynamic Token Variables in Design Blocks

## 动机

当前 design block 中的 token 值只能是静态字面量：

```omni
design AppDesign {
    token primary: color = "#6366f1";     // ✅ 静态值
    token spacing: size = "16px";         // ✅ 静态值
    token primary: color = myColor;       // ❌ 不支持 — 被解析为字面量字符串 "myColor"
    token primary: color = store.mode;    // ❌ 不支持 — 被解析为字面量字符串 "store.mode"
}
```

用户希望：
1. 引用 import 的常量：`import { BRAND_COLOR } from "./tokens.omni";` → `token primary: color = BRAND_COLOR;`
2. 引用 store 中的 signal：`token primary: color = ThemeStore.primaryColor;`

## 现状分析

通过全管线分析，design token 系统与表达式系统**完全隔离**：

| 管线层级 | Expression 表达式 | Design Token | 差距 |
|---------|------------------|--------------|------|
| Parser | Pratt 解析器（24 种 Expr 变体） | 手写字符串解析器 (`parse_string_or_ident`) | 完全不同的解析器 |
| AST | `Expr` 枚举 | `TokenDef { value: String }` | 纯字符串 vs 结构化 |
| Semantic | 符号收集 + 引用解析 + 类型检查 | **完全跳过** | 无符号/类型信息 |
| IR | `ExprIR` 结构化 IR 节点 | `TokenIR { value: String }` | 纯字符串 |
| Codegen | JS 表达式代码生成 | 静态 CSS `:root { --token: value; }` | 运行时 vs 编译时 |

### 关键约束

1. **CSS 自定义属性无法在编译时求值动态表达式** — CSS 的 `:root { --x: val }` 只能包含字面量
2. **已有的桥接机制** — `emit_style_assignment()` 通过 `el.style.setProperty("--name", val)` 在运行时设置 CSS 变量
3. **Signal 响应式系统** — `__omni_effect(() => { el.style.setProperty("--x", signal.value); })` 是已验证的模式
4. **Theme 运行时** — `__omni_theme.set(name)` 切换 `data-theme` HTML 属性，CSS 侧通过 `[data-theme="dark"]` 选择器批量覆盖 CSS 变量值

## 设计方案：两阶段渐进式

### Phase 1: 编译时常量引用（低风险，高价值）

**目标**：支持 import 的常量和简单标识符引用

```omni
// tokens.omni
const BRAND = "#6366f1";
const SPACING = "16px";

// theme.omni
import { BRAND, SPACING } from "./tokens.omni";

design AppDesign {
    token primary: color = BRAND;       // → 编译时求值为 "#6366f1"
    token spacing: size = SPACING;      // → 编译时求值为 "16px"
}
```

**实现路径**：

#### 1.1 Parser 层 (`parser/design.rs`)

修改 `parse_token()` 中值的解析逻辑，从 `parse_string_or_ident()` 改为：
- 先尝试用 expression parser 解析值部分（`=` 后，`;` 前）
- 如果解析为 `Expr::Literal(String)` 或 `Expr::VarRef`，接受
- 其他复杂表达式返回 parse error（Phase 1 限制）

AST 变更：
```rust
// ast/design.rs
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: TokenValue,  // String → TokenValue
    pub theme_values: Vec<(String, TokenValue)>,  // String → TokenValue
}

pub enum TokenValue {
    Literal(String),          // "#6366f1", "16px"
    Variable(String),         // "BRAND", "SPACING" — 编译时常量引用
    Expression(Expr),         // 任意表达式 (Phase 2 用)
}
```

**影响文件**：
- `crates/omnilang-core/src/parser/ast/design.rs` — TokenDef, TokenValue
- `crates/omnilang-core/src/parser/design.rs` — parse_token(), parse_string_or_ident()
- `crates/omnilang-core/src/parser/top_level/design.rs` — parse_design_def() 需要传入 parser 引用而非独立字符串解析

**关键挑战**：当前 design parser 是独立的字符串解析器（不依赖 `Parser` 结构体和 token 流）。需要改为：
- 在 `parse_design_def()` 中，将 `{...}` 之间的内容用 expression tokenizer 重新分词
- 或者将 design body 解析集成到主 parser 中

#### 1.2 Semantic 层 (`semantic/`)

在 `collect_module_symbols()` 中添加 design token 符号收集：
```rust
// collect.rs — 新增
for design in &module.designs {
    for token in &design.tokens {
        self.declare_or_warn(Symbol {
            name: token.name.clone(),
            kind: SymbolKind::Unknown,
            type_ann: Some(token.token_type.to_string()),
            ..
        });
    }
}
```

在 `resolve_and_validate_module()` 中添加 token 值的引用解析：
```rust
// resolve.rs — 新增
for design in &module.designs {
    for token in &design.tokens {
        if let TokenValue::Variable(name) = &token.value {
            // 检查 name 是否在符号表中
            // 检查 name 是否可求值为编译时常量
        }
    }
}
```

#### 1.3 IR 层 (`ir/lower/design.rs`)

`lower_design()` 需要接受 `&SemanticResult` 参数来查询编译时常量值：
```rust
fn lower_design(ast: &DesignDef, semantics: &SemanticResult) -> DesignIR {
    let tokens = ast.tokens.iter().map(|t| {
        let resolved_value = match &t.value {
            TokenValue::Literal(s) => s.clone(),
            TokenValue::Variable(name) => {
                // 在语义符号表中查找常量值
                resolve_compile_time_const(name, semantics)
                    .unwrap_or_else(|| format!("var(--{})", name))
            }
            _ => unreachable!(),
        };
        // ...
    }).collect();
}
```

**影响文件**：
- `crates/omnilang-core/src/ir/lower/design.rs` — lower_design() 增加语义参数
- `crates/omnilang-core/src/ir/lower/mod.rs` — 调用处传入 semantics
- `crates/omnilang-core/src/ir/design.rs` — TokenIR 不变（仍然是 String）

#### 1.4 Codegen 层

**无需变更** — 因为 Phase 1 在编译时完成常量求值，最终 TokenIR 中仍是静态 String。

### Phase 2: 动态表达式支持（Signal/Store 引用）

**目标**：支持 token 值引用运行时 signal，自动生成响应式 CSS 更新

```omni
design AppDesign {
    token primary: color = ThemeStore.primaryColor;
    token accent: color = App_dynamicAccent;
}
```

编译为：
```javascript
// Phase 2 生成的代码
__omni_effect(() => {
    const root = document.documentElement;
    root.style.setProperty("--primary", ThemeStore_primaryColor.value);
});
__omni_effect(() => {
    const root = document.documentElement;
    root.style.setProperty("--accent", App_dynamicAccent.value);
});
```

#### 2.1 Parser 层

在 `TokenValue::Expression(Expr)` 中允许任意表达式。复用现有的 expression parser。

#### 2.2 Semantic 层

扩展 Phase 1 的引用解析，对 `TokenValue::Expression(expr)` 递归调用 `resolve_expr_refs()`：
```rust
for design in &module.designs {
    for token in &design.tokens {
        match &token.value {
            TokenValue::Expression(expr) => {
                self.resolve_expr_refs(expr, &mut vctx);
                // 验证表达式类型是 String 或可转换为 String
            }
            _ => { /* Phase 1 逻辑 */ }
        }
    }
}
```

检查 token_type 与表达式类型的兼容性：
- `TokenType::Color` → 期望 `String`
- `TokenType::Size` → 期望 `String` 或 `Int/Float + 单位`
- `TokenType::Duration` → 期望 `String` 或 `Int/Float`

#### 2.3 IR 层 (`TokenIR` 变更)

```rust
// ir/design.rs
pub struct TokenIR {
    pub value: TokenValueIR,    // String → TokenValueIR
    pub token_type: TokenTypeIR,
    pub theme_values: FxHashMap<String, TokenValueIR>,
    pub source_span: Option<Span>,
}

pub enum TokenValueIR {
    Static(String),             // "#6366f1", "16px"
    Dynamic(ExprIR),            // 运行时表达式 — Phase 2
}
```

#### 2.4 Codegen 层

CSS 生成 (`design.rs:emit_design_css_impl()`)：
- `TokenValueIR::Static(s)` → 行为不变：`:root { --token: s; }`
- `TokenValueIR::Dynamic(expr)` → **静态 CSS 中跳过**，在 JS 侧生成 `__omni_effect`

JS 生成（在 `emit_theme_runtime()` 之后新增 `emit_dynamic_token_effects()`）：
```rust
pub(super) fn emit_dynamic_token_effects(&mut self) {
    for design in &self.design_map.values() {
        for (name, token) in &design.tokens {
            if let TokenValueIR::Dynamic(expr) = &token.value {
                let js_expr = self.expr_to_js(expr);
                // 复用已有的 CSS 自定义属性设置模式
                let _ = write!(
                    self.js,
                    "__omni_effect(() => {{\n\
                     \x20 const root = document.documentElement;\n\
                     \x20 root.style.setProperty(\"--{}\", __omni_sanitizeCss({}));\n\
                     }});\n",
                    name, js_expr
                );
            }
        }
    }
}
```

**影响文件**：
- `crates/omnilang-core/src/ir/design.rs` — TokenValueIR enum
- `crates/omnilang-core/src/ir/lower/design.rs` — lower_design(), resolve_value()
- `omnilang/src/codegen/omniview/design.rs` — emit_design_css_impl(), 新增 emit_dynamic_token_effects()
- `omnilang/src/codegen/omniview/mod.rs` — generate() 调用链

### 与现有 Theme 系统的关系

Dynamic token 和 Theme 系统可以共存。规则：

1. **静态 `:root` 默认值**：如果 token 是动态的，`:root` 中不声明初始值（或声明一个 CSS fallback `var(--token, fallback)`)
2. **Theme 覆盖**：如果 token 是动态的但有 theme 覆盖，theme 覆盖仍生效（CSS 级联中 `[data-theme="dark"] --primary: val` 会覆盖 `:root` 的值）
3. **动态值优先级**：JS 通过 `document.documentElement.style.setProperty()` 设置的值具有最高优先级（inline style on `<html>`）

建议行为：动态 token 值通过设置 `<html>` 的 inline style 来运作。这保证了它与 `data-theme` 属性协同工作 — CSS 级联会优先使用 inline style 值。

### 实现优先级和风险

| 任务 | 复杂度 | 风险 | 优先级 |
|------|--------|------|--------|
| Parser: TokenValue enum | 中 | 低 — 纯加性变更 | P0 |
| Parser: design body 集成到主 parser | 高 | 中 — 破坏现有独立字符串解析 | P0 |
| Semantic: token 符号收集 | 低 | 低 — 加性变更 | P0 |
| Semantic: 常量引用解析 | 中 | 中 — 需要常量求值机制 | P0 |
| IR: TokenValueIR | 中 | 低 — 加性变更 | P1 |
| IR: 常量求值器 | 中 | 中 — 新的 IR 子系统 | P1 |
| Codegen: CSS 跳过动态 token | 低 | 低 | P1 |
| Codegen: 动态 JS 效果生成 | 中 | 低 — 复用现有 `emit_style_assignment` | P1 |
| Phase 2: 完整表达式支持 | 高 | 中 — 多管线协同 | P2 |

### 关键决策

- **D1: 常量求值边界** — Phase 1 仅支持 `const` 导入和在 token 定义前声明的字面量常量。不支持函数调用、运算等（即使它们理论上可以在编译时求值）
- **D2: CSS fallback 策略** — 动态 token 在 `:root` 中不声明静态值，依赖 CSS 的 `var()` fallback 机制
- **D3: Theme 覆盖优先级** — JS `style.setProperty()` > CSS `[data-theme]` > CSS `:root` fallback
- **D4: 跨文件引用** — 被 import 的常量必须来自同项目的 `.omni` 文件，通过现有 import 系统解析

### 被拒绝的方案

- **方案 A: 仅用 CSS `var()` 链接 token** — `token primary: color = var(--otherToken);` — 太有限，不解决变量引用问题
- **方案 B: 完全运行时方案** — 所有 token 都变成 `__omni_effect` — 性能开销太大，静态 token 仍应走 CSS 自定义属性
- **方案 C: 新增 `@dynamic` token 语法** — 如 `@dynamic token primary: color = expr;` — 增加语法复杂度，且类型系统已经可以区分静态/动态
