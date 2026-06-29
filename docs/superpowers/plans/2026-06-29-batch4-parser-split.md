# Batch 4: Parser Module Split — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split 4 oversized parser files (`top_level.rs` 2085, `view.rs` 1993, `expr.rs` 1034, `ast.rs` 975) into directories with sub-modules. Every resulting file ≤800 lines.

**Architecture:**
- **top_level.rs → `top_level/`:** One file per declaration type (component, function, store, struct, module, motion). All methods are `impl Parser` blocks split across files.
- **view.rs → `view/`:** mod.rs (element parsing), directives.rs (@if/@for/@show), style.rs (CSS parsing).
- **expr.rs → `expr/`:** mod.rs (Pratt core), motion.rs (easing/motion parsers).
- **ast.rs → `ast/`:** mod.rs (core types), view.rs (OmniView types), motion.rs (motion/animation types).

**Tech Stack:** Rust 2021, cargo test

**Safety net:** `cargo test --lib` must return "375 passed; 0 failed" after every commit.

**Key mechanics:** All parser methods are `impl<'a> Parser<'a>` blocks. Splitting them across files uses Rust's inherent impl block feature — methods from multiple files combine on the same type. Private methods called across files must become `pub(super)`.

---

### Task 1: Split top_level.rs — create directory + move dispatcher

**Files:**
- Create: `src/parser/top_level/mod.rs`
- Delete: `src/parser/top_level.rs`

- [ ] **Step 1: Create directory and move**

```bash
mkdir -p omnilang/src/parser/top_level
mv omnilang/src/parser/top_level.rs omnilang/src/parser/top_level/mod.rs
```

- [ ] **Step 2: Verify `parser/mod.rs` still works**

`parser/mod.rs` has `pub mod top_level;` — this now resolves to `top_level/mod.rs`. No change needed.

```bash
cd omnilang && cargo check 2>&1 | grep "^error" | head -5
```
Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
cd omnilang && cargo fmt
git add omnilang/src/parser/top_level/
git commit -m "refactor: move top_level.rs → top_level/mod.rs (prep for splitting)"
```

---

### Task 2: Split top_level — extract component.rs + function.rs

**Files:**
- Create: `src/parser/top_level/component.rs`
- Create: `src/parser/top_level/function.rs`
- Modify: `src/parser/top_level/mod.rs`

- [ ] **Step 1: Extract component.rs** (~400 lines)

Move `parse_component_def()` and its helper methods (lines ~168-560 in original) to `component.rs`:

```rust
//! Component definition parser.
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_component_def(&mut self) -> Result<ComponentDef, ParseError> {
        // ... (moved verbatim)
    }
    // ... (component-specific helpers)
}
```

- [ ] **Step 2: Extract function.rs** (~350 lines)

Move `parse_function_def()`, `parse_action_def()`, `parse_lifecycle()`, and async function parsing (lines ~561-788):

```rust
//! Function, action, and lifecycle parser.
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_function_def(&mut self, default_vis: Visibility) -> Result<FunctionDef, ParseError> {
        // ... (moved verbatim)
    }
    pub(super) fn parse_action_def(&mut self) -> Result<ActionDef, ParseError> {
        // ... (moved verbatim)
    }
    // ... (parse_lifecycle, async function parsing)
}
```

- [ ] **Step 3: Update mod.rs**

Add at top:
```rust
mod component;
mod function;
```

Remove the moved method bodies. Keep: `parse_module()` dispatcher, `parse_store_def()`, `parse_theme_def()`, `parse_struct_def()`, `parse_tagged_union_def()`, `parse_type_alias()`, `parse_import()`, `parse_export()`, `parse_motion_def()`, `parse_timeline_def()`.

- [ ] **Step 4: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
```
Expected: "375 passed; 0 failed"

```bash
cargo fmt
git add omnilang/src/parser/top_level/
git commit -m "refactor: extract component + function parsers to sub-modules"
```

---

### Task 3: Split top_level — extract remaining parsers

**Files:**
- Create: `src/parser/top_level/store.rs` (~150 lines)
- Create: `src/parser/top_level/struct_def.rs` (~250 lines)
- Create: `src/parser/top_level/module.rs` (~350 lines)
- Create: `src/parser/top_level/motion.rs` (~535 lines)
- Modify: `src/parser/top_level/mod.rs`

Same pattern for each:

- [ ] **Extract store.rs** — `parse_store_def()` + helpers

