//! React codegen: data-router emission (`App.tsx`) — Task 6 (spec 2026-08-09
//! §4.1/§4.2 + §2.7 深链).
//!
//! Collects every `route_intent`/`layout_intent`/`query_params` across the
//! whole `IRProgram` (`IRProgram::all_components()`) and emits one `App.tsx`
//! at the output root:
//!
//! - `createBrowserRouter` route table + `RouterProvider` (spec §4.2 code
//!   shape — nested `children` for layouts, guard `loader` redirect,
//!   route-level `lazy`)
//! - per-route wrapper components for param injection (`useLoaderData`) and
//!   query injection (`useSearchParams`): a module-scope `element={<X
//!   id={params.id}/>}` would fail tsc (TS2304 — `params` not in scope), so
//!   injection lives inside a wrapper that owns the hook call (spec §4.2
//!   loader 透传形态)
//! - deep links work for free — `createBrowserRouter` is history-based, the
//!   URL is the source of truth (spec §2.7), no extra wiring needed
//! - route-level `lazy` emits `{ path, lazy: () => import(...) }` with NO
//!   `element` (React Router v7 lazy-module contract) and maps the lazy
//!   module's named exports: plain lazy → `Component: m.X`; lazy + params/
//!   guard/query → the module self-contains `XLoader`/`XLazy` exports (see
//!   `gen_lazy_module_exports`) referenced as `Component: m.XLazy,
//!   loader: m.XLoader` — App.tsx never imports the lazy module statically
//!
//! No `@route`/`@layout` component in the program → no `App.tsx` is emitted
//! (zero regression for existing projects without routing — the demo's
//! hand-wired `src/main.tsx` keeps importing the App component generated
//! from `main.omni` itself).
//!
//! Naming: the router owns `App.tsx` at the output root.  A project that
//! declares `@route`/`@layout` AND has a component named `App` would collide
//! (router wins — the component is shadowed); routed projects are expected
//! to keep the router entry as the single app root.
//!
//! Layout children use RELATIVE paths (spec §4.2: `'users'` under the
//! `/admin` layout), derived by stripping the layout prefix from the route's
//! full path; a route whose path equals the layout prefix becomes an
//! `index: true` child.

use crate::codegen::react::expr::format_expr_ir_tsx;
use crate::codegen::react::relative_import_path;
use crate::codegen::{CodeWriter, CompiledFile};
use crate::ir::{ComponentIR, IRProgram, QueryParamIR, TypeIR};
use rustc_hash::FxHashSet;

/// One routed component: either a `@route` entry or a `@layout` entry.
/// Components are emitted in deterministic (name-sorted) order — the
/// `all_components()` map iterates in non-deterministic hash order, so
/// sorting is required for stable snapshots.
struct RoutedComp<'a> {
    name: &'a str,
    comp: &'a ComponentIR,
    /// Full path of the `@route` declaration (None for layouts).
    path: Option<String>,
    /// Guard function name (route only).
    guard: Option<String>,
    /// Redirect target when the guard fails (route only).
    redirect: Option<String>,
    /// `lazy: true` — component module loads on demand (route only).
    lazy: bool,
    /// `@route` params (route only).
    params: &'a [String],
}

/// Emit the router entry `App.tsx` (or nothing when the program has no
/// `@route`/`@layout` component).
pub(super) fn gen_router_files(ir: &IRProgram) -> Vec<CompiledFile> {
    let mut routed: Vec<RoutedComp> = ir
        .all_components()
        .iter()
        .filter_map(|(name, comp)| {
            if let Some(r) = &comp.route_intent {
                Some(RoutedComp {
                    name,
                    comp,
                    path: Some(r.path.clone()),
                    guard: r.guard.clone(),
                    redirect: r.redirect.clone(),
                    lazy: r.lazy.unwrap_or(false),
                    params: &r.params,
                })
            } else if comp.layout_intent.is_some() {
                Some(RoutedComp {
                    name,
                    comp,
                    path: None,
                    guard: None,
                    redirect: None,
                    lazy: false,
                    params: &[],
                })
            } else {
                None
            }
        })
        .collect();
    if routed.is_empty() {
        return Vec::new();
    }
    routed.sort_by(|a, b| a.name.cmp(b.name));

    // Layouts in prefix-length-descending order (longest prefix wins when
    // several layouts could own a route); ties broken by name.
    let mut layouts: Vec<&RoutedComp> = routed.iter().filter(|r| r.path.is_none()).collect();
    layouts.sort_by(|a, b| {
        let pa = prefix_of(a).len();
        let pb = prefix_of(b).len();
        pb.cmp(&pa).then_with(|| a.name.cmp(b.name))
    });

    let mut w = CodeWriter::with_capacity(2048);
    emit_imports(&mut w, &routed);
    emit_wrappers(&mut w, &routed);
    emit_router(&mut w, &routed, &layouts);
    emit_app(&mut w, &routed);
    vec![CompiledFile {
        path: "App.tsx".to_string(),
        content: w.into_string(),
    }]
}

