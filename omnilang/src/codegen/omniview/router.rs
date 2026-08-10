//! OmniView hash router emission (spec §4.2 OmniView 形态 + §4.3 floor 契约 +
//! §2.7 深链/§2.8 懒加载降级)。
//!
//! Collects `@route`/`@layout`/`@query` intent from the IR and emits a
//! zero-dependency hash router into the generated JS:
//! - `__omni_routes` — router table: synthetic root entry `{ pattern: "/" }`
//!   first, then name-sorted layouts (`isLayout: true`) and routes.
//! - `__omni_apply()` — hashchange listener + initial pass: first-match wins,
//!   guard fn call (false → `location.hash = redirect`, default `/`), param
//!   segment + URLSearchParams query injection into writable signals
//!   (`__omni_subs` registry), layout chain walk, `<@slot/>` child moves,
//!   display toggling.
//!
//! Static-inline architecture: ALL routed pages exist in the DOM as hidden
//! `data-omni-route` containers (see store.rs / view/mod.rs) — the router only
//! toggles visibility, no rebinding, no runtime component factory.
//!
//! Honest degradation (ADR-054 — never silent):
//! - Lazy is a no-op: all pages are statically inlined (single document), per
//!   entry compile note only (spec §6 边界 5).
//! - Deep links are `#/path` hash URLs — no history.pushState (runtime
//!   environment limit, spec §4.2 注).
//! - `@load` fires for ALL routed pages at mount (static-inline consequence).
//!
//! Zero-regression contract: no route/layout intent → no router JS at all
//! (`has_routing` gate).

use crate::ir::*;

/// Whether the program carries any route/layout intent — gate for the whole
/// router emission (zero-regression: no intent → no router code).
pub(super) fn has_routing(ir: &IRProgram) -> bool {
    ir.all_components()
        .values()
        .any(|c| c.route_intent.is_some() || c.layout_intent.is_some())
}

/// A routed component: a `@route` (has a path) or a `@layout` (no path,
/// owns the subtree under its prefix).
#[derive(Clone)]
struct RoutedComp<'a> {
    name: &'a str,
    comp: &'a ComponentIR,
    pattern: String,
    guard: Option<String>,
    redirect: Option<String>,
    lazy: bool,
    params: Vec<String>,
}

