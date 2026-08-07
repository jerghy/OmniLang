//! Warns when a `@for` over OBJECT items (record/struct/map element type)
//! has no `key={expr}` — state is silently lost on reorder/filter.
//!
//! Category: correctness
//! Severity: warning
//!
//! Design doc 域 6 (控制流) 诊断表: 对象项 @for 无 key → warning
//! 「对象项列表建议提供 key=；未提供时使用索引，重排将丢失状态」.
//!
//! Runs at the AST level so the declared signal/prop/computed/const element
//! types are visible (the IR carries no type info). Unknown iterable types
//! are skipped — the rule is conservative and never fires on an untyped
//! expression.

use super::{LintDiagnostic, LintRule, LintSeverity};
use crate::parser::ast::*;
use rustc_hash::FxHashMap;

/// The design doc 域 6 诊断表 message, verbatim.
const FOR_KEY_WARNING: &str = "对象项列表建议提供 key=；未提供时使用索引，重排将丢失状态";

/// v4 perf HIGH: key 表达式是对象本身（裸循环变量）——OmniView keyFn 的
/// String() 会把全部项塌缩为同一 key（"[object Object]"），键控复用/FLIP
/// 形同虚设。必须用字段访问（item.id）或原始类型。
const FOR_KEY_OBJECT_WARNING: &str =
    "key 表达式是循环对象本身——全部项塌缩为同一 key；请用 item.id 等字段访问（如 key={item.id}）";

pub struct ForKey;

impl LintRule for ForKey {
    fn name(&self) -> &str {
        "for-key"
    }
    fn category(&self) -> &str {
        "correctness"
    }
    fn severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check_ast(&self, program: &Program, file_path: &str) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();
        for module in program.modules.values() {
            // Declared types visible to the view: component signals, props,
            // computeds, store signals, plus module-level consts (inferred
            // from values).
            let mut typed: FxHashMap<&str, &TypeAnnotation> = FxHashMap::default();
            let mut consts: FxHashMap<&str, &Expr> = FxHashMap::default();
            for comp in &module.components {
                for s in &comp.signals {
                    typed.insert(s.name.as_str(), &s.type_ann);
                }
                for p in &comp.props {
                    typed.insert(p.name.as_str(), &p.type_ann);
                }
                for c in &comp.computeds {
                    typed.insert(c.name.as_str(), &c.type_ann);
                }
            }
            for store in &module.stores {
                for s in &store.signals {
                    typed.insert(s.name.as_str(), &s.type_ann);
                }
            }
            for c in &module.consts {
                consts.insert(c.name.as_str(), &c.value);
            }

            for comp in &module.components {
                if let Some(view) = &comp.view {
                    check_view_nodes(&view.children, &typed, &consts, file_path, &mut diags);
                }
            }
        }
        diags
    }
}

/// Element-type classification for a `@for` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    /// Identity-stable primitive (int/float/string/bool/...) — no key needed.
    Primitive,
    /// Record/struct/map object item — key recommended (the warning case).
    Object,
    /// Type could not be determined — never warn (conservative).
    Unknown,
}

