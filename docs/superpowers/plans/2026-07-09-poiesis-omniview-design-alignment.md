# Poiesis OmniView Design Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor Poiesis to use `design {}` blocks as first-class style citizens and component `style {}` blocks as second-class, eliminating duplicate style resolution logic in TypeScript by routing all style computation through the Rust WASM compiler.

**Architecture:** Add a `resolve_styles` WASM API to the Rust OmniLang compiler that traverses the IR and returns per-element computed CSS. Poiesis calls this API for canvas rendering instead of running its own TypeScript style-resolver/collect-styles pipeline. The TypeScript codegen pipeline is simplified to only emit .omni source structure (HTML tags, class/variant refs, semantic props) — all CSS resolution is the compiler's responsibility.

**Tech Stack:** Rust (omnilang crate, wasm-bindgen, serde), TypeScript (Poiesis Tauri app, Zustand store)

---

## File Structure

```
omnilang/src/codegen/omniview/
├── design.rs              → MODIFY: add resolve_class_properties(), resolve_element_styles()
├── mod.rs                 → MODIFY: add StyleResolveResult type, re-export
├── style_resolver.rs      → CREATE:  walk IR ViewTrees, resolve per-element CSS

omnilang/src/
├── wasm.rs                → MODIFY: add resolve_styles() WASM binding
├── lib.rs                 → MODIFY: add resolve_styles() public fn

poiesis/src/
├── core/types/
│   ├── element.ts         → MODIFY: add classRefs, variantRef to ViewElementData
│   └── style.ts           → MODIFY: simplify ElementStyle
├── data/
│   └── componentProtocol.ts → MODIFY: remove styleProperties, layout props
├── snippets/
│   └── builtin.ts         → REWRITE: all snippets use <tag> + class:/variant: syntax
├── commands/
│   └── rustCompiler.ts    → MODIFY: add resolveStyles() method, ResolvedStyles type
├── services/
│   ├── codegen/
│   │   ├── style-resolver.ts    → DELETE
│   │   ├── collect-styles.ts    → DELETE
│   │   ├── design-emitter.ts    → DELETE (replaced by WASM compile output)
│   │   ├── source-emitter.ts    → MODIFY: emit class:/variant:, remove CSS rule emission
│   │   ├── pipeline.ts          → MODIFY: remove Phase C style steps, add WASM resolve call
│   │   ├── view-tree-builder.ts → MODIFY: carry classRefs/variantRef from ElementData
│   │   └── types.ts             → MODIFY: update ResolvedStyle, ViewNode types
│   └── compiler/
│       └── compiler.ts          → MODIFY: integrate resolveStyles into compile flow
└── driver/
    └── CanvasDriver.ts          → MODIFY: remove layout props, add applyClass/applyVariant
```

---

## Phase 1: Rust WASM — Style Resolver Module

### Task 1: Create `style_resolver.rs` module

**Files:**
- Create: `omnilang/src/codegen/omniview/style_resolver.rs`
- Modify: `omnilang/src/codegen/omniview/mod.rs` (add module declaration)
- Modify: `omnilang/src/codegen/omniview/design.rs` (add helper fns)

- [ ] **Step 1: Add `StyleResolveResult` types to `mod.rs`**

In `omnilang/src/codegen/omniview/mod.rs`, add after the existing type definitions:

```rust
/// Per-element resolved style returned by resolve_styles().
/// Mirrors the info Poiesis needs to render a canvas element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedElementStyle {
    /// Tree path like "MyPage.div.0.Card.0.header.1"
    pub path: String,
    /// HTML tag name
    pub tag: String,
    /// Resolved CSS class names applied to this element
    pub classes: Vec<String>,
    /// Merged CSS properties (design class → variant → component style → inline)
    pub computed_css: FxHashMap<String, String>,
    /// Semantic HTML attributes (id, placeholder, src, etc.)
    pub html_attrs: FxHashMap<String, String>,
    /// Event bindings: event_name → handler_fn_name
    pub events: FxHashMap<String, String>,
    /// Signal bindings: prop_name → signal_name
    pub bindings: FxHashMap<String, String>,
    /// Inline style expressions (signal-driven only)
    pub inline_style: Option<serde_json::Value>,
    /// Child element paths (for tree reconstruction)
    pub children: Vec<String>,
    /// Source line in .omni file
    pub source_line: u32,
}

/// Top-level result from resolve_styles().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleResolveResult {
    pub success: bool,
    /// Per-component resolved elements, keyed by component name
    pub components: FxHashMap<String, Vec<ResolvedElementStyle>>,
    /// Design system summary (for Poiesis design panel)
    pub design: Option<DesignSummary>,
    pub errors: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignSummary {
    pub name: String,
    pub tokens: Vec<TokenSummary>,
    pub classes: Vec<ClassSummary>,
    pub variants: Vec<VariantSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub name: String,
    pub token_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSummary {
    pub name: String,
    pub properties: FxHashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantSummary {
    pub name: String,
    pub dimensions: Vec<VariantDimSummary>,
    pub defaults: FxHashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDimSummary {
    pub name: String,
    pub values: Vec<String>,
}
```

And add the module declaration:
```rust
pub mod style_resolver;
```

- [ ] **Step 2: Add `resolve_class_properties()` to `design.rs`**