fn prefix_of<'a>(r: &RoutedComp<'a>) -> &'a str {
    r.comp
        .layout_intent
        .as_ref()
        .map(|l| l.prefix.as_str())
        .unwrap_or("")
}

/// Imports: react-router symbols (only what is actually used) + one named
/// import per routed component (skipped for lazy routes — they import
/// themselves via `lazy: () => import(...)`).
fn emit_imports(w: &mut CodeWriter, routed: &[RoutedComp]) {
    let has_guard = routed.iter().any(|r| r.guard.is_some());
    let has_param_wrapper = routed.iter().any(|r| !r.params.is_empty() && !r.lazy);
    let has_query_wrapper = routed.iter().any(|r| !r.lazy && has_injected_query(r.comp));
    let mut router_imports = vec!["createBrowserRouter", "RouterProvider"];
    if has_guard {
        router_imports.push("redirect");
    }
    if has_param_wrapper {
        router_imports.push("useLoaderData");
    }
    if has_query_wrapper {
        router_imports.push("useSearchParams");
    }
    w.emit_fmt(format_args!(
        "import {{ {} }} from 'react-router';",
        router_imports.join(", ")
    ));
    if routed.iter().any(|r| r.lazy) {
        w.emit("import { Suspense } from 'react';");
    }
    w.emit("");

    for r in routed.iter().filter(|r| !r.lazy) {
        let path = relative_import_path(None, r.comp.source_file.as_deref(), r.name);
        let symbols = match r.guard.as_deref() {
            Some(g) => format!("{}, {g}", r.name),
            None => r.name.to_string(),
        };
        w.emit_fmt(format_args!("import {{ {symbols} }} from '{path}';"));
    }
    w.emit("");
}

/// Whether any of the component's `@query` params are injectable: only
/// String-typed params are wired through `useSearchParams` — the URL yields
/// strings, so non-String params would need a parse step that is the user's
/// job (documented limitation; non-String `@query` params stay in the Props
/// interface as optional but are not injected).
pub(super) fn has_injected_query(comp: &ComponentIR) -> bool {
    comp.query_params.iter().any(|q| is_string_type(&q.ty))
}

fn is_string_type(ty: &TypeIR) -> bool {
    matches!(ty.kind.as_str(), "string" | "String" | "str")
}

/// The injectable String-typed `@query` params of a routed component.
fn injected_query(comp: &ComponentIR) -> Vec<&QueryParamIR> {
    comp.query_params
        .iter()
        .filter(|q| is_string_type(&q.ty))
        .collect()
}

/// Param/query wrapper components — one per routed component that declares
/// `@route` params or String `@query` params (skipped for lazy routes; the
/// lazy module must stay self-contained).
fn emit_wrappers(w: &mut CodeWriter, routed: &[RoutedComp]) {
    for r in routed {
        if r.lazy {
            continue;
        }
        let query = injected_query(r.comp);
        if r.params.is_empty() && query.is_empty() {
            continue;
        }
        let wrapper = format!("{}Route", r.name);
        w.emit_fmt(format_args!("function {wrapper}() {{"));
        w.indent();
        if !r.params.is_empty() {
            // Route params are always path strings — the cast must name a
            // concrete type (`{ id: string }`), a bare `{ id }` is a TS
            // syntax error in type position.
            let type_entries = r
                .params
                .iter()
                .map(|p| format!("{p}: string"))
                .collect::<Vec<_>>()
                .join("; ");
            w.emit_fmt(format_args!(
                "const params = useLoaderData() as {{ {} }};",
                type_entries
            ));
        }
        if !query.is_empty() {
            w.emit("const [searchParams] = useSearchParams();");
        }
        // Prop spread: params first, then query (defaults as fallbacks).
        let mut props: Vec<String> = Vec::new();
        for p in r.params {
            props.push(format!("{p}={{params.{p}}}"));
        }
        for q in query {
            // Defaults formatted as TSX literal expressions (e.g. `"all"`),
            // not the param name — `filter` would be undefined in wrapper scope.
            let value = match &q.default {
                Some(d) => format!(
                    "searchParams.get('{}') ?? {}",
                    q.name,
                    format_expr_ir_tsx(d, &FxHashSet::default())
                ),
                None => format!("searchParams.get('{}') ?? undefined", q.name),
            };
            props.push(format!("{}={{{value}}}", q.name));
        }
        w.emit_fmt(format_args!("return <{} {}/>;", r.name, props.join(" ")));
        w.dedent();
        w.emit("}");
        w.emit("");
    }
}

