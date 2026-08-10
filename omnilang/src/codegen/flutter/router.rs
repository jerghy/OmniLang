//! Flutter GoRouter emission (spec §4.1/§4.2 Flutter 形态 + §4.3 floor 契约 +
//! §2.7 深链/§2.8 懒加载降级)。
//!
//! Collects `@route`/`@layout`/`@query` intent from the IR and emits:
//! - `router.dart`: `GoRouter(initialLocation: '/', routes: [...])` table —
//!   `GoRoute` (static/param paths), `ShellRoute` (layout + child routes),
//!   `redirect:` guards (component-fn translation), pathParameters/query
//!   parameter injection, honest degradation notes for lazy + deep links.
//! - `main.dart` (only when no `@theme` shell is present): `MaterialApp.router`
//!   wiring (spec §4.2 代码形态). `@theme` + `@route` coexistence emits a
//!   CodegenWarning and defers merging to Task 10.
//!
//! Zero-regression contract: no route/layout intent → no router.dart at all.
//! Deep-link platform manifests are NEVER generated — only note comments
//! (spec §6 边界 4); lazy is a mobile no-op with a compile note (spec §6
//! 边界 5). Degradation notes are always emitted — never silent (ADR-054).

use crate::codegen::{CodeWriter, CodegenWarning, CompiledFile};
use crate::ir::*;

use super::expr::format_expr_ir_flutter;
use super::stmt::generate_lib_fn_flutter;

/// A routed component: a `@route` (has a path) or a `@layout` (no path,
/// owns the subtree under its prefix).
struct RoutedComp<'a> {
    name: &'a str,
    comp: &'a ComponentIR,
    path: Option<String>,
    guard: Option<String>,
    redirect: Option<String>,
    lazy: bool,
    params: Vec<String>,
}

/// Emit router.dart (+ main.dart when no theme shell) for the program's
/// route/layout intent. Empty when the program has none — zero-regression.
pub(super) fn gen_router_files(
    ir: &IRProgram,
    theme_shell_present: bool,
) -> (Vec<CompiledFile>, Vec<CodegenWarning>) {
    let mut routed: Vec<RoutedComp> = Vec::new();
    for (name, comp) in ir.all_components() {
        // Layouts render without a path but own the subtree under their
        // prefix (ShellRoute branch).
        let intent: Option<&dyn RouteLike> = if let Some(r) = &comp.route_intent {
            Some(r)
        } else if let Some(l) = &comp.layout_intent {
            Some(l)
        } else {
            None
        };
        let Some(intent) = intent else { continue };
        routed.push(RoutedComp {
            name,
            comp,
            path: intent.path().map(|p| p.to_string()),
            guard: intent.guard().map(|g| g.to_string()),
            redirect: intent.redirect().map(|r| r.to_string()),
            lazy: intent.lazy().unwrap_or(false),
            params: intent
                .params()
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>(),
        });
    }
    if routed.is_empty() {
        return (Vec::new(), Vec::new());
    }
    // Deterministic emission: name-sorted.
    routed.sort_by(|a, b| a.name.cmp(b.name));
    let mut layouts: Vec<&RoutedComp> = routed.iter().filter(|r| r.path.is_none()).collect();
    // Longest prefix first so `owning_layout` finds the innermost owner.
    layouts.sort_by_key(|l| std::cmp::Reverse(prefix_of(l).len()));

    let mut warnings = Vec::new();
    if theme_shell_present {
        warnings.push(CodegenWarning::new(
            "@theme 外壳与 @route 路由并存——main.dart 由主题外壳发射（MaterialApp.home），GoRouter 未接线；合并方案见批次 2 Task 10",
        ));
    }

    let mut router = CodeWriter::new();
    emit_router(&mut router, &routed, &layouts, ir);
    emit_notes(&mut router);

    let mut files = vec![CompiledFile {
        path: "router.dart".to_string(),
        content: router.into_string(),
    }];
    if !theme_shell_present {
        let mut main = CodeWriter::new();
        emit_main(&mut main);
        files.push(CompiledFile {
            path: "main.dart".to_string(),
            content: main.into_string(),
        });
    }
    (files, warnings)
}

/// Route-like intent view shared by `RouteIntentIR` and `LayoutIntentIR`.
trait RouteLike {
    fn path(&self) -> Option<&str>;
    fn guard(&self) -> Option<&str>;
    fn redirect(&self) -> Option<&str>;
    fn lazy(&self) -> Option<bool>;
    fn params(&self) -> &[String];
}

impl RouteLike for RouteIntentIR {
    fn path(&self) -> Option<&str> {
        Some(&self.path)
    }

    fn guard(&self) -> Option<&str> {
        self.guard.as_deref()
    }

    fn redirect(&self) -> Option<&str> {
        self.redirect.as_deref()
    }

    fn lazy(&self) -> Option<bool> {
        self.lazy
    }

    fn params(&self) -> &[String] {
        &self.params
    }
}

impl RouteLike for LayoutIntentIR {
    fn path(&self) -> Option<&str> {
        None
    }