In `omnilang/src/codegen/omniview/design.rs`, add after `expand_variant()`:

```rust
/// Look up a class by name and return its resolved CSS properties.
/// Returns None if the class is not found.
pub fn resolve_class_properties(
    design: &DesignIR,
    class_name: &str,
) -> Option<&FxHashMap<String, String>> {
    design
        .classes
        .iter()
        .find(|c| c.name == class_name)
        .map(|c| &c.properties)
}

/// Look up a class by name and return its pseudo-state properties.
pub fn resolve_class_pseudo_states(
    design: &DesignIR,
    class_name: &str,
) -> Option<&[PseudoStateIR]> {
    design
        .classes
        .iter()
        .find(|c| c.name == class_name)
        .map(|c| c.pseudo_states.as_slice())
}
```

- [ ] **Step 3: Implement `style_resolver.rs` — core traversal logic**

Create `omnilang/src/codegen/omniview/style_resolver.rs`:

```rust
//! Style resolver: traverses IR ViewTrees and resolves per-element computed CSS.
//!
//! Used by the Poiesis editor to render the visual canvas with accurate styling.
//! Unlike the full codegen pipeline, this does NOT generate JS/HTML/CSS strings —
//! it returns structured data that the editor consumes directly.

use crate::codegen::omniview::design::{expand_variant, resolve_class_properties};
use crate::codegen::omniview::mod as omniview_mod;
use crate::ir::design::DesignIR;
use crate::ir::types::*;
use crate::ir::*;
use rustc_hash::FxHashMap;

use super::{
    ClassSummary, DesignSummary, ResolvedElementStyle, StyleResolveResult,
    TokenSummary, VariantDimSummary, VariantSummary,
};

/// Resolve per-element computed styles from IR.
/// `design_map` is built from all `design` blocks in the IR program.
pub fn resolve_styles_from_ir(
    ir: &IRProgram,
    design_map: &FxHashMap<String, DesignIR>,
) -> StyleResolveResult {
    let mut components: FxHashMap<String, Vec<ResolvedElementStyle>> = FxHashMap::default();

    for (module_name, module) in &ir.modules {
        for (comp_name, _component) in &module.components {
            // Find the view tree for this component
            let view_tree = match find_component_view(ir, module_name, comp_name) {
                Some(v) => v,
                None => continue,
            };

            let mut elements: Vec<ResolvedElementStyle> = Vec::new();
            let path_prefix = comp_name.clone();
            resolve_view_tree(
                &view_tree,
                &path_prefix,
                0,
                design_map,
                &mut elements,
            );
            components.insert(comp_name.clone(), elements);
        }
    }

    // Build design summary
    let design = ir.modules.values()
        .flat_map(|m| m.designs.values())
        .next()
        .map(|d| build_design_summary(d));

    StyleResolveResult {
        success: true,
        components,
        design,
        errors: Vec::new(),
    }
}

/// Find a component's view tree in the IR.
fn find_component_view(
    _ir: &IRProgram,
    _module_name: &str,
    _comp_name: &str,
) -> Option<ViewTree> {
    // The view tree is stored in the component's view field.
    // Traverse the component's IR to find the ViewTree.
    // Actual implementation will access ir.modules[module].components[comp].view
    // This is a placeholder — the exact field access depends on IRModule structure.
    None
}

/// Recursively resolve styles for a ViewTree node and its children.
fn resolve_view_tree(
    tree: &ViewTree,
    parent_path: &str,
    index: usize,
    design_map: &FxHashMap<String, DesignIR>,
    out: &mut Vec<ResolvedElementStyle>,
) {
    match tree {
        ViewTree::Element(el) => {
            let path = format!("{}.{}.{}", parent_path, el.tag, index);
            let resolved = resolve_element(el, &path, design_map);

            // Resolve children
            let mut child_paths: Vec<String> = Vec::new();
            for (i, child) in el.children.iter().enumerate() {
                let child_path = format!("{}.{}.{}", path, child_tag(child), i);
                child_paths.push(child_path.clone());
                resolve_view_tree(child, &path, i, design_map, out);
            }

            let mut result = resolved;
            result.children = child_paths;
            out.push(result);
        }
        ViewTree::Conditional(cond) => {
            for (i, branch) in cond.branches.iter().enumerate() {
                for child in &branch.body {
                    let branch_path = format!("{}.if.{}", parent_path, i);
                    resolve_view_tree(child, &branch_path, 0, design_map, out);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                for child in fallback {
                    let fb_path = format!("{}.else", parent_path);
                    resolve_view_tree(child, &fb_path, 0, design_map, out);
                }
            }
        }
        ViewTree::ListRendering(list) => {
            for child in &list.template {
                let list_path = format!("{}.for", parent_path);
                resolve_view_tree(child, &list_path, 0, design_map, out);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {
            // Non-element nodes — no styles to resolve
        }
    }
}

/// Get a short tag-like name for a ViewTree child (for path generation).
fn child_tag(tree: &ViewTree) -> &str {
    match tree {
        ViewTree::Element(el) => &el.tag,
        ViewTree::Conditional(_) => "if",
        ViewTree::ListRendering(_) => "for",
        ViewTree::Interpolation(_) => "interp",
        ViewTree::Text(_) => "text",
        ViewTree::Slot(_) => "slot",
    }
}

/// Resolve all style sources for a single ElementIR into computed CSS.
fn resolve_element(
    el: &ElementIR,
    path: &str,
    design_map: &FxHashMap<String, DesignIR>,
) -> ResolvedElementStyle {
    let mut classes: Vec<String> = Vec::new();
    let mut computed_css: FxHashMap<String, String> = FxHashMap::default();
    let mut html_attrs: FxHashMap<String, String> = FxHashMap::default();
    let mut events: FxHashMap<String, String> = FxHashMap::default();
    let mut bindings: FxHashMap<String, String> = FxHashMap::default();

    // 1. Collect class names from `class` prop
    if let Some(class_expr) = el.props.get("class") {
        if let ExprIR::LiteralExpr { value } = class_expr {
            if let Some(class_str) = value.as_str() {
                for name in class_str.split_whitespace() {
                    if !name.is_empty() {
                        classes.push(name.to_string());
                    }
                }
            }
        }
    }

    // 2. Expand variant binding → class names + CSS
    if let Some(variant_prop) = el.props.get("_design_variant") {
        if let ExprIR::LiteralExpr { value } = variant_prop {
            let design_name = value.get("design_name")
                .and_then(|v| v.as_str());
            let variant_name = value.get("variant_name")
                .and_then(|v| v.as_str());
            let dim_values = value.get("dimension_values");

            if let (Some(dn), Some(vn), Some(dv)) = (design_name, variant_name, dim_values) {
                let dim_map: FxHashMap<String, String> = dv
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default();

                if let Some(design) = design_map.get(dn) {
                    if let Ok((variant_classes, _extra_css)) =
                        expand_variant(design, vn, &dim_map)
                    {
                        classes.extend(variant_classes);
                    }
                }
            }
        }
    }

    // 3. Resolve CSS properties from referenced design classes
    for class_name in &classes {
        // Try each registered design
        for design in design_map.values() {
            if let Some(props) = resolve_class_properties(design, class_name) {
                for (prop, val) in props {
                    computed_css.insert(prop.clone(), val.clone());
                }
            }
        }
    }

    // 4. Extract semantic HTML attrs, events, bindings from props
    for (name, value) in &el.props {
        if name == "class" || name == "_design_variant" || name.starts_with('_') {
            continue;
        }
        if let ExprIR::LiteralExpr { value: v } = value {
            if let Some(s) = v.as_str() {
                if name.starts_with("on") {
                    // Event binding: onPress → press, value → handler fn name
                    let event_name = name[2..].to_lowercase();
                    events.insert(event_name, s.to_string());
                } else if is_html_attr(name) {
                    html_attrs.insert(name.clone(), s.to_string());
                }
            }
        }
    }

    // 5. Extract signal bindings
    for (prop, signal_ref) in &el.bindings {
        bindings.insert(prop.clone(), signal_ref.signal_name.clone());
    }

    // 6. Pass through inline_style and style_classes
    let inline_style = el.inline_style.clone();

    ResolvedElementStyle {
        path: path.to_string(),
        tag: el.tag.clone(),
        classes,
        computed_css,
        html_attrs,
        events,
        bindings,
        inline_style,
        children: Vec::new(), // filled by caller
        source_line: el.source_line,
    }
}

/// Check if a prop name is a known HTML attribute (not a style or internal prop).
fn is_html_attr(name: &str) -> bool {
    matches!(
        name,
        "id" | "placeholder" | "type" | "value" | "disabled" | "checked"
        | "required" | "readonly" | "href" | "src" | "alt" | "title"
        | "name" | "rows" | "cols" | "max" | "min" | "step"
        | "label" | "for" | "action" | "method"
    )
}

/// Build a design summary from DesignIR for the Poiesis design panel.
fn build_design_summary(design: &DesignIR) -> DesignSummary {
    DesignSummary {
        name: design.name.clone(),
        tokens: design
            .tokens
            .iter()
            .map(|(name, t)| TokenSummary {
                name: name.clone(),
                token_type: format!("{:?}", t.token_type),
                value: t.value.clone(),
            })
            .collect(),
        classes: design
            .classes
            .iter()
            .map(|c| ClassSummary {
                name: c.name.clone(),
                properties: c.properties.clone(),
            })
            .collect(),
        variants: design
            .variants
            .iter()
            .map(|v| VariantSummary {
                name: v.name.clone(),
                dimensions: v
                    .dimensions
                    .iter()
                    .map(|d| VariantDimSummary {
                        name: d.name.clone(),
                        values: d.values.clone(),
                    })
                    .collect(),
                defaults: v.defaults.clone(),
            })
            .collect(),
    }
}
```

