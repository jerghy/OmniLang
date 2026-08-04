//! FormatStmt trait — eliminates statement formatting duplication across 9 codegen backends.
//!
//! Provides a single `format_stmt` dispatch method backed by overrideable helpers
//! for each `StatementIR` variant. Default implementations produce C-family syntax
//! (JS/Dart/Rust style). Backends override only the methods that differ.
//!
//! | Formatter | Family | Key characteristics |
//! |-----------|--------|-------------------|
//! | `JsStmtFormatter` | JS (JavaScript) | `let`, `null`, `const` for-of, TS annotations |
//! | `ReactStmtFormatter` | React (TSX) | `let`, `null`, `const` for-of, setter mutations |
//! | `VueStmtFormatter` | Vue (TS) | `const`, `null`, `const` for-of, `.value` mutations |
//! | `DartStmtFormatter` | Dart (Dart, Flutter) | `var`/type-first, `null`, `var` for-of |
//! | `FlutterStmtFormatter` | Flutter | Same as Dart + `setState` mutation wrapping |
//! | `RustStmtFormatter` | Rust | `let`, `None`, bare for-in, `self.` mutations |
//! | `DioxusStmtFormatter` | Dioxus | `let`, `Default::default()`, bare for-in, `.set()` mutations |
//! | `PythonStmtFormatter` | Python | bare name, `None`, no semicolons, `elif` keyword |

use crate::codegen::shared::expr::FormatExpr;
use crate::codegen::CodeWriter;
use crate::ir::*;

// ═══════════════════════════════════════════════════════════
// Trait definition
// ═══════════════════════════════════════════════════════════

/// Trait for target-language statement formatting.
///
/// Default implementations produce C-family syntax (JS/Dart/Rust style).
/// Backends override only the methods that differ.
pub trait FormatStmt {
    /// The expression formatter used by this statement formatter.
    type ExprFormatter: FormatExpr;

    /// Access the expression formatter.
    fn expr_fmt(&self) -> &Self::ExprFormatter;

    // ═══ Language-specific settings (override per backend) ═══

    /// Keyword for `let`-style variable declarations (e.g., `"let"`, `"var"`, `"let mut"`).
    fn let_keyword(&self) -> &str {
        "let"
    }

    /// Keyword for MUTABLE `let`-style declarations — defaults to
    /// `let_keyword()`. Rust-family backends override with `"let mut"`
    /// (Task 5.1 — a reassigned binding must be declared mutable in
    /// Rust-compiled output, not just at the IR level).
    fn mutable_let_keyword(&self) -> &str {
        self.let_keyword()
    }

    /// Fallback value when a statement position expression is None.
    fn null_value(&self) -> &str {
        "null"
    }

    /// Fallback value for `return` when no value is provided.
    fn return_null_value(&self) -> &str {
        self.null_value()
    }

    /// Whether to append a semicolon at the end of most statements.
    fn needs_semicolon(&self) -> bool {
        true
    }

    /// Whether to use `const` instead of the `let_keyword()` for immutable
    /// bindings. `let mut` bindings ALWAYS use `let_keyword()` regardless of
    /// this flag (Task 5.1 — the IR `is_mut` flag decides).
    fn use_const_for_immutable(&self) -> bool {
        false
    }

    /// Keyword before the loop variable in for-in statements.
    /// JS: `"const"`,  Dart: `"var"`,  Rust: `""`.
    fn for_in_keyword(&self) -> &str {
        "const"
    }

    /// Whether for-in uses `of` (JS-style) or `in` (Dart-style).
    fn for_in_uses_of(&self) -> bool {
        true
    }

    // ═══ Type annotation for let statements ═══

    /// Format a type annotation for a let statement (e.g., `: string` or `String`).
    fn format_let_type_ann(&self, _ty: &Option<TypeIR>) -> String {
        String::new()
    }

    // ═══ Core dispatch ═══

    /// Format any `StatementIR` node according to this language's syntax.
    fn format_stmt(&self, w: &mut CodeWriter, stmt: &StatementIR) {
        match stmt {
            StatementIR::LetStmtIR {
                name,
                value,
                r#type,
                is_mut,
            } => self.format_let_stmt(w, name, value, r#type, *is_mut),
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => self.format_if_stmt(w, condition, then_body, else_ifs, else_body),
            StatementIR::ReturnStmtIR { value } => self.format_return_stmt(w, value),
            StatementIR::WhileStmtIR { condition, body } => {
                self.format_while_stmt(w, condition, body)
            }
            StatementIR::ForInStmtIR {
                var_name,
                iterable,
                body,
            } => self.format_for_in_stmt(w, var_name, iterable, body),
            StatementIR::ExprStmtIR { expr } => self.format_expr_stmt(w, expr),
            StatementIR::BreakStmtIR => self.format_break_stmt(w),
            StatementIR::ContinueStmtIR => self.format_continue_stmt(w),
            StatementIR::MutationStmt { intent } => {
                if let Some(intent) = intent {
                    self.format_mutation_stmt(w, intent);
                }
            }
            StatementIR::EventEmitStmtIR { event_name, args } => {
                self.format_event_emit_stmt(w, event_name, args);
            }
            StatementIR::BlockStmtIR { body } => {
                self.format_block(w, body);
            }
            StatementIR::ThrowStmtIR { expr } => self.format_throw_stmt(w, expr),
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_var,
                catch_body,
                finally_body,
            } => self.format_try_catch_stmt(w, try_body, catch_var, catch_body, finally_body),
        }
    }