- [ ] **Extract struct_def.rs** — `parse_struct_def()`, `parse_tagged_union_def()`, `parse_type_alias()`

- [ ] **Extract module.rs** — `parse_module()` (the public entry point that dispatches to all other parsers), `parse_import()`, `parse_export()`. Actually keep `parse_module()` in `mod.rs` since it's the main dispatcher — move only `parse_import()` and `parse_export()`.

- [ ] **Extract motion.rs** — `parse_motion_def()`, `parse_timeline_def()`, `parse_position_string()`

After each extraction: `cargo test --lib` → verify 375 passed → commit.

Final `mod.rs` content (~100 lines): `parse_module()` dispatcher, module declarations, re-exports.

- [ ] **Final verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/parser/top_level/
git commit -m "refactor: extract store, struct_def, module, motion parsers — top_level complete"
```

---

### Task 4: Split view.rs → `view/` directory

**Files:**
- Create: `src/parser/view/mod.rs`
- Create: `src/parser/view/directives.rs`
- Create: `src/parser/view/style.rs`
- Delete: `src/parser/view.rs`

- [ ] **Step 1: Create directory and move**

```bash
mkdir -p omnilang/src/parser/view
mv omnilang/src/parser/view.rs omnilang/src/parser/view/mod.rs
```

- [ ] **Step 2: Extract directives.rs** (~700 lines)

Move directive parsers (lines ~1175-1705):
- `parse_if_directive()`, `parse_for_directive()`, `parse_show_directive()`
- Motion trigger attribute parsers: `parse_enter_trigger()`, `parse_hover_trigger()`, etc.
- `is_keyword_attr_name()`, `is_event_like()`

```rust
//! @if, @for, @show directive parsers and motion trigger attributes.
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_if_directive(&mut self) -> Result<ViewNode, ParseError> { ... }
    pub(super) fn parse_for_directive(&mut self) -> Result<ViewNode, ParseError> { ... }
    // ... (all directive and trigger methods)
}
```

- [ ] **Step 3: Extract style.rs** (~593 lines)

Move style parsing (lines ~1294-1993):
- `parse_css_property_name()`, all style value parsing methods
- `style_value_to_part()` (standalone fn → `pub(super) fn`)
- `consume_css_function_args()`, `try_consume_dimension()`, `try_consume_dimension_part()`, `try_parse_dimension()`, `is_text_token()`, `token_text()`

```rust
//! CSS style parsing: property names, values, dimensions, functions.
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

pub(super) fn style_value_to_part(value: &StyleValue) -> StyleValuePart { ... }

impl<'a> Parser<'a> {
    pub(super) fn parse_css_property_name(&mut self, msg: &str) -> Result<String, ParseError> { ... }
    // ... (all style parsing methods)
}
```

- [ ] **Step 4: Update mod.rs**

Keep in `mod.rs` (~700 lines): `parse_view_block()`, `parse_element()`, and the main element attribute parsing loop that calls into directives and style.

Add at top:
```rust
mod directives;
mod style;
```

- [ ] **Step 5: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/parser/view/
git commit -m "refactor: split view.rs into mod + directives + style sub-modules"
```

---

### Task 5: Split expr.rs → `expr/` directory

**Files:**
- Create: `src/parser/expr/mod.rs`
- Create: `src/parser/expr/motion.rs`
- Delete: `src/parser/expr.rs`

- [ ] **Step 1: Create directory and move**

```bash
mkdir -p omnilang/src/parser/expr
mv omnilang/src/parser/expr.rs omnilang/src/parser/expr/mod.rs
```

- [ ] **Step 2: Extract motion.rs** (~276 lines)

Move easing/motion expression parsers (lines ~758-1034):
- `parse_number_literal()`, `parse_elastic_params()`, `parse_easing()`, `parse_stagger_call()`, `parse_motion_expr()`

```rust
//! Motion and easing expression parsers.
use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_easing(&mut self) -> Result<EasingExpr, ParseError> { ... }
    pub(super) fn parse_motion_expr(&mut self) -> Result<Expr, ParseError> { ... }
    pub(super) fn parse_stagger_call(&mut self) -> Result<StaggerCall, ParseError> { ... }
    pub(super) fn parse_number_literal(&mut self) -> Result<f64, ParseError> { ... }
    fn parse_elastic_params(&mut self) -> Result<EasingExpr, ParseError> { ... }
}
```