/// Self-contained lazy module exports — emitted into the routed component's
/// OWN module (not App.tsx) for `@route lazy: true` components that need
/// wiring beyond the bare component (honest-tsc gate: TS2304/TS2322 — the
/// lazy object in App.tsx may only reference names that module itself
/// exports).
///
/// - `XLoader` — guard check and/or route-param source: `export function
///   XLoader({ params }: { params: Readonly<Record<string, string |
///   undefined>> }) { return G() ? params : redirect('/R'); }`. The params
///   annotation is structurally identical to react-router's `Params`
///   (mapped type over string collapses to an index signature) without an
///   import — `Readonly`/`Record` are lib globals, and importing `Params`
///   would risk TS1484 under isolatedModules. `Readonly<Record<...>>` is
///   required over bare `Record<...>`: `Params` is readonly, and a readonly
///   index signature is not assignable to a mutable one at the
///   `loader: m.XLoader` check.
/// - `XLazy` — param/query injection wrapper mirroring the non-lazy
///   `emit_wrappers` shape (hook calls must live inside the module that
///   imports them; a module-scope `element={<X params={params}/>}` would
///   fail tsc on out-of-scope names).
///
/// Emits nothing when the component is not a lazy route, or when it has no
/// params/guard/injectable query (plain lazy modules are self-contained by
/// construction — `Component: m.X` suffices).
pub(super) fn gen_lazy_module_exports(w: &mut CodeWriter, name: &str, comp: &ComponentIR) {
    let Some(route) = comp.route_intent.as_ref() else {
        return;
    };
    if !route.lazy.unwrap_or(false) {
        return;
    }
    if route.params.is_empty() && route.guard.is_none() && !has_injected_query(comp) {
        return;
    }
    // Loader: guard redirect + param source (module-scope export — the
    // loader's guard fn reference is resolved inside the module).
    if route.guard.is_some() || !route.params.is_empty() {
        let redirect = route.redirect.as_deref().unwrap_or("/");
        let guard = route.guard.as_deref().unwrap_or("");
        let (sig, body) = if route.params.is_empty() {
            ("()", format!("{guard}() ? null : redirect('{redirect}')"))
        } else if let Some(g) = route.guard.as_deref() {
            (
                "({ params }: { params: Readonly<Record<string, string | undefined>> })",
                format!("{g}() ? params : redirect('{redirect}')"),
            )
        } else {
            (
                "({ params }: { params: Readonly<Record<string, string | undefined>> })",
                "params".to_string(),
            )
        };
        w.emit_fmt(format_args!("export function {name}Loader{sig} {{"));
        w.indent();
        w.emit_fmt(format_args!("return {body};"));
        w.dedent();
        w.emit("}");
        w.emit("");
    }
    // Injection wrapper: mirrors emit_wrappers (useLoaderData cast + search
    // params with default fallbacks).
    if !route.params.is_empty() || has_injected_query(comp) {
        let query = injected_query(comp);
        w.emit_fmt(format_args!("export function {name}Lazy() {{"));
        w.indent();
        if !route.params.is_empty() {
            let type_entries = route
                .params
                .iter()
                .map(|p| format!("{p}: string"))
                .collect::<Vec<_>>()
                .join("; ");
            w.emit_fmt(format_args!(
                "const params = useLoaderData() as {{ {} }};",
                type_entries
            ));
        }
        if !query.is_empty() {
            w.emit("const [searchParams] = useSearchParams();");
        }
        let mut props: Vec<String> = Vec::new();
        for p in &route.params {
            props.push(format!("{p}={{params.{p}}}"));
        }
        for q in query {
            let value = match &q.default {
                Some(d) => format!(
                    "searchParams.get('{}') ?? {}",
                    q.name,
                    format_expr_ir_tsx(d, &FxHashSet::default())
                ),
                None => format!("searchParams.get('{}') ?? undefined", q.name),
            };
            props.push(format!("{}={{{value}}}", q.name));
        }
        w.emit_fmt(format_args!("return <{name} {}/>;", props.join(" ")));
        w.dedent();
        w.emit("}");
    }
}