    // ═══ Per-variant overridable methods ═══

    /// Format a `let` statement.
    ///
    /// `is_mut` (the IR `LetStmtIR.is_mut` flag) selects the keyword:
    /// mutable → `let_keyword()`; immutable → `const` when
    /// `use_const_for_immutable()`, else `let_keyword()`.
    fn format_let_stmt(
        &self,
        w: &mut CodeWriter,
        name: &str,
        value: &Option<ExprIR>,
        type_ann: &Option<TypeIR>,
        is_mut: bool,
    ) {
        let kw = if is_mut {
            self.mutable_let_keyword()
        } else if self.use_const_for_immutable() {
            "const"
        } else {
            self.let_keyword()
        };
        let ty = self.format_let_type_ann(type_ann);
        let rhs = value
            .as_ref()
            .map(|v| self.expr_fmt().format_expr(v))
            .unwrap_or_else(|| self.null_value().to_string());
        let semi = if self.needs_semicolon() { ";" } else { "" };
        w.emit_fmt(format_args!("{kw} {name}{ty} = {rhs}{semi}"));
    }

    /// Format a `return` statement.
    fn format_return_stmt(&self, w: &mut CodeWriter, value: &Option<ExprIR>) {
        let v = value
            .as_ref()
            .map(|v| self.expr_fmt().format_expr(v))
            .unwrap_or_else(|| self.return_null_value().to_string());
        let semi = if self.needs_semicolon() { ";" } else { "" };
        w.emit_fmt(format_args!("return {v}{semi}"));
    }

    /// Format an `if/else if/else` chain.
    fn format_if_stmt(
        &self,
        w: &mut CodeWriter,
        condition: &ExprIR,
        then_body: &BlockIR,
        else_ifs: &[(ExprIR, BlockIR)],
        else_body: &Option<BlockIR>,
    ) {
        let cond = self.expr_fmt().format_expr(condition);
        w.emit_fmt(format_args!("if ({cond}) {{"));
        w.indent();
        self.format_block(w, then_body);
        w.dedent();
        for (elif_cond, elif_body) in else_ifs {
            let elif = self.expr_fmt().format_expr(elif_cond);
            w.emit_fmt(format_args!("}} else if ({elif}) {{"));
            w.indent();
            self.format_block(w, elif_body);
            w.dedent();
        }
        if let Some(ref eb) = else_body {
            if !eb.statements.is_empty() {
                w.emit("} else {");
                w.indent();
                self.format_block(w, eb);
                w.dedent();
            }
        }
        w.emit("}");
    }

    /// Format a `while` statement.
    fn format_while_stmt(&self, w: &mut CodeWriter, condition: &ExprIR, body: &BlockIR) {
        let cond = self.expr_fmt().format_expr(condition);
        w.emit_fmt(format_args!("while ({cond}) {{"));
        w.indent();
        self.format_block(w, body);
        w.dedent();
        w.emit("}");
    }

    /// Format a `for...in` statement.
    fn format_for_in_stmt(
        &self,
        w: &mut CodeWriter,
        var_name: &str,
        iterable: &ExprIR,
        body: &BlockIR,
    ) {
        let iter = self.expr_fmt().format_expr(iterable);
        let kw = self.for_in_keyword();
        let sep = if self.for_in_uses_of() { "of" } else { "in" };
        if kw.is_empty() {
            w.emit_fmt(format_args!("for {var_name} {sep} {iter} {{"));
        } else {
            w.emit_fmt(format_args!("for ({kw} {var_name} {sep} {iter}) {{"));
        }
        w.indent();
        self.format_block(w, body);
        w.dedent();
        w.emit("}");
    }

    /// Format an expression statement.
    fn format_expr_stmt(&self, w: &mut CodeWriter, expr: &ExprIR) {
        let line = self.expr_fmt().format_expr(expr);
        if line != self.null_value() {
            if self.needs_semicolon() {
                w.emit_fmt(format_args!("{line};"));
            } else {
                w.emit(&line);
            }
        }
    }

    /// Format a `break` statement.
    fn format_break_stmt(&self, w: &mut CodeWriter) {
        let semi = if self.needs_semicolon() { ";" } else { "" };
        w.emit_fmt(format_args!("break{semi}"));
    }