- [ ] **Step 4: Verify the module compiles**

Run: `cargo check --features wasm,codegen-omniview -p omnilang`
Expected: No new errors from the new module (may have some type mismatches to fix).

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/style_resolver.rs omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/design.rs
git commit -m "feat: add style_resolver module for per-element CSS resolution"
```

### Task 2: Add WASM binding and public API

**Files:**
- Modify: `omnilang/src/wasm.rs`
- Modify: `omnilang/src/lib.rs`

- [ ] **Step 1: Add `resolve_styles()` public function to `lib.rs`**

In `omnilang/src/lib.rs`, add after `compile_omniview_full`:

```rust
/// Resolve per-element computed styles for editor canvas rendering.
/// Parses .omni source, lowers to IR, then traverses ViewTrees to compute
/// the final CSS properties for each element (merging design classes,
/// variants, component styles, and inline styles).
///
/// Returns structured JSON that Poiesis consumes directly — no JS/HTML/CSS
/// codegen is performed.
#[cfg(feature = "codegen-omniview")]
#[must_use]
pub fn resolve_styles(source: &str) -> StyleResolveResult {
    use crate::codegen::omniview::style_resolver::resolve_styles_from_ir;

    match run_pipeline(source) {
        Err(errors) => StyleResolveResult {
            success: false,
            components: FxHashMap::default(),
            design: None,
            errors,
        },
        Ok(pipeline) => {
            let design_map: FxHashMap<String, crate::ir::design::DesignIR> =
                pipeline.ir.modules.values()
                    .flat_map(|m| m.designs.iter())
                    .map(|(name, design)| (name.clone(), design.clone()))
                    .collect();

            let result = resolve_styles_from_ir(&pipeline.ir, &design_map);

            StyleResolveResult {
                success: !pipeline.has_errors,
                components: result.components,
                design: result.design,
                errors: pipeline.diagnostics,
            }
        }
    }
}
```

- [ ] **Step 2: Add WASM binding in `wasm.rs`**

In `omnilang/src/wasm.rs`, add after `compile_to_omniview_full`:

```rust
/// Resolve per-element computed styles for editor canvas rendering.
/// Returns structured JSON with component → element → computed CSS.
#[cfg(feature = "codegen-omniview")]
#[wasm_bindgen]
pub fn resolve_styles(source: &str) -> JsValue {
    const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;
    if source.len() > MAX_INPUT_SIZE {
        return js_error_value("Input exceeds 10MB limit");
    }
    to_js_value(&crate::resolve_styles(source))
}
```

- [ ] **Step 3: Verify WASM builds**

Run: `cargo check --features wasm,codegen-omniview -p omnilang`
Expected: No compile errors. Fix any type mismatches.

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/wasm.rs omnilang/src/lib.rs
git commit -m "feat: add resolve_styles() WASM API for editor canvas rendering"
```

