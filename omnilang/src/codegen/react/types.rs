//! React user-type emission: structs / tagged unions / type aliases
//! (deferral #49 — Phase 5.3).
//!
//! The React target never declared user types: `struct Todo { ... }` /
//! `type Rating = Star { level: int } | Blank` were referenced by every
//! annotation (`items: Todo[]`, `describe(r: Rating)`) but had no
//! declaration in generated output, so tsc reported TS2304 (Cannot find
//! name 'Todo'/'ApiResult'/'Rating'), and the useState-generic guard
//! suppressed typed state for struct-typed signals — `useState([])` inferred
//! `never[]`, cascading into TS2322 on every setter call and TS2339 on every
//! field read (DataTable's RowItem rows).
//!
//! Strategy — INLINE declarations, no merged file: a generated `.ts`/`.tsx`
//! file is its own module, so `type Todo = {...}` can be declared at the top
//! of every file that references it. That keeps the store generator
//! self-contained (no cross-file import lines, no output-dir-relative paths,
//! no tsconfig include questions) at the cost of per-file duplication —
//! acceptable for generated output. Only types actually referenced by a
//! file's annotations are emitted (a dangling declaration trips the
//! no-unused-vars eslint gate), and the set is closed transitively (a
//! struct field may reference another user type).
//!
//! Mappings (JsTypeFormatter): `int`→`number`, `String`→`string`,
//! `[T]`/`Array<T>`→`T[]`, `Map<K,V>`→`Record<K,V>`, `Option<T>`→`T | null`,
//! user types as-is. Tagged unions become discriminated unions: variant
//! values emit `{ "_variant": "Star", "level": 5 }` and match arms dispatch
//! on `_v._variant === "Star"`, so `_variant` is the literal discriminator
//! and variant fields are only visible after narrowing (structurally safe —
//! the match emitter's `if (_v._variant === ...)` narrows exactly that).

use crate::codegen::shared::types::{FormatType, JsTypeFormatter};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashSet;

/// Names of every user-declared type (structs, tagged unions, aliases) in
/// the program — the "resolves in generated output" set. Builtin container
/// names (`Array`, `Map`, `Option`, ...) are never in this set.
pub(super) fn user_type_names(ir: &IRProgram) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for module in ir.modules.values() {
        names.extend(module.structs.keys().cloned());
        names.extend(module.enums.keys().cloned());
        names.extend(module.type_aliases.keys().cloned());
    }
    names
}

/// Record user type names referenced by a `TypeIR` annotation (recursively
/// through containers — `Array<RowItem>` records `RowItem`).
pub(super) fn collect_annotation_refs(
    ty: &TypeIR,
    user_names: &FxHashSet<String>,
    out: &mut FxHashSet<String>,
) {
    if let TypeKind::Named { name } = &ty.kind {
        if user_names.contains(name) {
            out.insert(name.clone());
        }
    }
    for t in &ty.type_args {
        collect_annotation_refs(t, user_names, out);
    }
}

/// Record user type names that appear as whole words in a pre-formatted TS
/// string (component event `param_type` — lowered to a TS string, not a
/// TypeIR). Word-boundary matching keeps `Rating` from matching `RatingX`.
pub(super) fn collect_ts_string_refs(
    s: &str,
    user_names: &FxHashSet<String>,
    out: &mut FxHashSet<String>,
) {
    if s.is_empty() {
        return;
    }
    let bytes = s.as_bytes();
    for name in user_names {
        if name.is_empty() {
            continue;
        }
        let mut search_from = 0;
        while let Some(pos) = s[search_from..].find(name.as_str()) {
            let at = search_from + pos;
            let before_ok = at == 0 || !ident_char(bytes[at - 1]);
            let after = at + name.len();
            let after_ok = after >= bytes.len() || !ident_char(bytes[after]);
            if before_ok && after_ok {
                out.insert(name.clone());
                break;
            }
            search_from = at + 1;
        }
    }
}