/// The `createBrowserRouter` route table: top-level route entries + layout
/// entries with nested relative-path `children` (routes whose full path
/// lives under the layout prefix), index children for path == prefix.
fn emit_router(w: &mut CodeWriter, routed: &[RoutedComp], layouts: &[&RoutedComp]) {
    w.emit("const router = createBrowserRouter([");
    w.indent();
    for r in routed {
        if r.path.is_none() {
            continue; // layout — emitted with its children below
        }
        if owning_layout(r, layouts).is_some() {
            continue; // emitted inside its layout's `children`
        }
        emit_route_entry(w, r);
    }
    let mut layout_order = layouts.to_vec();
    layout_order.sort_by(|a, b| a.name.cmp(b.name));
    for l in layout_order {
        emit_layout_entry(w, l, routed);
    }
    w.dedent();
    w.emit("]);");
    w.emit("");
}

/// The layout owning a route, if any (longest matching prefix wins —
/// `layouts` is already sorted prefix-length-descending).
fn owning_layout<'a>(r: &RoutedComp, layouts: &[&'a RoutedComp<'a>]) -> Option<&'a RoutedComp<'a>> {
    let full = r.path.as_deref()?;
    layouts.iter().copied().find(|l| {
        let prefix = prefix_of(l);
        full == prefix || full.starts_with(&format!("{prefix}/"))
    })
}

/// Children of a layout: routes whose full path starts with the layout
/// prefix (name-sorted for determinism).
fn layout_children<'a>(
    layout: &RoutedComp,
    routed: &'a [RoutedComp<'a>],
) -> Vec<&'a RoutedComp<'a>> {
    let prefix = prefix_of(layout);
    let mut children: Vec<&RoutedComp> = routed
        .iter()
        .filter(|r| {
            let Some(full) = r.path.as_deref() else {
                return false;
            };
            full == prefix || full.starts_with(&format!("{prefix}/"))
        })
        .collect();
    children.sort_by(|a, b| a.name.cmp(b.name));
    children
}

fn emit_layout_entry(w: &mut CodeWriter, l: &RoutedComp, routed: &[RoutedComp]) {
    let prefix = prefix_of(l);
    let children = layout_children(l, routed);
    if children.is_empty() {
        w.emit_fmt(format_args!(
            "{{ path: '{prefix}', element: {} }},",
            element_for(l)
        ));
        return;
    }
    w.emit("{");
    w.indent();
    w.emit_fmt(format_args!("path: '{prefix}',"));
    w.emit_fmt(format_args!("element: {},", element_for(l)));
    w.emit("children: [");
    w.indent();
    for c in children {
        let rest = c.path.as_deref().unwrap_or("");
        let path_line = if rest == prefix {
            "index: true".to_string()
        } else {
            let rel = rest.strip_prefix(&format!("{prefix}/")).unwrap_or(rest);
            format!("path: '{rel}'")
        };
        emit_entry_body(w, c, &path_line);
    }
    w.dedent();
    w.emit("],");
    w.dedent();
    w.emit("},");
}

fn emit_route_entry(w: &mut CodeWriter, r: &RoutedComp) {
    emit_entry_body(
        w,
        r,
        &format!("path: '{}'", r.path.as_deref().unwrap_or("/")),
    );
}