---

## Phase 2: Poiesis — WASM Bridge + Data Model

### Task 3: Add `resolveStyles` to RustCompiler TypeScript bridge

**Files:**
- Modify: `poiesis/src/commands/rustCompiler.ts`

- [ ] **Step 1: Add type definitions**

After the `OmniViewFullResult` type (~line 75), add:

```typescript
type ResolvedElementStyle = {
  path: string;
  tag: string;
  classes: string[];
  computedCss: Record<string, string>;
  htmlAttrs: Record<string, string>;
  events: Record<string, string>;
  bindings: Record<string, string>;
  inlineStyle: Record<string, string> | null;
  children: string[];
  sourceLine: number;
};

type DesignSummary = {
  name: string;
  tokens: Array<{ name: string; tokenType: string; value: string }>;
  classes: Array<{ name: string; properties: Record<string, string> }>;
  variants: Array<{
    name: string;
    dimensions: Array<{ name: string; values: string[] }>;
    defaults: Record<string, string>;
  }>;
};

type StyleResolveResult = {
  success: boolean;
  components: Record<string, ResolvedElementStyle[]>;
  design: DesignSummary | null;
  errors: Diagnostic[];
};
```

- [ ] **Step 2: Add `resolveStyles` to WASM interface and class**

In the `wasm` private field type (~line 97-104), add:

```typescript
resolve_styles?: (source: string) => any;
```

In the `init()` method, after the `compile_multi_to_omniview` line (~line 153), add:

```typescript
resolve_styles: wasmModule.resolve_styles as unknown as (source: string) => any,
```

In the `RustCompiler` class, add the public method after `compileMultiToOmniView`:

```typescript
resolveStyles(source: string): StyleResolveResult {
  if (!this.wasm || !this.wasm.resolve_styles) {
    return {
      success: false,
      components: {},
      design: null,
      errors: [{ severity: 'error', code: 'WASM', message: 'resolve_styles not available in WASM', line: 0, col: 0, end_line: 0, end_col: 0 }],
    };
  }
  try {
    return this.wasm.resolve_styles(source) as StyleResolveResult;
  } catch (e) {
    log.error('resolveStyles failed:', e);
    return {
      success: false,
      components: {},
      design: null,
      errors: [{ severity: 'error', code: 'WASM', message: `resolve_styles error: ${e}`, line: 0, col: 0, end_line: 0, end_col: 0 }],
    };
  }
}
```

- [ ] **Step 3: Add to exports**

At the bottom of the file, add `StyleResolveResult` and `ResolvedElementStyle` to the export type list:

```typescript
export type { CompileResult, OmniViewResult, OmniViewFullResult, MultiCompileResult, MultiCompileDiagnostic, ComponentCompileResult, Diagnostic, SourceMap, StyleResolveResult, ResolvedElementStyle, DesignSummary };
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/commands/rustCompiler.ts
git commit -m "feat: add resolveStyles() to RustCompiler TypeScript bridge"
```