- [ ] **Step 3: Update mod.rs**

Keep Pratt parser core (~758 lines): `parse_expr()`, `parse_expr_bp()`, `parse_struct_literal_suffix()`, `parse_match_pattern()`, `parse_match_body()`, `parse_prefix()`, `infix_bp()`, `parse_infix()`, `parse_assign()`, `parse_compound_assign()`, `parse_ident_expr()`, `parse_call_args()`, `parse_map_entries()`, `parse_primary()`.

Add `mod motion;` at top.

- [ ] **Step 4: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/parser/expr/
git commit -m "refactor: split expr.rs — extract motion/easing parsers to sub-module"
```

---

### Task 6: Split ast.rs → `ast/` directory

**Files:**
- Create: `src/parser/ast/mod.rs`
- Create: `src/parser/ast/view.rs`
- Create: `src/parser/ast/motion.rs`
- Delete: `src/parser/ast.rs`

- [ ] **Step 1: Create directory and move**

```bash
mkdir -p omnilang/src/parser/ast
mv omnilang/src/parser/ast.rs omnilang/src/parser/ast/mod.rs
```

- [ ] **Step 2: Extract view.rs** (~210 lines)

Move OmniView-specific AST types (lines ~356-588):
- `ComponentDef`, `PropDecl`, `SignalDecl`, `ComputedDecl`, `EventDecl`, `RefDecl`, `LifecycleHook`, `LifecycleKind`, `ActionDef`, `ViewBlock`, `ViewNode`, `Element`, `ElementProp`, `EventHandler`, `ClassValue`, `IfDirective`, `ForDirective`, `StyleRule`, `StyleValue`, `StyleValuePart`

All types keep `pub` visibility. Add `use super::*;` to access base types like `Expr`, `Stmt`, `Span`, `FunctionDef`.

- [ ] **Step 3: Extract motion.rs** (~310 lines)

Move motion/animation + store/theme types (lines ~589-898):
- `MotionDef`, `TimelineDef`, `MotionBody`, `KeyframeBody`, `KeyframeStop`, `StopPosition`, `TimelineBody`, `TimelineItem`, `TimelineItemKind`, `DirectionItem`, `Direction`, `Connector`, `PositionValue`, `CallbackBody`, `CallbackSpec`, `PlaybackProp`, `LoopValue`, `DirectionValue`, `FillValue`, `InterruptValue`, `WhenValue`, `EasingExpr`, `StaggerCall`, `StaggerTiming`, `StaggerDirection`, `MotionTrigger`, `TriggerKind`, `StateDef`, `HeroDef`, `StoreDef`, `GetterDecl`, `ThemeDef`, `ThemeCategory`

All types keep `pub` visibility.

- [ ] **Step 4: Update mod.rs**

Keep core types (~500 lines):
- `TypeAnnotation`, `Expr`, `Literal`, `BinaryOpKind`, `UnaryOpKind`, `TemplatePart`
- `Stmt`, `AssignTarget`, `Block`
- `MatchArm`, `MatchPattern`, `MatchPatternKind`
- `Param`, `FunctionDef`, `StructField`, `StructDef`, `TaggedUnionDef`, `Variant`, `TypeAlias`
- `Visibility`, `NamedImport`, `NamespaceImport`, `TypeImport`, `ExportFrom`, `ImportStmt`
- `Module`, `Program`

Add at top:
```rust
pub mod view;
pub mod motion;
pub use view::*;
pub use motion::*;
```

The `pub use` re-exports ensure all callers that `use crate::parser::ast::*` still see all types.

- [ ] **Step 5: Verify and commit**

```bash
cd omnilang && cargo test --lib 2>&1 && cargo fmt
git add omnilang/src/parser/ast/
git commit -m "refactor: split ast.rs into mod + view + motion sub-modules"
```

---

### Task 7: Final verification

- [ ] **Step 1: Check all parser file sizes**

```bash
cd omnilang && find src/parser -name "*.rs" -exec wc -l {} \; | sort -rn | awk '$1 > 800 {print $1, $2}'
```
Expected: zero files over 800 lines.

- [ ] **Step 2: Full test suite + eval harness**

```bash
cd omnilang && cargo test --lib 2>&1 | grep "test result:"
bash .claude/evals/eval-run-omnilang.sh
```

- [ ] **Step 3: Commit**

```bash
git commit -m "chore: verify parser split — all files ≤800, 375 tests green"
```