fn check_view_nodes(
    nodes: &[ViewNode],
    typed: &FxHashMap<&str, &TypeAnnotation>,
    consts: &FxHashMap<&str, &Expr>,
    file_path: &str,
    diags: &mut Vec<LintDiagnostic>,
) {
    for node in nodes {
        match node {
            ViewNode::ForDirective(for_dir) => {
                if for_dir.key_expr.is_none()
                    && infer_item_kind(&for_dir.iterable, typed, consts) == ItemKind::Object
                {
                    diags.push(LintDiagnostic {
                        rule: "for-key".into(),
                        severity: LintSeverity::Warning,
                        message: FOR_KEY_WARNING.into(),
                        file: file_path.to_string(),
                        line: for_dir.span.line,
                        col: for_dir.span.col,
                    });
                }
                // v4 perf HIGH: key 表达式存在但为对象本身（裸循环变量）——
                // OmniView keyFn 的 String() 会把全部项塌缩为
                // "[object Object]"（键控复用/FLIP 形同虚设）。要求字段
                // 访问（item.id）或原始类型。
                if let Some(Expr::VarRef(name, _)) = &for_dir.key_expr {
                    if name == &for_dir.item_name
                        && infer_item_kind(&for_dir.iterable, typed, consts) == ItemKind::Object
                    {
                        diags.push(LintDiagnostic {
                            rule: "for-key".into(),
                            severity: LintSeverity::Warning,
                            message: FOR_KEY_OBJECT_WARNING.into(),
                            file: file_path.to_string(),
                            line: for_dir.span.line,
                            col: for_dir.span.col,
                        });
                    }
                }
                // Thread the loop binding's element type into the body so a
                // nested `@for cell in row` over a typed outer item is
                // classified too (clone — inner bindings must not leak out).
                let mut inner_typed = typed.clone();
                if let Some(ann) = typed
                    .get(match &for_dir.iterable {
                        Expr::VarRef(name, _) => name.as_str(),
                        _ => "",
                    })
                    .and_then(|ann| list_element_annotation(ann))
                {
                    inner_typed.insert(for_dir.item_name.as_str(), ann);
                }
                check_view_nodes(&for_dir.body, &inner_typed, consts, file_path, diags);
                if let Some(ref empty) = for_dir.empty_body {
                    check_view_nodes(empty, typed, consts, file_path, diags);
                }
            }
            ViewNode::Element(el) => {
                check_view_nodes(&el.children, typed, consts, file_path, diags);
            }
            ViewNode::IfDirective(if_dir) => {
                check_view_nodes(&if_dir.body, typed, consts, file_path, diags);
                for (_, body) in &if_dir.else_ifs {
                    check_view_nodes(body, typed, consts, file_path, diags);
                }
                if let Some(ref else_body) = if_dir.else_body {
                    check_view_nodes(else_body, typed, consts, file_path, diags);
                }
            }
            ViewNode::Text(_) | ViewNode::Interpolation(_) | ViewNode::SlotDirective { .. } => {}
        }
    }
}

/// Classify the ITEM type of a `@for` iterable expression.
fn infer_item_kind(
    expr: &Expr,
    typed: &FxHashMap<&str, &TypeAnnotation>,
    consts: &FxHashMap<&str, &Expr>,
) -> ItemKind {
    match expr {
        Expr::VarRef(name, _) => {
            if let Some(ann) = typed.get(name.as_str()) {
                match list_element_annotation(ann) {
                    Some(item_ann) => classify_annotation(item_ann),
                    None => ItemKind::Unknown,
                }
            } else if let Some(value) = consts.get(name.as_str()) {
                infer_item_kind(value, typed, consts)
            } else {
                ItemKind::Unknown
            }
        }
        // `@for x in [...]` — classify from the first element when known.
        Expr::ArrayLiteral { elements, .. } => elements
            .first()
            .map(|e| infer_element_kind(e, typed, consts))
            .unwrap_or(ItemKind::Unknown),
        // `@for v in {k: v, ...}` — classify from the first VALUE entry when
        // known (mirrors the array-literal heuristic; empty maps → Unknown).
        Expr::MapLiteral { entries, .. } => entries
            .first()
            .map(|(_, v)| infer_element_kind(v, typed, consts))
            .unwrap_or(ItemKind::Unknown),
        _ => ItemKind::Unknown,
    }
}