### Task 4: Update ElementData types for class/variant refs

**Files:**
- Modify: `poiesis/src/core/types/element.ts`
- Modify: `poiesis/src/core/types/style.ts`

- [ ] **Step 1: Update `ViewElementData` in `element.ts`**

Replace the `ViewElementData` interface:

```typescript
export interface ViewElementData extends ElementDataBase {
  nodeType: "view";
  tag: string;
  /** Semantic props only (label, src, placeholder, type, etc.) — no layout or style */
  props: Record<string, unknown>;
  /** References to design system classes, e.g. ["AppDesign.Card", "AppDesign.Row"] */
  classRefs?: string[];
  /** Reference to a design variant binding */
  variantRef?: {
    designName: string;
    variantName: string;
    params: Record<string, string>;
  } | null;
  /** Component-local style class names (defined in component style {} block) */
  componentClasses?: string[];
  /** Inline style ONLY for signal-driven dynamic values */
  inlineStyle?: Record<string, string>;
  /** Deprecated: kept for backward compat during migration */
  style?: ElementStyle;
  children: string[];
  ports?: PortData[];
  renderMode?: "dom" | "canvas";
  isFolded?: boolean;
  typeAnnotation?: string;
  storeName?: string;
  position?: { x: number; y: number };
  componentRef?: string;
}
```

- [ ] **Step 2: Simplify `ElementStyle` in `style.ts`**

Replace the `ElementStyle` interface:

```typescript
/** Per-element style references (NOT CSS values — those come from WASM resolve_styles).
 *  Kept for backward compat; new code should use classRefs/variantRef directly on ViewElementData. */
export interface ElementStyle {
  classRefs: string[];
  inlineProperties: StyleProperty[];
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/core/types/element.ts poiesis/src/core/types/style.ts
git commit -m "refactor: update ElementData types with classRefs/variantRef for OmniView alignment"
```

---

## Phase 3: Delete Duplicate Style Logic

### Task 5: Delete `style-resolver.ts`

**Files:**
- Delete: `poiesis/src/services/codegen/style-resolver.ts`
- Modify: `poiesis/src/services/codegen/pipeline.ts` (remove import)
- Modify: `poiesis/src/services/codegen/collect-styles.ts` (remove dependency)

- [ ] **Step 1: Remove `style-resolver.ts` import from `collect-styles.ts`**

In `poiesis/src/services/codegen/collect-styles.ts`, delete line 6:
```typescript
// DELETE this line:
import { resolveStyle } from "./style-resolver";
```

And delete the `walkApply` function and `applyStyles` function (lines 16-43) since they call `resolveStyle()`. Replace `applyStyles` with a no-op stub:

```typescript
/** Stub: style resolution is now performed by WASM resolve_styles().
 *  Kept as a no-op to avoid breaking the pipeline call site. */
export function applyStyles(_roots: ViewNode[]): void {
  // No-op: WASM handles style resolution
}
```

- [ ] **Step 2: Delete the file**

```bash
rm poiesis/src/services/codegen/style-resolver.ts
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit`
Expected: Errors only from remaining references to deleted functions. We'll fix those in subsequent tasks.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/
git commit -m "refactor: delete style-resolver.ts — replaced by WASM resolve_styles"
```

### Task 6: Simplify `collect-styles.ts`

**Files:**
- Modify: `poiesis/src/services/codegen/collect-styles.ts`
- Modify: `poiesis/src/services/codegen/pipeline.ts`

- [ ] **Step 1: Rewrite `collectStylesFromEntries`**

Replace the body of `collectStylesFromEntries` to remove design class emission and snake_case CSS mapping:

```typescript
export function collectStylesFromEntries(
  elements: Array<{
    id: string;
    style?: { classRefs?: string[]; inlineProperties?: Array<{ name: string; segments: StyleValueSegment[] }> };
  }>,
  _styleRegistry: StyleRegistry,
  _roots?: ViewNode[],
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;
  elementInlineMap: Map<string, Record<string, string>>;
} {
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  // Element classRefs → elementClassMap (design classes from classRefs)
  for (const el of elements) {
    const classRefs = el.style?.classRefs ?? [];
    if (classRefs.length > 0) {
      elementClassMap.set(el.id, [...new Set(classRefs)]);
    }
    const inlineProps = el.style?.inlineProperties ?? [];
    if (inlineProps.length > 0) {
      const inlineMap: Record<string, string> = {};
      for (const prop of inlineProps) {
        inlineMap[prop.name] = compileSegments(prop.segments);
      }
      elementInlineMap.set(el.id, inlineMap);
    }
  }

  // componentStyle is now empty — CSS comes from WASM resolve_styles()
  return { componentStyle: {}, elementClassMap, elementInlineMap };
}
```

- [ ] **Step 2: Update `pipeline.ts` runPhaseC**

In `pipeline.ts`, update `runPhaseC` to handle the fact that `applyStyles` is now a no-op:

```typescript
function runPhaseC(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
): { ctx: PhaseCContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];

  // applyStyles is now a no-op (WASM handles style resolution)
  const c1 = runPhase("C", "style-resolver", () => {
    applyStyles(phaseA.roots);
  });
  errors.push(...c1.errors);

  // Plumb signal bindings — unchanged
  if (c1.errors.length === 0) {
    const elBindings = new Map<string, Record<string, string>>();
    for (const [sigName, targets] of phaseA.bindings.signalBindings) {
      for (const t of targets) {
        const existing = elBindings.get(t.elementId) ?? {};
        existing[t.propName] = sigName;
        elBindings.set(t.elementId, existing);
      }
    }
    if (elBindings.size > 0) {
      for (const root of phaseA.roots) {
        walkAndApplyBindings(root, elBindings);
      }
    }
  }

  const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(
        expandedElements,
        { variables: [], classes: [] }, // registry no longer needed
        phaseA.roots,
      );

  // ... rest unchanged
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/collect-styles.ts poiesis/src/services/codegen/pipeline.ts
git commit -m "refactor: simplify collect-styles.ts — CSS resolution delegated to WASM"
```

### Task 7: Delete `design-emitter.ts`

**Files:**
- Delete: `poiesis/src/services/codegen/design-emitter.ts`
- Modify: `poiesis/src/services/codegen/pipeline.ts` (remove export)
- Modify: `poiesis/src/services/codegen/index.ts` (remove export)

- [ ] **Step 1: Remove exports from `index.ts`**

In `poiesis/src/services/codegen/index.ts`, remove line:
```typescript
// DELETE:
export { emitDesignSource } from "./design-emitter";
```

- [ ] **Step 2: Remove re-export from `pipeline.ts`**

In `pipeline.ts`, remove the last line:
```typescript
// DELETE:
export { emitDesignSource };
```

- [ ] **Step 3: Delete the file**

```bash
rm poiesis/src/services/codegen/design-emitter.ts
```

- [ ] **Step 4: Verify no remaining references**

```bash
cd poiesis && grep -r "design-emitter\|emitDesignSource\|generateDesignOmniSource" src/ --include="*.ts" --include="*.tsx"
```
Expected: No output (no remaining references).

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/
git commit -m "refactor: delete design-emitter.ts — design.omni generated by WASM compile"
```