    fn guard(&self) -> Option<&str> {
        None
    }

    fn redirect(&self) -> Option<&str> {
        None
    }

    fn lazy(&self) -> Option<bool> {
        None
    }

    fn params(&self) -> &[String] {
        &[]
    }
}

/// The `router.dart` file: GoRouter table + top-level guard functions.
///
/// GoRouter matches the FULL URL per route — nested routes keep absolute
/// paths (`path: '/admin/users'`), there is no index-child concept.
fn emit_router(w: &mut CodeWriter, routed: &[RoutedComp], layouts: &[&RoutedComp], ir: &IRProgram) {
    // 路由表本身不引用 material 符号（守卫/redirect/builder 均为
    // 隐式类型闭包）——只引 go_router，避免 dart analyze unused_import。
    w.emit("// Generated by OmniLang — GoRouter 路由表（spec §4.2 代码形态）");
    w.emit("import 'package:go_router/go_router.dart';");
    for r in routed {
        w.emit_fmt(format_args!("import './{}.dart';", r.name));
    }
    w.emit("");
    emit_guard_fns(w, routed, ir);
    w.emit("final router = GoRouter(");
    w.indent();
    w.emit("initialLocation: '/',");
    w.emit("routes: [");
    w.indent();
    for r in routed {
        if r.path.is_none() || owning_layout(r, layouts).is_some() {
            continue; // layouts / layout-owned children — emitted below
        }
        emit_route_entry(w, r);
    }
    // Top-level layouts (not owned by another layout) — nested layouts are
    // emitted recursively inside their owner's `routes` (ShellRoute nesting).
    let mut top_layouts = layouts.to_vec();
    top_layouts.sort_by(|a, b| a.name.cmp(b.name));
    for l in top_layouts {
        if owning_layout(l, layouts).is_some() {
            continue;
        }
        emit_layout_entry(w, l, routed, layouts);
    }
    w.dedent();
    w.emit("],");
    w.dedent();
    w.emit(");");
    w.emit("");
}

/// Guard functions emitted as TOP-LEVEL Dart functions in router.dart
/// (`generate_lib_fn_flutter` — the guard is a component fn, spec §4.1
/// 组件 fn 翻译). Each unique name emitted once, name-sorted.
fn emit_guard_fns(w: &mut CodeWriter, routed: &[RoutedComp], ir: &IRProgram) {
    let mut names: Vec<&str> = routed.iter().filter_map(|r| r.guard.as_deref()).collect();
    names.sort_unstable();
    names.dedup();
    let mut emitted_any = false;
    for name in names {
        let func = routed
            .iter()
            .find_map(|r| r.comp.fns.iter().find(|f| f.name == name))
            .or_else(|| ir.all_functions().iter().copied().find(|f| f.name == name));
        let Some(func) = func else { continue };
        if !emitted_any {
            w.emit("// 守卫：组件 fn 翻译为顶层 Dart 函数（spec §4.1）");
            emitted_any = true;
        }
        generate_lib_fn_flutter(w, func);
        w.emit("");
    }
}

/// Deep-link + lazy + keep-alive degradation notes — spec §6 边界 4/5/7:
/// platform manifests, web deferred imports and StatefulShellRoute state
/// keeping are NOT generated (notes only, ADR-054: never silent).
fn emit_notes(w: &mut CodeWriter) {
    w.emit("/*");
    w.emit(" * 深链（spec §6 边界 4）：GoRouter web URL 同步天然，移动端平台清单未生成——");
    w.emit(" * 深链路由需在平台侧配置 intent-filter / universal links（诚实降级注记）。");
    w.emit(" * 懒加载（spec §6 边界 5）：deferred import（web）// 移动端单 bundle 无按需加载。");
    w.emit(" * 页面保挂载（spec §6 边界 7）：v0 用标准 ShellRoute 无 keep-alive——");
    w.emit(" * StatefulShellRoute.indexedStack 状态保持留待后续（诚实降级注记）。");
    w.emit(" */");
    w.emit("");
}

/// The layout owning a route or nested layout, if any (longest matching
/// prefix wins — `layouts` is sorted prefix-length-descending). `full` is
/// the route's path, or the layout's own prefix for layouts. A layout never
/// owns itself.
fn owning_layout<'a>(r: &RoutedComp, layouts: &[&'a RoutedComp<'a>]) -> Option<&'a RoutedComp<'a>> {
    let full = r.path.as_deref().unwrap_or_else(|| prefix_of(r));
    layouts.iter().copied().find(|l| {
        let is_self = r.path.is_none() && l.name == r.name;
        let prefix = prefix_of(l);
        !is_self && (full == prefix || full.starts_with(&format!("{prefix}/")))
    })
}