/// Classify a single array-literal element expression.
fn infer_element_kind(
    expr: &Expr,
    typed: &FxHashMap<&str, &TypeAnnotation>,
    consts: &FxHashMap<&str, &Expr>,
) -> ItemKind {
    match expr {
        Expr::Literal(lit, _) => {
            if matches!(lit, Literal::Null) {
                ItemKind::Unknown
            } else {
                ItemKind::Primitive
            }
        }
        Expr::StructLiteral { .. } | Expr::MapLiteral { .. } => ItemKind::Object,
        Expr::VarRef(name, _) => typed
            .get(name.as_str())
            .map(|ann| {
                if is_primitive_annotation(ann) {
                    ItemKind::Primitive
                } else {
                    ItemKind::Object
                }
            })
            .or_else(|| {
                consts
                    .get(name.as_str())
                    .map(|value| infer_element_kind(value, typed, consts))
            })
            .unwrap_or(ItemKind::Unknown),
        _ => ItemKind::Unknown,
    }
}

/// Extract the ELEMENT annotation of a list-like declared type
/// (`Array<T>`/`List<T>`/`Vec<T>` → `T`; `Map<K,V>` → `V` — map items are
/// the values). `Optional` unwraps. Non-list annotations → None.
fn list_element_annotation(ann: &TypeAnnotation) -> Option<&TypeAnnotation> {
    match ann {
        TypeAnnotation::Named {
            name, type_args, ..
        } if matches!(name.as_str(), "Array" | "List" | "Vec") => type_args.first(),
        TypeAnnotation::Named {
            name, type_args, ..
        } if matches!(name.as_str(), "Map" | "Dict" | "HashMap") => type_args.get(1),
        TypeAnnotation::Optional { base, .. } => list_element_annotation(base),
        _ => None,
    }
}

/// Classify a declared element annotation: primitives are identity-stable,
/// everything else (named structs/records, nested arrays/maps, unions
/// containing objects) is an object item.
fn classify_annotation(ann: &TypeAnnotation) -> ItemKind {
    if is_primitive_annotation(ann) {
        ItemKind::Primitive
    } else {
        ItemKind::Object
    }
}