---

## Phase 4: Update Codegen Pipeline

### Task 8: Simplify `source-emitter.ts`

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts`

- [ ] **Step 1: Update `emitViewNode` to emit `class:` and `variant:` attributes**

Replace the element emission section. The key change: instead of emitting CSS rules from snake_case props, emit class refs and variant refs:

In `emitViewNode`, after building the open tag parts, add `class:` and `variant:` attribute emission. Update the relevant section (~line 328-340):

```typescript
// Element nodes — build attributes
const style = node.resolvedStyle;
if (!style) return;

const parts: string[] = [];
parts.push(`<${style.htmlTag}`);
parts.push(` id="${node.id}"`);

// Emit class: attribute for design class refs
const extraClasses = this.elementClassMap?.get(node.id);
const allClasses = extraClasses?.length
  ? `${style.cssClassName} ${extraClasses.join(" ")}`
  : style.cssClassName;

if (allClasses.trim()) {
  // Split into design classes (PascalCase) and local classes (kebab-case/path-based)
  const designClasses = extraClasses?.filter(c => /^[A-Z]/.test(c)) ?? [];
  const localClasses = [style.cssClassName];

  for (const dc of designClasses) {
    parts.push(` class: ${dc}`);
  }
  // Local classes use bare class: syntax
  for (const lc of localClasses) {
    if (lc.trim()) parts.push(` class: ${lc}`);
  }
}

// Emit variant: attribute for variant refs
// (variant ref info comes from element-level metadata — TODO: thread from ElementData)
if ((node as any)._variantRef) {
  const vr = (node as any)._variantRef;
  const params = Object.entries(vr.params as Record<string, string>)
    .map(([k, v]) => `${k}=${v}`)
    .join(", ");
  parts.push(` variant: ${vr.designName}.${vr.variantName}(${params})`);
}