/// Children of a layout: routes AND nested layouts whose full path lives
/// under the layout prefix AND whose owning layout is THIS layout.
/// Name-sorted for determinism.
fn layout_children<'a>(
    layout: &RoutedComp,
    routed: &'a [RoutedComp<'a>],
    layouts: &[&'a RoutedComp<'a>],
) -> Vec<&'a RoutedComp<'a>> {
    let prefix = prefix_of(layout);
    let mut children: Vec<&RoutedComp> = routed
        .iter()
        .filter(|r| {
            let full = r.path.as_deref().unwrap_or_else(|| prefix_of(r));
            let under = full == prefix || full.starts_with(&format!("{prefix}/"));
            under && matches!(owning_layout(r, layouts), Some(o) if o.name == layout.name)
        })
        .collect();
    children.sort_by(|a, b| a.name.cmp(b.name));
    children
}

/// Emit one layout entry: `ShellRoute(builder: (_, __, child) =>
/// {Layout}(children: child), routes: [...])` — the layout receives its
/// slot content through the unnamed-slot `children` param (Widget? field).
/// Childless layouts fall back to a plain `GoRoute` (slot renders empty).
fn emit_layout_entry(
    w: &mut CodeWriter,
    l: &RoutedComp,
    routed: &[RoutedComp],
    layouts: &[&RoutedComp],
) {
    let children = layout_children(l, routed, layouts);
    if children.is_empty() {
        w.emit_fmt(format_args!(
            "GoRoute(path: '{}', builder: (_, __) => const {}()),",
            prefix_of(l),
            l.name
        ));
        return;
    }
    w.emit_fmt(format_args!("ShellRoute("));
    w.indent();
    w.emit_fmt(format_args!(
        "builder: (_, __, child) => {}(children: child),",
        l.name
    ));
    w.emit("routes: [");
    w.indent();
    for c in children {
        if c.path.is_none() {
            // Nested layout — recurse (ShellRoute nesting).
            emit_layout_entry(w, c, routed, layouts);
            continue;
        }
        emit_route_entry(w, c);
    }
    w.dedent();
    w.emit("],");
    w.dedent();
    w.emit("),");
}

fn emit_route_entry(w: &mut CodeWriter, r: &RoutedComp) {
    w.emit_fmt(format_args!(
        "GoRoute(path: '{}',",
        r.path.as_deref().unwrap_or("/")
    ));
    w.indent();
    if let Some(guard) = &r.guard {
        let redirect = r.redirect.as_deref().unwrap_or("/");
        w.emit_fmt(format_args!(
            "redirect: (_, __) => {guard}() ? null : '{redirect}',"
        ));
    }
    emit_entry_body(w, r);
    w.dedent();
    w.emit("),");
}

/// The `builder:` line — pathParameters/query injection (spec §4.2 参数注入
/// 形态). No injections → `builder: (_, __) => const XPage()` (widget
/// constructors are ALWAYS const in this codegen). Lazy is a mobile no-op
/// with a compile note (spec §6 边界 5).
fn emit_entry_body(w: &mut CodeWriter, r: &RoutedComp) {
    let mut injects: Vec<String> = Vec::new();
    for p in &r.params {
        injects.push(format!("{p}: s.pathParameters['{p}']!"));
    }
    for q in &r.comp.query_params {
        if !is_string_ty(&q.ty) {
            continue; // String-typed only (query params are URI strings)
        }
        let default = match &q.default {
            Some(d) => format_expr_ir_flutter(d),
            None => "''".to_string(),
        };
        injects.push(format!(
            "{}: s.uri.queryParameters['{}'] ?? {}",
            q.name, q.name, default
        ));
    }
    if injects.is_empty() {
        w.emit_fmt(format_args!("builder: (_, __) => const {}(),", r.name));
    } else {
        w.emit_fmt(format_args!(
            "builder: (_, s) => {}({}),",
            r.name,
            injects.join(", ")
        ));
    }
    if r.lazy {
        w.emit("// lazy: true — 移动端单 bundle 无按需加载（no-op 降级注记，spec §6 边界 5）");
    }
}

/// `main.dart` — MaterialApp.router wiring, mirroring the theme-shell shape
/// (emitted ONLY when no `@theme` shell exists; see gen_router_files).
fn emit_main(w: &mut CodeWriter) {
    w.emit("// Generated by OmniLang — app shell with GoRouter 接线（spec §4.2）");
    w.emit("import 'package:flutter/material.dart';");
    w.emit("import 'package:flutter_riverpod/flutter_riverpod.dart';");
    w.emit("import 'router.dart';");
    w.emit("");
    w.emit("void main() {");
    w.indent();
    w.emit("runApp(const ProviderScope(child: OmniApp()));");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class OmniApp extends ConsumerWidget {");
    w.indent();
    w.emit("const OmniApp({super.key});");
    w.emit("");
    w.emit("@override");
    w.emit("Widget build(BuildContext context, WidgetRef ref) {");
    w.indent();
    w.emit("return MaterialApp.router(routerConfig: router);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("");
}

fn prefix_of<'a>(r: &'a RoutedComp<'a>) -> &'a str {
    r.comp
        .layout_intent
        .as_ref()
        .map(|l| l.prefix.as_str())
        .unwrap_or("")
}

/// Query params are URI strings — only string-typed ones are injected.
fn is_string_ty(ty: &TypeIR) -> bool {
    matches!(ty.kind.as_str(), "string" | "String" | "str")
}