    /// Format a `continue` statement.
    fn format_continue_stmt(&self, w: &mut CodeWriter) {
        let semi = if self.needs_semicolon() { ";" } else { "" };
        w.emit_fmt(format_args!("continue{semi}"));
    }

    // ═══ Mutation statement support methods ═══

    /// How to reference a mutation target variable in this language.
    fn mutation_target_expr(&self, target: &str) -> String {
        format!("this.{}", target)
    }

    /// Method name for array push in this language.
    fn mutation_push_method(&self) -> &'static str {
        "push"
    }

    /// Method name for array pop in this language.
    fn mutation_pop_method(&self) -> &'static str {
        "pop"
    }

    /// Optional prefix emitted before a mutation statement (e.g., `setState(() {`).
    fn mutation_stmt_prefix(&self) -> Option<&str> {
        None
    }

    /// Optional suffix emitted after a mutation statement (e.g., `});`).
    fn mutation_stmt_suffix(&self) -> Option<&str> {
        None
    }

    /// Format a DirectAssign mutation (override for backends needing special syntax like `.set()`).
    fn format_direct_assign_mutation(&self, w: &mut CodeWriter, target: &str, amount: &str) {
        let semi = if self.needs_semicolon() { ";" } else { "" };
        w.emit_fmt(format_args!("{target} = {amount}{semi}"));
    }

    /// Format a mutation statement — handles all variants via tagged enum dispatch.
    fn format_mutation_stmt(&self, w: &mut CodeWriter, intent: &MutationIntent) {
        let semi = if self.needs_semicolon() { ";" } else { "" };

        if let Some(prefix) = self.mutation_stmt_prefix() {
            w.emit(prefix);
            w.indent();
        }

        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amount = value
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                self.format_direct_assign_mutation(w, &ref_t, &amount);
            }
            MutationIntent::Increment { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} += {amt}{semi}"));
            }
            MutationIntent::Decrement { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} -= {amt}{semi}"));
            }
            MutationIntent::Multiply { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} *= {amt}{semi}"));
            }
            MutationIntent::Divide { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} /= {amt}{semi}"));
            }
            MutationIntent::Modulo { target, amount } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let amt = amount
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "0".into());
                w.emit_fmt(format_args!("{ref_t} %= {amt}{semi}"));
            }
            MutationIntent::Push { target, item } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let it = item
                    .as_ref()
                    .map(|v| self.expr_fmt().format_expr(v))
                    .unwrap_or_else(|| "undefined".into());
                let push_m = self.mutation_push_method();
                w.emit_fmt(format_args!("{ref_t}.{push_m}({it}){semi}"));
            }
            MutationIntent::Pop { target } => {
                let ref_t = self.mutation_target_expr(&target.name);
                let pop_m = self.mutation_pop_method();
                w.emit_fmt(format_args!("{ref_t}.{pop_m}(){semi}"));
            }
        }

        if let Some(suffix) = self.mutation_stmt_suffix() {
            w.dedent();
            w.emit(suffix);
        }
    }

    /// Format an event emit statement.
    fn format_event_emit_stmt(&self, w: &mut CodeWriter, event_name: &str, args: &[ExprIR]) {
        let arg_str = if args.is_empty() {
            String::from("()")
        } else {
            let formatted: Vec<String> = args
                .iter()
                .map(|a| self.expr_fmt().format_expr(a))
                .collect();
            format!("({})", formatted.join(", "))
        };
        w.emit_fmt(format_args!("{event_name}{arg_str};"));
    }

    /// Format a `throw` statement.
    fn format_throw_stmt(&self, w: &mut CodeWriter, expr: &ExprIR) {
        let e = self.expr_fmt().format_expr(expr);
        w.emit_fmt(format_args!("throw {e};"));
    }

    /// Format a `try/catch/finally` statement.
    fn format_try_catch_stmt(
        &self,
        w: &mut CodeWriter,
        try_body: &BlockIR,
        catch_var: &Option<String>,
        catch_body: &Option<BlockIR>,
        finally_body: &Option<BlockIR>,
    ) {
        w.emit("try {");
        w.indent();
        self.format_block(w, try_body);
        w.dedent();
        if let Some(ref cv) = catch_var {
            w.emit_fmt(format_args!("}} catch ({cv}) {{"));
        } else if catch_body.is_some() {
            w.emit("} catch (_) {");
        }
        if let Some(ref cb) = catch_body {
            w.indent();
            self.format_block(w, cb);
            w.dedent();
        }
        if let Some(ref fb) = finally_body {
            w.emit("} finally {");
            w.indent();
            self.format_block(w, fb);
            w.dedent();
        }
        w.emit("}");
    }

    /// Format a block of statements.
    fn format_block(&self, w: &mut CodeWriter, block: &BlockIR) {
        for stmt in &block.statements {
            self.format_stmt(w, stmt);
        }
    }
}