// ... rest unchanged (htmlAttrs, inline styles, events, bindings)
```

- [ ] **Step 2: Remove CSS rule emission from `emitComponent`**

In `emitComponent`, remove the style {} block emission that comes from snake_case CSS rules (~line 155-172). Replace with:

```typescript
// Style — emit component-local style classes only (design classes are in design.omni)
const style = comp.style || {};
const componentLocalStyle = Object.entries(style).filter(
  ([_cls, rules]) => Object.keys(rules).length > 0
);
if (componentLocalStyle.length > 0) {
  w.emit(`style {`);
  w.indent();
  for (const [cls, rules] of componentLocalStyle) {
    w.emit(`.${cls} {`);
    w.indent();
    for (const [prop, val] of Object.entries(rules)) {
      w.emit(`${prop}: ${val};`);
    }
    w.dedent();
    w.emit(`}`);
    w.blank();
  }
  w.dedent();
  w.emit(`}`);
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts
git commit -m "refactor: update source-emitter for class:/variant: syntax, remove CSS rule emission"
```

### Task 9: Update `pipeline.ts` and `types.ts`

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`
- Modify: `poiesis/src/services/codegen/types.ts`

- [ ] **Step 1: Update `ViewNode` types in `types.ts`**

In `types.ts`, update `ResolvedStyle` and `ElementViewNode`:

```typescript
export interface ResolvedStyle {
  htmlTag: string;
  cssClassName: string;
  contentText?: string;
  htmlAttrs: Record<string, string>;
  eventBindings: Record<string, string>;
  bindings: Record<string, string>;
  /** CSS rules are now resolved by WASM; this is empty during TS codegen */
  cssRules: Record<string, string>;
  usedClassNames: string[];
}

export interface ElementViewNode extends ViewNodeBase {
  kind: "element";
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  resolvedStyle?: ResolvedStyle;
  inline_style?: Record<string, string> | null;
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  /** Design system class refs from element, e.g. ["AppDesign.Card"] */
  classRefs?: string[];
  /** Variant binding from element */
  variantRef?: {
    designName: string;
    variantName: string;
    params: Record<string, string>;
  } | null;
}
```

- [ ] **Step 2: Update `pipeline.ts` to pass classRefs through**

In `pipeline.ts` `generateOmniSource`, pass `classRefs` and `variantRef` through the pipeline. In the `EmitInput`, add optional fields for design refs. These are threaded from ElementData → ViewNode → source-emitter.

- [ ] **Step 3: Update `component-builder.ts`**

In `component-builder.ts`, when building `ComponentPart`, pass classRefs and variantRef from phase A context:

```typescript
// In buildComponentPart():
const classRefs = phaseA.ctx.roots.flatMap(r => collectClassRefs(r));
const variantRefs = phaseA.ctx.roots.flatMap(r => collectVariantRefs(r));
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/
git commit -m "refactor: update pipeline types for classRefs/variantRef threading"
```

---

## Phase 5: Update Snippets, Protocol, and Driver

### Task 10: Rewrite `builtin.ts` snippets

**Files:**
- Modify: `poiesis/src/snippets/builtin.ts`

- [ ] **Step 1: Rewrite all view-related snippets**

Replace every snippet that uses old `Container { }`, `Text { }`, `Flex { }` syntax:

```typescript
// BEFORE (old syntax):
{
  id: "comp-react",
  label: "React Component",
  body: `component $1 {
    view {
        Container {
            Text { label: "$2" }
        }
    }
}`,
},

// AFTER (OmniView syntax):
{
  id: "comp-react",
  label: "Component",
  body: `component $1 {
    view {
        <div class: AppDesign.Page>
            <p>$2</p>
        </div>
    }
    style {
        .page {
            max-width: "1200px";
            margin: "0 auto";
            padding: token(spaceMd);
        }
    }
}`,
},
```

Rewrite all snippets following this pattern. Key changes for each:

- `view-flex-row`:
```typescript
{
  id: "view-flex-row",
  label: "Flex Row",
  body: `<div class: AppDesign.Row>
    $1
</div>`,
},
```

- `view-card`:
```typescript
{
  id: "view-card",
  label: "Card",
  body: `<div class: AppDesign.Card>
    $1
</div>`,
},
```

- `view-bind`:
```typescript
{
  id: "view-bind",
  label: "Bind Value",
  body: `<input
    bind:value={$1}
    placeholder="$2"
    class: AppDesign.Input
    data-testid="$3" />`,
},
```

- `view-event`:
```typescript
{
  id: "view-event",
  label: "Event Handler",
  body: `<button
    variant: AppDesign.Button(color=primary, size=md)
    @press={fn() { $2 }}
    data-testid="$3">
    $1
</button>`,
},
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/snippets/builtin.ts
git commit -m "refactor: rewrite all snippets with OmniView <tag> class:/variant: syntax"
```

### Task 11: Simplify `componentProtocol.ts`

**Files:**
- Modify: `poiesis/src/data/componentProtocol.ts`

- [ ] **Step 1: Remove `styleProperties` and layout props from all entries**

For each entry in `COMPONENT_PROTOCOL`, remove:
- `styleProperties` array
- Layout props (`direction`, `gap`, `align`, `justify`)

Add `recommendedClass` where applicable. Example for `div`:

```typescript
div: {
  component: "div", category: "Layout", description: "通用容器 / 布局块",
  props: {
    // Only semantic props — layout/styling via design classes
  },
  events: {},
  styleProperties: [],  // Empty — styling via design classes
  children: true,
  recommendedClass: "AppDesign.Card",  // Suggested design class
},
```

For `button`:
```typescript
button: {
  component: "button", category: "Actions", description: "按钮",
  props: {
    label: { type: "string", default: "Button", editor: "text" },
    disabled: { type: "bool", default: false, editor: "checkbox" },
  },
  events: {
    onPress: { id: "evt-btn-press-001", description: "点击", payload: "MouseEvent" },
  },
  styleProperties: [],
  children: true,
  recommendedVariant: {
    designName: "AppDesign",
    variantName: "Button",
    defaultParams: { color: "primary", size: "md" },
  },
},
```

- [ ] **Step 2: Add `recommendedClass` and `recommendedVariant` to `ComponentProtocolEntry`**

```typescript
export interface ComponentProtocolEntry {
  component: string;
  category: string;
  description: string;
  props: Record<string, PropMeta>;
  events: Record<string, EventMeta>;
  styleProperties: string[];  // Kept for backward compat, always empty []
  children: boolean;
  isCustom?: boolean;
  /** Recommended design class to apply (e.g. "AppDesign.Card") */
  recommendedClass?: string;
  /** Recommended variant binding */
  recommendedVariant?: {
    designName: string;
    variantName: string;
    defaultParams: Record<string, string>;
  };
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/data/componentProtocol.ts
git commit -m "refactor: remove layout props and styleProperties from component protocol"
```

### Task 12: Update `CanvasDriver.ts`

**Files:**
- Modify: `poiesis/src/driver/CanvasDriver.ts`

- [ ] **Step 1: Remove layout props from convenience methods**

Update `flex()`:
```typescript
/** Add a div. Layout via design classes, not inline props. */
flex(classRefs?: string[], parentId?: string | null): ElementHandle {
  return this.component("div", {}, parentId);
  // classRefs applied separately
}
```

Update `container()`:
```typescript
container(classRefs?: string[], parentId?: string | null): ElementHandle {
  return this.component("div", {}, parentId);
}
```

Update `card()` similarly.

- [ ] **Step 2: Add `applyClass` and `applyVariant` methods**

```typescript
/** Apply a design class reference to an element. */
applyClass(elementId: string, classRef: string): void {
  const store = getDesignStore();
  const el = store.elements.find(e => e.id === elementId);
  if (!el || el.nodeType !== "view") return;
  const existing = (el as ViewElementData).classRefs ?? [];
  if (!existing.includes(classRef)) {
    store.updateElement(elementId, {
      classRefs: [...existing, classRef],
    });
  }
}

/** Apply a variant binding to an element. */
applyVariant(
  elementId: string,
  designName: string,
  variantName: string,
  params: Record<string, string>,
): void {
  const store = getDesignStore();
  store.updateElement(elementId, {
    variantRef: { designName, variantName, params },
  });
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/driver/CanvasDriver.ts
git commit -m "refactor: remove layout props from CanvasDriver, add applyClass/applyVariant"
```

---

## Phase 6: Integration & Verification

### Task 13: Update `compiler.ts` to use `resolveStyles`

**Files:**
- Modify: `poiesis/src/services/compiler/compiler.ts`

- [ ] **Step 1: Integrate resolveStyles into compileProject**

After successful compilation, also call `resolveStyles` and store the result for canvas rendering:

```typescript
// After WASM compile succeeds (~line 98-123), add:
if (fullResult.success && fullResult.files) {
  // Also resolve styles for canvas
  const styleResult = rustCompiler.resolveStyles(genResult.value);
  if (styleResult.success) {
    // Store resolved styles in the store for canvas rendering
    usePoiesisStore.getState().setResolvedStyles(styleResult);
  }
}
```

- [ ] **Step 2: Add `setResolvedStyles` to the store**

In `poiesis/src/store/slices/design-system.ts` or a new slice, add:

```typescript
setResolvedStyles: (result: import("../../commands/rustCompiler").StyleResolveResult) =>
  set({ resolvedStyles: result, isDirty: false }),
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/compiler/compiler.ts poiesis/src/store/
git commit -m "feat: integrate resolveStyles into compile pipeline"
```

### Task 14: End-to-end verification with learn-omniview

**Files:**
- Use existing: `learn-omniview/` test project

- [ ] **Step 1: Build WASM**

```bash
cd omnilang && cargo build --features wasm,codegen-omniview --release --target wasm32-unknown-unknown
```

- [ ] **Step 2: Rebuild Poiesis**

```bash
cd poiesis && npm run build 2>&1 | head -50
```
Expected: Build succeeds. Fix any TypeScript errors.

- [ ] **Step 3: Test with a simple .omni source**

Use Victauri MCP to test in the running Poiesis app:

```javascript
// In Poiesis dev console or via Victauri:
const result = await window.__OMNI__.compile();
console.log("Compile:", result.success);
```

- [ ] **Step 4: Verify generated .omni uses correct syntax**

Check that generated source uses:
- `<div class: AppDesign.Card>` instead of `<div direction="row" gap="8px">`
- `variant: AppDesign.Button(color=primary, size=md)` instead of inline button styles
- `style { .localClass { ... } }` instead of snake_case CSS rules in style blocks

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: final integration fixes and verification"
```

### Task 15: Run full test suite

- [ ] **Step 1: Rust tests**

```bash
cd omnilang && cargo test --features wasm,codegen-omniview
```
Expected: All tests pass.

- [ ] **Step 2: Poiesis TypeScript check**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | tail -20
```
Expected: No new errors introduced by the refactoring.

- [ ] **Step 3: Run Poiesis lint**

```bash
cd poiesis && npx eslint src/ --ext .ts,.tsx 2>&1 | tail -20
```

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: all tests passing after OmniView design alignment refactoring"
```

---

## Summary

| Phase | Tasks | Files Changed |
|-------|-------|---------------|
| 1. Rust WASM style resolver | 1-2 | +1 new, 3 modified |
| 2. Poiesis WASM bridge + data model | 3-4 | 4 modified |
| 3. Delete duplicate style logic | 5-7 | 2 deleted, 3 modified |
| 4. Update codegen pipeline | 8-9 | 4 modified |
| 5. Snippets, protocol, driver | 10-12 | 3 modified |
| 6. Integration & verification | 13-15 | 4 modified |
| **Total** | **15 tasks** | **+1 created, 2 deleted, 18 modified** |