/// Whether a declared type annotation is a primitive (identity-stable).
fn is_primitive_annotation(ann: &TypeAnnotation) -> bool {
    match ann {
        TypeAnnotation::Named { name, .. } => matches!(
            name.as_str(),
            "i8" | "i16"
                | "i32"
                | "i64"
                | "int"
                | "Int"
                | "integer"
                | "isize"
                | "usize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "f64"
                | "f32"
                | "float"
                | "Float"
                | "num"
                | "number"
                | "String"
                | "str"
                | "string"
                | "bool"
                | "Bool"
                | "boolean"
                | "void"
                | "Void"
                | "()"
                | "unit"
                | "null"
                | "Null"
                | "nil"
                | "None"
        ),
        TypeAnnotation::Optional { base, .. } => is_primitive_annotation(base),
        TypeAnnotation::Union { types, .. } => types.iter().all(is_primitive_annotation),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn parse(source: &str) -> Program {
        let tokens = crate::lexer::tokenize(source);
        let (program, errors) = parser::parse(tokens, source);
        assert!(errors.is_empty(), "fixture must parse cleanly: {errors:?}");
        program
    }

    fn warn_count(program: &Program, file: &str) -> usize {
        ForKey.check_ast(program, file).len()
    }

    #[test]
    fn object_items_without_key_warn() {
        let program = parse(
            r#"
struct Todo { id: i32, title: String }
component App {
    signal todos: Array<Todo> = [];
    view {
        @for t in todos { <div>{t.title}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 1);
    }

    #[test]
    fn object_items_with_key_do_not_warn() {
        let program = parse(
            r#"
struct Todo { id: i32, title: String }
component App {
    signal todos: Array<Todo> = [];
    view {
        @for t in todos key={t.id} { <div>{t.title}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn primitive_items_without_key_do_not_warn() {
        let program = parse(
            r#"
component App {
    signal items: Array<String> = [];
    view {
        @for item in items { <div>{item}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn map_items_without_key_warn() {
        let program = parse(
            r#"
component App {
    signal entries: Map<String, Todo> = {};
    view {
        @for t in entries { <div>{t.title}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 1);
    }

    #[test]
    fn primitive_map_values_do_not_warn() {
        let program = parse(
            r#"
component App {
    signal byName: Map<String, String> = {};
    view {
        @for v in byName { <div>{v}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn array_literal_of_objects_warns() {
        let program = parse(
            r#"
component App {
    view {
        @for t in [{ id: 1 }, { id: 2 }] { <div>row</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 1);
    }

    #[test]
    fn array_literal_of_primitives_does_not_warn() {
        let program = parse(
            r#"
component App {
    view {
        @for s in ["a", "b"] { <div>{s}</div> }
      }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn map_literal_of_primitives_does_not_warn() {
        let program = parse(
            r#"
component App {
    view {
        @for v in {a: 1, b: 2} { <div>{v}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn map_literal_of_objects_warns() {
        let program = parse(
            r#"
component App {
    view {
        @for v in {a: {id: 1}, b: {id: 2}} { <div>row</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 1);
    }

    #[test]
    fn unknown_iterable_type_does_not_warn() {
        let program = parse(
            r#"
fn getItems() -> Array<String> { return []; }

component App {
    view {
        @for item in getItems() { <div>{item}</div> }
    }
}
"#,
        );
        // A function call iterable has no statically known element type —
        // the rule must stay silent (conservative).
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn nested_lists_checked_recursively() {
        let program = parse(
            r#"
struct Row { id: i32 }
component App {
    signal matrix: Array<Array<Row>> = [];
    view {
        @for row in matrix {
            <div>@for cell in row { <span>{cell.id}</span> }</div>
        }
    }
}
"#,
        );
        // Outer item (Array<Row>) is an object; inner item (Row) is an
        // object too — both keyless → 2 warnings.
        assert_eq!(warn_count(&program, "test.omni"), 2);
    }

    #[test]
    fn object_items_with_key_in_nested_list_only_inner_warns() {
        let program = parse(
            r#"
struct Row { id: i32 }
component App {
    signal matrix: Array<Array<Row>> = [];
    view {
        // v4 4.2: 裸循环变量 key={row} 本身是对象 → 现在也警告（塌缩），
        // 改用字段访问 key={row[0].id} 以保持本测试原意（外层有合法 key）。
        @for row in matrix key={row[0].id} {
            <div>@for cell in row { <span>{cell.id}</span> }</div>
        }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 1);
    }

    #[test]
    fn object_key_expr_warns_with_field_hint() {
        // v4 perf HIGH: `@for item in items` + `key={item}`（对象项）——
        // 用户按既有 lint 建议补 key 恰好踩中 keyFn 的 String() 塌缩
        // （全部项 → "[object Object]"）。新增警告：key 表达式是对象
        // 本身时必须用字段访问（item.id）。
        let program = parse(
            r#"
struct Todo { id: i32, title: String }
component App {
    signal todos: Array<Todo> = [];
    view {
        @for t in todos key={t} { <div>{t.title}</div> }
    }
}
"#,
        );
        let diags = ForKey.check_ast(&program, "test.omni");
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("对象") && d.message.contains("字段")),
            "缺对象 key 警告：{:?}",
            diags
        );
    }

    #[test]
    fn primitive_items_with_bare_key_expr_do_not_warn() {
        // 原始类型项的 key={item} 不塌缩（String() 有区分度）——零警告。
        let program = parse(
            r#"
component App {
    signal items: Array<String> = [];
    view {
        @for item in items key={item} { <div>{item}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }

    #[test]
    fn unknown_iterable_with_bare_key_expr_does_not_warn() {
        // 未知迭代类型保持保守——即使 key 是裸循环变量也不警告。
        let program = parse(
            r#"
fn getItems() -> Array<String> { return []; }

component App {
    view {
        @for item in getItems() key={item} { <div>{item}</div> }
    }
}
"#,
        );
        assert_eq!(warn_count(&program, "test.omni"), 0);
    }
}