fn ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Close `referenced` transitively: a referenced type's own declaration may
/// reference further user types (a struct field typed with another struct).
/// Iterates to a fixed point over struct fields, union variant fields and
/// alias targets.
pub(super) fn close_type_refs(
    ir: &IRProgram,
    user_names: &FxHashSet<String>,
    referenced: &mut FxHashSet<String>,
) {
    let mut stack: Vec<String> = referenced.iter().cloned().collect();
    while let Some(name) = stack.pop() {
        let mut deps: FxHashSet<String> = FxHashSet::default();
        if let Some(decl) = resolve_declaration(ir, &name) {
            match decl {
                TypeDecl::Struct(s) => {
                    for f in &s.fields {
                        collect_annotation_refs(&f.r#type, user_names, &mut deps);
                    }
                }
                TypeDecl::Union(e) => {
                    for v in &e.variants {
                        for f in &v.fields {
                            collect_annotation_refs(&f.r#type, user_names, &mut deps);
                        }
                    }
                }
                TypeDecl::Alias(a) => collect_annotation_refs(&a.target, user_names, &mut deps),
            }
        }
        for dep in deps {
            if referenced.insert(dep.clone()) {
                stack.push(dep);
            }
        }
    }
}

enum TypeDecl<'a> {
    Struct(&'a StructIR),
    Union(&'a EnumIR),
    Alias(&'a TypeAliasIR),
}

/// Resolve a user type name to its (first) declaration in deterministic
/// order — sorted module key, then sorted name. Duplicate declarations
/// across modules (e.g. `ValidationResult` in types.omni + validation.omni)
/// resolve first-wins, mirroring the Flutter target's types.dart.
fn resolve_declaration<'a>(ir: &'a IRProgram, name: &str) -> Option<TypeDecl<'a>> {
    let mut module_keys: Vec<&String> = ir.modules.keys().collect();
    module_keys.sort();
    for key in module_keys {
        let module = &ir.modules[key];
        if let Some(s) = module.structs.get(name) {
            return Some(TypeDecl::Struct(s));
        }
        if let Some(e) = module.enums.get(name) {
            return Some(TypeDecl::Union(e));
        }
        if let Some(a) = module.type_aliases.get(name) {
            return Some(TypeDecl::Alias(a));
        }
    }
    None
}

/// Emit `type X = ...;` declarations for `referenced` (must already be
/// transitively closed) in deterministic (sorted) order. Emits nothing when
/// the set is empty — the no-unused-vars eslint gate forbids dangling
/// declarations.
pub(super) fn emit_type_declarations(
    w: &mut CodeWriter,
    ir: &IRProgram,
    referenced: &FxHashSet<String>,
) {
    if referenced.is_empty() {
        return;
    }
    let mut names: Vec<&String> = referenced.iter().collect();
    names.sort();
    for name in names {
        if let Some(decl) = resolve_declaration(ir, name) {
            match decl {
                TypeDecl::Struct(s) => emit_struct_type(w, s),
                TypeDecl::Union(e) => emit_union_type(w, e),
                TypeDecl::Alias(a) => emit_alias_type(w, a),
            }
        }
    }
}

fn emit_struct_type(w: &mut CodeWriter, s: &StructIR) {
    // Duplicate field names are dropped defensively (first wins) — a TS
    // object type cannot repeat a member.
    let mut fields: Vec<&FieldIR> = Vec::new();
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    for f in &s.fields {
        if seen.insert(f.name.as_str()) {
            fields.push(f);
        }
    }
    w.emit_fmt(format_args!("type {} = {{", s.name));
    w.indent();
    for f in fields {
        let ts = JsTypeFormatter.format_type(f.r#type.kind.as_str(), &f.r#type.type_args);
        w.emit_fmt(format_args!("{}: {};", f.name, ts));
    }
    w.dedent();
    w.emit("};");
}

fn emit_union_type(w: &mut CodeWriter, e: &EnumIR) {
    // Discriminated union: `_variant` carries the variant name as a string
    // LITERAL type — the match emitter's `if (_v._variant === "Star")`
    // narrows on exactly that, so variant fields become accessible.
    let members: Vec<String> = e
        .variants
        .iter()
        .map(|v| {
            let mut parts: Vec<String> = vec![format!("_variant: \"{}\"", v.name)];
            let mut seen: FxHashSet<&str> = FxHashSet::default();
            for f in &v.fields {
                if f.name == "_variant" || !seen.insert(f.name.as_str()) {
                    continue;
                }
                let ts = JsTypeFormatter.format_type(f.r#type.kind.as_str(), &f.r#type.type_args);
                parts.push(format!("{}: {}", f.name, ts));
            }
            format!("{{ {} }}", parts.join("; "))
        })
        .collect();
    w.emit_fmt(format_args!("type {} = {};", e.name, members.join(" | ")));
}

fn emit_alias_type(w: &mut CodeWriter, a: &TypeAliasIR) {
    let ts = JsTypeFormatter.format_type(a.target.kind.as_str(), &a.target.type_args);
    w.emit_fmt(format_args!("type {} = {};", a.name, ts));
}