/// Emit the hash router JS (route table + runtime + listener + degradation
/// notes) into `js`. Called right before the IIFE closes — the emitted code
/// shares the IIFE scope with `__omni_subs` (runtime.rs).
pub(super) fn emit_router_js(
    js: &mut String,
    ir: &IRProgram,
    root_name: &str,
    warnings: &mut Vec<String>,
) {
    let mut routed: Vec<RoutedComp> = Vec::new();
    for (name, comp) in ir.all_components() {
        // Layouts render without a path but own the subtree under their prefix.
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
            pattern: intent.pattern().to_string(),
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
    // Deterministic emission: name-sorted.
    routed.sort_by(|a, b| a.name.cmp(b.name));

    // Root special-cases: the root container doubles as the "/" route page.
    let mut root_entry: Option<RoutedComp> = None;
    let mut routed: Vec<RoutedComp> = routed
        .into_iter()
        .filter(|r| {
            if r.name == root_name {
                root_entry = Some(RoutedComp { ..r.clone() });
                false
            } else {
                true
            }
        })
        .collect();
    // A routed root: its @route path must be "/" (path → redirect target);
    // a root @layout makes no sense (nothing nests under the root page).
    let mut root_guard: Option<String> = None;
    let mut root_redirect: Option<String> = None;
    if let Some(r) = &root_entry {
        if r.comp.route_intent.is_some() {
            if r.pattern == "/" {
                root_guard = r.guard.clone();
                root_redirect = r.redirect.clone();
            } else {
                warnings.push(format!(
                    "OmniView: 根组件 `{root_name}` 的 @route 路径 `{}` ≠ \"/\" ——根容器固定为 \"/\" 路由页，声明被忽略",
                    r.pattern
                ));
            }
        } else {
            warnings.push(format!(
                "OmniView: 根组件 `{root_name}` 声明 @layout——根容器固定为 \"/\" 路由页，布局意图被忽略"
            ));
        }
    }

    // Guard presence check: the guard must be a component fn or a public
    // module fn — otherwise the emitted identifier would be undefined at
    // runtime. Missing guards degrade to `null` with an honest warning.
    for r in &mut routed {
        if let Some(g) = r.guard.clone() {
            let exists = r.comp.fns.iter().any(|f| f.name == g)
                || ir.all_functions().iter().any(|f| f.name == g);
            if !exists {
                warnings.push(format!(
                    "OmniView: 路由 `{}` 的守卫 fn `{g}` 未定义——守卫降级为 null（不拦截）",
                    r.pattern
                ));
                r.guard = None;
            }
        }
    }

    // Longest prefix first so `owning_layout` finds the innermost owner.
    let mut layouts: Vec<&RoutedComp> = routed
        .iter()
        .filter(|r| r.comp.layout_intent.is_some())
        .collect();
    layouts.sort_by_key(|l| std::cmp::Reverse(prefix_of(l).len()));

    let mut entries: Vec<&RoutedComp> = routed.iter().collect();
    entries.sort_by(|a, b| a.name.cmp(b.name));

    js.push_str("\n/// ── Hash Router (Task 8) ──\n");
    js.push_str("const __omni_routes = [\n");
    // Synthetic root entry first: pattern "/" always matches. The guard is a
    // direct identifier — component/module fns are IIFE-local `function`
    // declarations (hoisted), so `__omni_apply` calls `hit.guard()` directly.
    {
        let guard = match &root_guard {
            Some(g) => format!("{root_name}_{g}"),
            None => "null".to_string(),
        };
        let redirect = match &root_redirect {
            Some(r) => format!("\"{}\"", r),
            None => "null".to_string(),
        };
        js.push_str(&format!(
            "  {{ pattern: \"/\", comp: \"{root_name}\", params: [], query: [], guard: {guard}, redirect: {redirect}, layout: null, isLayout: false }},\n"
        ));
    }
    for r in entries {
        if r.comp.layout_intent.is_some() {
            let layout = owning_layout(r, &layouts)
                .map(|l| format!("\"{}\"", prefix_of(l)))
                .unwrap_or_else(|| "null".to_string());
            js.push_str(&format!(
                "  {{ pattern: \"{}\", comp: \"{}\", params: [], query: [], guard: null, redirect: null, layout: {layout}, isLayout: true }},\n",
                prefix_of(r),
                r.name
            ));
            continue;
        }
        let guard = match &r.guard {
            Some(g) => format!("{}_{}", r.name, g),
            None => "null".to_string(),
        };
        let redirect = match &r.redirect {
            Some(r) => format!("\"{}\"", r),
            None => "null".to_string(),
        };
        let layout = owning_layout(r, &layouts)
            .map(|l| format!("\"{}\"", prefix_of(l)))
            .unwrap_or_else(|| "null".to_string());
        let params = r
            .params
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        let query = r
            .comp
            .query_params
            .iter()
            .filter(|q| is_string_ty(&q.ty))
            .map(|q| format!("\"{}\"", q.name))
            .collect::<Vec<_>>()
            .join(", ");
        js.push_str(&format!(
            "  {{ pattern: \"{}\", comp: \"{}\", params: [{}], query: [{}], guard: {guard}, redirect: {redirect}, layout: {layout}, isLayout: false }},\n",
            r.pattern,
            r.name,
            params,
            query
        ));
    }
    js.push_str("];\n\n");

    js.push_str(EMIT_ROUTER_RUNTIME);

    // Degradation notes — spec §6 边界 4/5 + static-inline @load consequence.
    // ADR-054: notes are always emitted, never silent.
    let mut lazy_names: Vec<&str> = routed.iter().filter(|r| r.lazy).map(|r| r.name).collect();
    lazy_names.sort_unstable();
    lazy_names.dedup();
    js.push_str("/*\n");
    js.push_str(" * 深链（spec §6 边界 4）：hash 路由 `#/path` 可直达可分享——运行环境不支持\n");
    js.push_str(" * history.pushState（spec §4.2 注），URL 形态为 `#/path`（诚实降级注记）。\n");
    js.push_str(" * 懒加载（spec §6 边界 5）：OmniView 静态内联架构——全部路由页同文档内联，\n");
    js.push_str(" * 动态 import 无按需加载（no-op 降级注记）。\n");
    if !lazy_names.is_empty() {
        js.push_str(&format!(
            " *   声明 lazy 的路由：{}\n",
            lazy_names.join(", ")
        ));
    }
    js.push_str(" * @load 时机（静态内联后果）：所有路由页的 @load 在挂载时全部触发——\n");
    js.push_str(" * 页面切换不重新触发（诚实降级注记）。\n");
    js.push_str(" */\n");
    js.push_str("window.addEventListener(\"hashchange\", __omni_apply);\n");
    js.push_str("__omni_apply();\n");
}

/// The zero-dependency router runtime, verbatim. Shares the IIFE scope with
/// `__omni_subs` (runtime.rs) so param/query injection writes the writable
/// route signals emitted by store.rs.
const EMIT_ROUTER_RUNTIME: &str = r##"const __omni_currentPath = () => {
  const h = location.hash || "";
  const noHash = h.startsWith("#") ? h.slice(1) : h;
  const q = noHash.indexOf("?");
  const p = q >= 0 ? noHash.slice(0, q) : noHash;
  return p === "" ? "/" : p;
};
const __omni_match = (pattern, path) => {
  if (pattern === path) return true;
  const ps = pattern.split("/").filter((s) => s !== "");
  const as = path.split("/").filter((s) => s !== "");
  if (ps.length !== as.length) return false;
  for (let i = 0; i < ps.length; i++) {
    if (ps[i].startsWith(":")) continue;
    if (ps[i] !== as[i]) return false;
  }
  return true;
};
const __omni_paramValues = (pattern, path) => {
  const out = {};
  const ps = pattern.split("/").filter((s) => s !== "");
  const as = path.split("/").filter((s) => s !== "");
  for (let i = 0; i < ps.length; i++) {
    if (!ps[i].startsWith(":")) continue;
    try {
      out[ps[i].slice(1)] = decodeURIComponent(as[i]);
    } catch (e) {
      out[ps[i].slice(1)] = as[i];
    }
  }
  return out;
};
const __omni_queryValues = () => {
  const h = location.hash || "";
  const q = h.indexOf("?");
  if (q < 0) return {};
  const sp = new URLSearchParams(h.slice(q + 1));
  const out = {};
  for (const [k, v] of sp.entries()) out[k] = v;
  return out;
};
const __omni_layoutByPrefix = (prefix) => {
  for (let i = 0; i < __omni_routes.length; i++) {
    const e = __omni_routes[i];
    if (e.isLayout && e.pattern === prefix) return e;
  }
  return null;
};
const __omni_apply = () => {
  const path = __omni_currentPath();
  let hit = null;
  for (let i = 0; i < __omni_routes.length; i++) {
    if (__omni_routes[i].isLayout) continue;
    if (__omni_match(__omni_routes[i].pattern, path)) { hit = __omni_routes[i]; break; }
  }
  if (!hit) hit = __omni_routes[0];
  if (hit.guard) {
    try {
      if (!hit.guard()) {
        location.hash = "#" + (hit.redirect || "/");
        return;
      }
    } catch (e) {
      __omni_reportError("warn:route:guard", e);
    }
  }
  const params = __omni_paramValues(hit.pattern, path);
  for (const p of hit.params) {
    const key = hit.comp + "_" + p;
    if (__omni_subs.has(key) && params[p] !== undefined) __omni_subs.get(key).set(params[p]);
  }
  const query = __omni_queryValues();
  for (const q of hit.query) {
    const key = hit.comp + "_" + q;
    if (__omni_subs.has(key) && query[q] !== undefined) __omni_subs.get(key).set(query[q]);
  }
  const chain = [];
  let cur = hit.pattern;
  chain.push(cur);
  if (hit.layout) {
    let l = __omni_layoutByPrefix(hit.layout);
    while (l) {
      chain.push(l.pattern);
      const slot = document.querySelector('[data-omni-route="' + l.pattern + '"] slot:not([name])');
      const childEl = document.querySelector('[data-omni-route="' + cur + '"]');
      if (slot && childEl && childEl.parentElement !== slot) slot.appendChild(childEl);
      cur = l.pattern;
      l = l.layout ? __omni_layoutByPrefix(l.layout) : null;
    }
  }
  for (let i = 0; i < __omni_routes.length; i++) {
    const e = __omni_routes[i];
    if (e.isLayout) continue;
    const el = document.querySelector('[data-omni-route="' + e.pattern + '"]');
    if (!el) continue;
    el.style.display = chain.indexOf(e.pattern) >= 0 ? "" : "none";
  }
};
"##;

/// Route-like intent view shared by `RouteIntentIR` and `LayoutIntentIR`.
trait RouteLike {
    fn pattern(&self) -> &str;
    fn guard(&self) -> Option<&str>;
    fn redirect(&self) -> Option<&str>;
    fn lazy(&self) -> Option<bool>;
    fn params(&self) -> &[String];
}

impl RouteLike for RouteIntentIR {
    fn pattern(&self) -> &str {
        &self.path
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
    fn pattern(&self) -> &str {
        &self.prefix
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

/// The layout owning a route or nested layout, if any (longest matching
/// prefix wins — `layouts` is sorted prefix-length-descending). A layout
/// never owns itself.
fn owning_layout<'a>(r: &RoutedComp, layouts: &[&'a RoutedComp<'a>]) -> Option<&'a RoutedComp<'a>> {
    let full = if r.comp.layout_intent.is_some() {
        prefix_of(r)
    } else {
        r.pattern.as_str()
    };
    layouts.iter().copied().find(|l| {
        let is_self = r.comp.layout_intent.is_some() && l.name == r.name;
        let prefix = prefix_of(l);
        !is_self && (full == prefix || full.starts_with(&format!("{prefix}/")))
    })
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