fn emit_entry_body(w: &mut CodeWriter, r: &RoutedComp, path_line: &str) {
    let loader = loader_for(r);
    let lazy = lazy_for(r);
    if lazy.is_some() {
        // Lazy routes carry NO element (spec §4.2 `{ path, lazy }`) — the
        // component module is only reachable via the dynamic import, so a
        // static `element: <X />` would reference an unimported name
        // (honest-tsc gate: TS2304). Params/guard/query wiring lives inside
        // the lazy module itself (`XLoader`/`XLazy` exports — see
        // `gen_lazy_module_exports`), referenced through the lazy object.
        w.emit_fmt(format_args!(
            "{{ {path_line}, {} }},",
            lazy.as_deref().unwrap_or("")
        ));
        return;
    }
    if loader.is_none() {
        w.emit_fmt(format_args!(
            "{{ {path_line}, element: {} }},",
            element_for(r)
        ));
        return;
    }
    w.emit("{");
    w.indent();
    w.emit_fmt(format_args!("{path_line},"));
    if let Some(l) = loader {
        w.emit_fmt(format_args!("{l},"));
    }
    w.emit_fmt(format_args!("element: {},", element_for(r)));
    w.dedent();
    w.emit("},");
}

/// `element: <X />` or `<XRoute />` (wrapper when a param/query wrapper was
/// emitted).
fn element_for(r: &RoutedComp) -> String {
    let wrapped = !r.lazy && (!r.params.is_empty() || has_injected_query(r.comp));
    if wrapped {
        format!("<{}Route />", r.name)
    } else {
        format!("<{} />", r.name)
    }
}

/// Guard loader shape (spec §4.2): `loader: () => (isAuthorized() ? null :
/// redirect('/login'))`; with route params the loader doubles as the param
/// source — `loader: ({ params }) => (isAuthorized() ? params :
/// redirect('/login'))`.
///
/// Lazy routes never get a loader here: the guard function lives in the lazy
/// module, so App.tsx has no name to reference (honest-tsc gate: TS2304);
/// `gen_lazy_module_exports` emits the loader inside the module and the
/// `lazy:` object wires it via `loader: m.XLoader`.
fn loader_for(r: &RoutedComp) -> Option<String> {
    if r.lazy {
        return None;
    }
    let guard = r.guard.as_deref()?;
    let redirect = r.redirect.as_deref().unwrap_or("/");
    let args = if r.params.is_empty() {
        "()"
    } else {
        "({ params })"
    };
    if r.params.is_empty() {
        Some(format!(
            "loader: {args} => ({guard}() ? null : redirect('{redirect}'))"
        ))
    } else {
        Some(format!(
            "loader: {args} => ({guard}() ? params : redirect('{redirect}'))"
        ))
    }
}

/// `lazy: () => import('./components/X').then(m => ({ ... }))` — maps the
/// generated named exports to the lazy-module contract. The `Component`
/// value depends on what the lazy module exports (see
/// `gen_lazy_module_exports`):
///
/// - plain lazy → `Component: m.X` (the component's own named export)
/// - lazy + params/query → `Component: m.XLazy` (param/query injection needs
///   a wrapper that owns the hook calls; the wrapper lives in the lazy
///   module so App.tsx never imports it statically)
/// - lazy + guard → `loader: m.XLoader` (the guard loader lives in the lazy
///   module — App.tsx has no `isAdmin` name to reference)
fn lazy_for(r: &RoutedComp) -> Option<String> {
    if !r.lazy {
        return None;
    }
    let path = relative_import_path(None, r.comp.source_file.as_deref(), r.name);
    let needs_wrapper = !r.params.is_empty() || has_injected_query(r.comp);
    let component = if needs_wrapper {
        format!("Component: m.{}Lazy", r.name)
    } else {
        format!("Component: m.{}", r.name)
    };
    let loader = if !r.params.is_empty() || r.guard.is_some() {
        format!(", loader: m.{}Loader", r.name)
    } else {
        String::new()
    };
    Some(format!(
        "lazy: () => import('{path}').then(m => ({{ {component}{loader} }}))"
    ))
}

/// `export default function App() { return <RouterProvider router={router}
/// />; }` — plus a named `App` export: the react-test-app template's
/// hand-owned `src/main.tsx` imports `{ App }` (named).  Suspense wraps the
/// provider when any route is lazy.
fn emit_app(w: &mut CodeWriter, routed: &[RoutedComp]) {
    w.emit("export default function App() {");
    w.indent();
    let body = if routed.iter().any(|r| r.lazy) {
        "<Suspense fallback={null}><RouterProvider router={router} /></Suspense>"
    } else {
        "<RouterProvider router={router} />"
    };
    w.emit_fmt(format_args!("return {body};"));
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("export { App };");
}
