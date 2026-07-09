# DesignCanvas Rendering Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the dual-path DesignCanvas rendering engine with a unified WASM-driven architecture featuring an infinite canvas with zoom/pan and device artboard.

**Architecture:** WASM `codegen::omniview::generate()` runs the full compilation pipeline and additionally records an `element_tree` of structured render nodes during its HTML walk. React consumes this tree via `OmniViewElementTree`/`OmniViewElement` (React.createElement with stable nid keys + React.memo). CSS is managed by a scope pool that only replaces `<style>` elements whose content hash changed. An `InfiniteCanvas` wraps an `Artboard` with CSS transform-based zoom/pan.

**Tech Stack:** Rust (omnilang WASM), TypeScript/React (poiesis), Zustand (store)

## Global Constraints

- WASM compile path: reuse `codegen::omniview::generate()` unchanged, only append `element_tree` collection
- No new WASM functions; reuse `compile_to_omniview` with `editor_mode: true`
- React rendering: `React.createElement` with stable `data-nid` keys, no `dangerouslySetInnerHTML`
- CSS updates: scope-pooled `<style>` elements, hash-compare before replace
- Zoom: CSS `transform: scale()` + `transform-origin: 0 0`, range [0.1, 5.0]
- Existing editor bridge (`editor_mode`) produces `data-nid` attributes and `postMessage`

---

### Task 1: Add `element_tree` to WASM OmniViewOutput

**Files:**
- Modify: `omnilang/src/codegen/omniview/mod.rs` (struct definitions)
- Modify: `omnilang/src/codegen/omniview/html.rs` (record tree during walk)

**Interfaces:**
- Produces: `ElementTreeNode { nid: String, tag: String, text: Option<String>, children: Vec<ElementTreeNode> }`
- Produces: `OmniViewOutput.element_tree: Vec<ElementTreeNode>`

- [ ] **Step 1: Add ElementTreeNode struct and element_tree to both OmniViewOutput and StyleResolveResult**

In `omnilang/src/codegen/omniview/mod.rs`, add before `StyleResolveResult`:

```rust
/// A single node in the rendered element tree, used by the Poiesis editor
/// to reconstruct the view hierarchy in React without parsing HTML strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementTreeNode {
    pub nid: String,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ElementTreeNode>,
}
```

Add `element_tree` field to `StyleResolveResult` (after `element_tags`):

```rust
    /// Structured element tree for React-side rendering.
    pub element_tags: FxHashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_tree: Vec<ElementTreeNode>,
```

Add `element_tree` field to `OmniViewOutput` struct (after `per_component_css`):

```rust
    pub per_component_css: FxHashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_tree: Vec<ElementTreeNode>,
```

In `lib.rs`, update `resolve_styles()` and `resolve_styles_multi()` to pass `element_tree`:

In `resolve_styles()` (around line 243), add:
```rust
            element_tree: gen_output.element_tree,
```

In `resolve_styles_multi()` (around line 283), add:
```rust
            element_tree: output.element_tree,
```

In the `OmniViewGenerator` struct, add the tree recording fields (after `per_component_css`):

```rust
    per_component_css: FxHashMap<String, String>,
    current_css_start: usize,
    component_style_classes: FxHashSet<String>,
    /// Stack of ElementTreeNode builders — each level collects children of the current parent.
    element_tree_stack: Vec<ElementTreeNode>,
    /// Root-level tree nodes accumulated during view_to_html.
    element_tree: Vec<ElementTreeNode>,
```

Initialize them in the `generate()` function's generator constructor (after `component_style_classes`):

```rust
    component_style_classes: FxHashSet::default(),
    element_tree_stack: Vec::new(),
    element_tree: Vec::new(),
```

- [ ] **Step 2: Add tree recording helpers to html.rs**

In `omnilang/src/codegen/omniview/html.rs`, add helper methods to the `impl OmniViewGenerator` block (before `next_nid`):

```rust
    /// Begin recording a tree node. Called when we emit an opening tag.
    fn tree_push(&mut self, nid: String, tag: String) {
        self.element_tree_stack.push(ElementTreeNode {
            nid,
            tag,
            text: None,
            children: Vec::new(),
        });
    }

    /// Add a text leaf node to the current parent's children.
    fn tree_text(&mut self, nid: String, text: String) {
        let node = ElementTreeNode {
            nid,
            tag: "#text".to_string(),
            text: Some(text),
            children: Vec::new(),
        };
        if let Some(parent) = self.element_tree_stack.last_mut() {
            parent.children.push(node);
        } else {
            self.element_tree.push(node);
        }
    }

    /// Finish recording a tree node. Pops the stack and appends to parent.
    fn tree_pop(&mut self) {
        if let Some(node) = self.element_tree_stack.pop() {
            if let Some(parent) = self.element_tree_stack.last_mut() {
                parent.children.push(node);
            } else {
                self.element_tree.push(node);
            }
        }
    }
```

Need to import `ElementTreeNode` in html.rs:

```rust
use super::ElementTreeNode;
```

- [ ] **Step 3: Hook tree recording into element_to_html**

In `element_to_html`, after `let nid = self.next_nid(&el.tag);` (line 64), add:

```rust
        self.tree_push(nid.clone(), html_tag.clone());
```

At the self-closing void element return points — after the `css_only` return (line 755) and after `return out;` for void tags (line 779), BEFORE each return, add `self.tree_pop();`.

For `css_only` void return, change:
```rust
                self.element_tags
                    .insert(key, html_tag.to_string());
                return String::new();
```
to:
```rust
                self.element_tags
                    .insert(key, html_tag.to_string());
                self.tree_pop();
                return String::new();
```

For non-css_only void return, after `return out;` (line 779), add before:
```rust
            self.tree_pop();
            return out;
```

For fragment (line 785-791), after the for loop over children, add:
```rust
            self.tree_pop(); // fragment has no wrapper, pop after children
```

For normal elements (line 793-821), after `let _ = writeln!(out, "{}</{}>", pad, html_tag);` (line 820), add:
```rust
            self.tree_pop();
```

- [ ] **Step 4: Hook tree recording into view_to_html for Text nodes**

In `view_to_html`, the `ViewTree::Text` arm (line 19-25), change:
```rust
            ViewTree::Text(text) => {
                if text.trim().is_empty() {
                    String::new()
                } else {
                    format!("{}{}\n", pad, escape_html(text))
                }
            }
```
to use next_nid for text nodes and record them:
```rust
            ViewTree::Text(text) => {
                if text.trim().is_empty() {
                    String::new()
                } else {
                    let tnid = self.next_nid("text");
                    self.tree_text(tnid, escape_html(text));
                    format!("{}{}\n", pad, escape_html(text))
                }
            }
```

- [ ] **Step 5: Assign element_tree from generator at end of generate()**

In `mod.rs` `generate()`, after all component generation is done and before returning `gen`, add:

```rust
    // Transfer accumulated element_tree from generator
    gen.element_tree = std::mem::take(&mut gen.element_tree);
```

- [ ] **Step 6: Rebuild WASM and verify**

Run: `cd omnilang && wasm-pack build --target web --out-dir ../poiesis/src/wasm --features wasm,codegen-omniview`

Expected: Build succeeds, `OmniViewOutput` serializes with `element_tree` field (empty for simple components, populated for components with views).

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/omniview/mod.rs omnilang/src/codegen/omniview/html.rs poiesis/src/wasm/
git commit -m "feat: add element_tree to WASM OmniViewOutput for React rendering"
```

---

### Task 2: Create useWasmCompile hook

**Files:**
- Create: `poiesis/src/components/design/useWasmCompile.ts`

**Interfaces:**
- Consumes: `rustCompiler` (existing singleton), `generateOmniFiles`, `emitDesignSource`
- Consumes: Zustand store (`elements`, `wires`, `rootOrder`, `activeDesign`, `componentDefs`)
- Produces: `useWasmCompile(): { output: StyleResolveResult | null, isCompiling: boolean, errors: string[] }`

- [ ] **Step 1: Update rustCompiler.ts TypeScript types and write the hook**

In `poiesis/src/commands/rustCompiler.ts`, add to the `StyleResolveResult` type (after `element_tags`):

```typescript
  /** Structured element tree for React rendering */
  element_tree?: { nid: string; tag: string; text?: string | null; children: any[] }[];
```

Export the element tree type via a new export line:

```typescript
export type { ..., StyleResolveResult };
```

Then create `poiesis/src/components/design/useWasmCompile.ts`:
import { useState, useEffect, useRef, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useComponentDefs } from "../../store/hooks";
import { rustCompiler, type StyleResolveResult } from "../../commands/rustCompiler";
import { generateOmniFiles } from "../../services/codegen/pipeline";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { CodeTab } from "../../core/types/codegraph";

export interface OmniViewOutput {
  css: string;
  component_css: Record<string, string>;
  element_tree: ElementTreeNode[];
  element_classes: Record<string, string[]>;
  element_tags: Record<string, string>;
  warnings: string[];
}

export interface ElementTreeNode {
  nid: string;
  tag: string;
  text?: string | null;
  children: ElementTreeNode[];
}

function useWasmCompile(): {
  output: OmniViewOutput | null;
  isCompiling: boolean;
  errors: string[];
} {
  const elements = useElements();
  const rootOrder = useRootOrder();
  const wires = usePoiesisStore((s) => s.wires);
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const componentDefs = useComponentDefs();

  const [output, setOutput] = useState<OmniViewOutput | null>(null);
  const [isCompiling, setIsCompiling] = useState(false);
  const [errors, setErrors] = useState<string[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastSourcesRef = useRef<string>("");

  useEffect(() => {
    if (!rustCompiler.isReady) return;
    if (elements.length === 0) {
      setOutput(null);
      return;
    }

    // 150ms debounce
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setIsCompiling(true);

      try {
        const codeTabs: CodeTab[] = [{
          id: "main", type: "logic", label: "Main", icon: "📄",
          graph: { elements, wires }, variables: [],
        }];

        const mainResult = generateOmniFiles({
          designElements: elements,
          designWires: wires,
          codeTabs,
          componentName: "main",
          rootOrder,
          filePath: "src/main.omni",
          designName: activeDesign?.name
            && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
            ? activeDesign.name : undefined,
        });

        const sources: Record<string, string> = {};
        if (mainResult.value) Object.assign(sources, mainResult.value);

        // Generate source for each custom component
        if (componentDefs) {
          for (const [compName, compDef] of Object.entries(componentDefs)) {
            if (!compDef.elements || compDef.elements.length === 0) continue;
            const { generateOmniFragment } = require("../../services/codegen/pipeline");
            const compResult = generateOmniFragment({
              componentName: compName,
              designElements: compDef.elements,
              designWires: compDef.wires || [],
              codeTabs: [],
              rootOrder: compDef.rootOrder,
              fromDefinition: true,
            });
            if (compResult.value) {
              sources[`src/components/${compName}.omni`] = compResult.value;
            }
          }
        }

        // Add design.omni
        const designSrc = activeDesign?.name
          && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
          ? emitDesignSource(activeDesign)
          : null;
        if (designSrc) sources["src/design.omni"] = designSrc;

        // Build path mappings
        const pathMapping: Record<string, string> = {
          "./design.omni": "src/design.omni",
        };
        if (componentDefs) {
          for (const compName of Object.keys(componentDefs)) {
            pathMapping[`./components/${compName}`] = `src/components/${compName}.omni`;
          }
        }

        // Skip compile if sources haven't changed
        const sourcesKey = JSON.stringify(sources);
        if (sourcesKey === lastSourcesRef.current) {
          setIsCompiling(false);
          return;
        }
        lastSourcesRef.current = sourcesKey;

        const result = rustCompiler.resolveStylesMulti(sources, pathMapping);

        if (result.success) {
          setOutput({
            css: result.design_css,
            component_css: result.component_css,
            element_tree: (result as any).element_tree || [],
            element_classes: result.element_classes,
            element_tags: result.element_tags,
            warnings: result.warnings,
          });
          setErrors([]);
        } else {
          setErrors(result.errors?.map((e: any) => e.message) || ["Compilation failed"]);
        }
      } catch (e) {
        setErrors([e instanceof Error ? e.message : String(e)]);
      } finally {
        setIsCompiling(false);
      }
    }, 150);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [elements, wires, rootOrder, activeDesign, componentDefs]);

  return { output, isCompiling, errors };
}

export { useWasmCompile };
```

- [ ] **Step 2: Verify type-check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors from useWasmCompile.ts

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/useWasmCompile.ts
git commit -m "feat: add useWasmCompile hook with debounced WASM compilation"
```

---

### Task 3: Create OmniViewElementTree and refactor OmniViewElement

**Files:**
- Create: `poiesis/src/components/design/OmniViewElementTree.tsx`
- Modify: `poiesis/src/components/design/OmniViewElement.tsx`

**Interfaces:**
- Consumes: `OmniViewOutput` from `useWasmCompile` (element_tree, element_classes, element_tags)
- Consumes: Zustand store `selectedElementIds`, `selectElement`
- Produces: Rendered React element tree with `data-nid` attributes and click-to-select

- [ ] **Step 1: Rewrite OmniViewElement.tsx**

Replace `poiesis/src/components/design/OmniViewElement.tsx`:

```typescript
// components/design/OmniViewElement.tsx
// React.memo component for rendering a single element tree node.
// Uses stable nid as React key for zero-flicker reconciliation.

import React from "react";
import type { ElementTreeNode } from "./useWasmCompile";

export interface OmniViewElementProps {
  node: ElementTreeNode;
  tag: string;
  className: string | undefined;
  isSelected: boolean;
  onSelect: (nid: string) => void;
  onDoubleClick: (nid: string) => void;
  classMap: ReadonlyMap<string, string[]>;
  tagMap: ReadonlyMap<string, string>;
}

export const OmniViewElement = React.memo(function OmniViewElement({
  node,
  tag,
  isSelected,
  onSelect,
  onDoubleClick,
}: OmniViewElementProps) {
  const style: React.CSSProperties = {
    outline: isSelected ? "2px solid #7C3AED" : "none",
    outlineOffset: "-2px",
    cursor: "pointer",
  };

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onSelect(node.nid);
  };

  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onDoubleClick(node.nid);
  };

  return React.createElement(
    tag,
    {
      key: node.nid,
      "data-nid": node.nid,
      className: node.tag === "#text" ? undefined : undefined, // className set by parent via CSS
      style,
      onClick: handleClick,
      onDoubleClick: handleDoubleClick,
    },
    node.text,
    ...node.children.map((child) => (
      <OmniViewElement
        key={child.nid}
        node={child}
        tag={child.tag === "#text" ? "span" : child.tag}
        isSelected={false}
        onSelect={onSelect}
        onDoubleClick={onDoubleClick}
      />
    )),
  );
});
```

- [ ] **Step 2: Create OmniViewElementTree.tsx**

```typescript
// components/design/OmniViewElementTree.tsx
// Renders the WASM-produced element tree using React.createElement.
// CSS is injected via <style> tags managed by useCssScopePool (Task 4).

import { useMemo, useCallback } from "react";
import { OmniViewElement } from "./OmniViewElement";
import type { OmniViewOutput, ElementTreeNode } from "./useWasmCompile";

export interface OmniViewElementTreeProps {
  output: OmniViewOutput;
  selectedNid: string | null;
  onSelectNid: (nid: string) => void;
  onDoubleClickNid: (nid: string) => void;
}

export function OmniViewElementTree({
  output,
  selectedNid,
  onSelectNid,
  onDoubleClickNid,
}: OmniViewElementTreeProps) {
  const classMap = useMemo(
    () => new Map(Object.entries(output.element_classes)),
    [output.element_classes],
  );

  const tagMap = useMemo(
    () => new Map(Object.entries(output.element_tags)),
    [output.element_tags],
  );

  if (output.element_tree.length === 0) {
    return (
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "center",
        height: "100%", color: "#94a3b8", fontSize: 13,
      }}>
        空画布 — 从组件面板拖入元素
      </div>
    );
  }

  return (
    <>
      {output.element_tree.map((node) => (
        <OmniViewElement
          key={node.nid}
          node={node}
          tag={tagMap.get(node.nid) || node.tag}
          className={(classMap.get(node.nid) || []).join(" ")}
          isSelected={node.nid === selectedNid}
          onSelect={onSelectNid}
          onDoubleClick={onDoubleClickNid}
        />
      ))}
    </>
  );
}
```

- [ ] **Step 3: Verify type-check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors from OmniViewElement.tsx or OmniViewElementTree.tsx

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/design/OmniViewElement.tsx poiesis/src/components/design/OmniViewElementTree.tsx
git commit -m "feat: add WASM-driven OmniViewElementTree with stable nid rendering"
```

---

### Task 4: Create useCssScopePool hook

**Files:**
- Create: `poiesis/src/components/design/useCssScopePool.ts`

**Interfaces:**
- Consumes: `OmniViewOutput.css`, `OmniViewOutput.component_css`
- Produces: `CssScopeElements` — a fragment of `<style>` elements to render inside ArtboardContent

- [ ] **Step 1: Write the hook**

```typescript
// poiesis/src/components/design/useCssScopePool.ts
import { useRef, useEffect, useCallback, type ReactNode } from "react";

function hashStr(s: string): number {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    hash = ((hash << 5) - hash) + ch;
    hash |= 0;
  }
  return hash;
}

interface UseCssScopePoolResult {
  /** Call inside ArtboardContent to render the <style> pool. */
  renderStyles: () => ReactNode;
  /** Force-update all scopes (call when output changes). */
  updateScopes: (css: string, componentCss: Record<string, string>) => void;
}

export function useCssScopePool(): UseCssScopePoolResult {
  const scopeRefs = useRef<Map<string, HTMLStyleElement>>(new Map());
  const lastHashes = useRef<{ design: number; components: Map<string, number> }>({
    design: 0,
    components: new Map(),
  });

  const upsertScope = useCallback((scopeId: string, css: string) => {
    let el = scopeRefs.current.get(scopeId);
    if (!el) {
      el = document.createElement("style");
      el.setAttribute("data-css-scope", scopeId);
      // Append to a known container — the caller's parent manages insertion
      scopeRefs.current.set(scopeId, el);
    }
    el.textContent = css;
    return el;
  }, []);

  const removeScope = useCallback((scopeId: string) => {
    const el = scopeRefs.current.get(scopeId);
    if (el) {
      el.remove();
      scopeRefs.current.delete(scopeId);
    }
  }, []);

  const updateScopes = useCallback(
    (css: string, componentCss: Record<string, string>) => {
      // Design CSS
      const designHash = hashStr(css);
      if (designHash !== lastHashes.current.design) {
        upsertScope("design", css);
        lastHashes.current.design = designHash;
      }

      // Component CSS — per component
      const newComponents = new Set(Object.keys(componentCss));
      for (const [name, compCss] of Object.entries(componentCss)) {
        const hash = hashStr(compCss);
        if (hash !== lastHashes.current.components.get(name)) {
          upsertScope(`component:${name}`, compCss);
          lastHashes.current.components.set(name, hash);
        }
      }

      // Remove scopes for deleted components
      for (const [name] of lastHashes.current.components) {
        if (!newComponents.has(name)) {
          removeScope(`component:${name}`);
          lastHashes.current.components.delete(name);
        }
      }
    },
    [upsertScope, removeScope],
  );

  const renderStyles = useCallback((): ReactNode => {
    // Return the style elements as an array for React to render
    // In practice, these are managed via DOM refs; the React side just needs
    // a container. We render placeholder divs that get replaced on mount.
    return null; // Managed via direct DOM — see ArtboardContent for container ref
  }, []);

  return { renderStyles, updateScopes };
}
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/components/design/useCssScopePool.ts
git commit -m "feat: add useCssScopePool hook for incremental CSS updates"
```

---

### Task 5: Refactor OmniViewRenderer to use new pipeline

**Files:**
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`
- Modify: `poiesis/src/components/design/render-utils.ts` (remove `buildInlineStyleMap`)

**Interfaces:**
- Consumes: `useWasmCompile` (Task 2), `useCssScopePool` (Task 4), `OmniViewElementTree` (Task 3)
- Consumes: Zustand store selectElement
- Produces: Self-contained renderer for the artboard content area

- [ ] **Step 1: Rewrite OmniViewRenderer.tsx**

```typescript
// components/design/OmniViewRenderer.tsx
// WASM-driven renderer — single compilation path, no TS fallback.
// Calls useWasmCompile for debounced WASM compilation, passes results
// to OmniViewElementTree for React rendering and useCssScopePool for CSS.

import { useCallback, useMemo, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { useWasmCompile } from "./useWasmCompile";
import { useCssScopePool } from "./useCssScopePool";
import { OmniViewElementTree } from "./OmniViewElementTree";

export function OmniViewRenderer() {
  const selectedElementIds = usePoiesisStore((s) => s.selectedElementIds);
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);

  const { output, isCompiling, errors } = useWasmCompile();
  const { updateScopes } = useCssScopePool();
  const styleContainerRef = useRef<HTMLDivElement>(null);

  // Update CSS scopes when output changes
  useEffect(() => {
    if (!output) return;
    updateScopes(output.css, output.component_css);
  }, [output, updateScopes]);

  // nid → elementId mapping (from sourceMap or element_classes keys)
  const selectedNid = useMemo(() => {
    if (!output || selectedElementIds.length === 0) return null;
    // element_classes maps element.id → string[]
    // Find the nid that matches the selected element ID
    for (const [id, classes] of Object.entries(output.element_classes)) {
      if (selectedElementIds.includes(id)) {
        // The nid = id (they match in the current WASM; for component expansion,
        // we'll map through sourceMap later)
        return id;
      }
    }
    return null;
  }, [output, selectedElementIds]);

  const handleSelectNid = useCallback(
    (nid: string) => {
      // Map nid → elementId: in current WASM, element_classes keys ARE element IDs
      // For expanded components, the mapping is nid → originalDefId
      selectElement(nid);
    },
    [selectElement],
  );

  const handleDoubleClickNid = useCallback(
    (nid: string) => {
      toggleCodeFlow(codeFlowVisible ? null : nid);
    },
    [toggleCodeFlow, codeFlowVisible],
  );

  return (
    <div
      data-renderer="omniview"
      style={{
        width: "100%", height: "100%", overflow: "auto",
        padding: 8, boxSizing: "border-box",
      }}
    >
      {/* CSS scope pool container */}
      <div ref={styleContainerRef} data-css-scopes="">
        {/* Managed by useCssScopePool via direct DOM */}
      </div>

      {errors.length > 0 && (
        <div style={{
          position: "absolute", bottom: 8, right: 8,
          background: "#fef2f2", border: "1px solid #fecaca",
          borderRadius: 6, padding: "6px 12px", fontSize: 11,
          color: "#dc2626", maxWidth: 320, zIndex: 10,
        }}>
          {errors[0]}
        </div>
      )}

      {isCompiling && (
        <div style={{
          position: "absolute", top: 8, right: 8,
          background: "#eff6ff", border: "1px solid #bfdbfe",
          borderRadius: 4, padding: "2px 8px", fontSize: 11,
          color: "#3b82f6", zIndex: 10,
        }}>
          compiling...
        </div>
      )}

      {output ? (
        <OmniViewElementTree
          output={output}
          selectedNid={selectedNid}
          onSelectNid={handleSelectNid}
          onDoubleClickNid={handleDoubleClickNid}
        />
      ) : (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "center",
          height: "100%", color: "#94a3b8", fontSize: 13,
        }}>
          空画布 — 从组件面板拖入元素
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Remove buildInlineStyleMap from render-utils.ts**

Remove the `buildInlineStyleMap` function from `poiesis/src/components/design/render-utils.ts` (lines 72-113). Keep `resolveTag`, `tagMap`, `blockTags`, `getDefaultInlineStyles`, `resolveSegmentsValue` as they may be used elsewhere.

- [ ] **Step 3: Verify type-check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No new errors. Fix any imports referencing removed `buildInlineStyleMap`.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/design/OmniViewRenderer.tsx poiesis/src/components/design/render-utils.ts
git commit -m "refactor: replace dual-path OmniViewRenderer with WASM-only pipeline"
```

---

### Task 6: Create canvas store slice

**Files:**
- Create: `poiesis/src/components/canvas/canvas-store.ts`
- Modify: `poiesis/src/store/types.ts` (add CanvasState types)
- Modify: `poiesis/src/store/index.ts` (add canvas slice)

**Interfaces:**
- Produces: `useCanvasStore` hook with zoom, pan, devicePreset, scroll state and actions

- [ ] **Step 1: Create canvas-store.ts**

```typescript
// poiesis/src/components/canvas/canvas-store.ts
import { create } from "zustand";

export interface DevicePreset {
  name: string;
  width: number;
  height: number; // 0 = auto (fit content)
}

export const DEVICE_PRESETS: DevicePreset[] = [
  { name: "iPhone 15", width: 393, height: 852 },
  { name: "Pixel 8", width: 412, height: 915 },
  { name: "iPad", width: 768, height: 1024 },
  { name: "iPad Pro", width: 1024, height: 1366 },
  { name: "Desktop", width: 1440, height: 900 },
  { name: "Full HD", width: 1920, height: 1080 },
  { name: "Custom", width: 0, height: 0 },
];

export interface CanvasState {
  zoom: number;
  panX: number;
  panY: number;
  devicePreset: DevicePreset;
  artboardScrollTop: number;
  artboardScrollLeft: number;
  setZoom: (zoom: number) => void;
  setPan: (x: number, y: number) => void;
  setDevicePreset: (preset: DevicePreset) => void;
  setArtboardScroll: (top: number, left: number) => void;
  zoomAtPoint: (newZoom: number, cursorX: number, cursorY: number) => void;
  fitToView: (viewportW: number, viewportH: number) => void;
  resetView: () => void;
}

export const useCanvasStore = create<CanvasState>((set, get) => ({
  zoom: 1.0,
  panX: 0,
  panY: 0,
  devicePreset: DEVICE_PRESETS[0], // iPhone 15 default
  artboardScrollTop: 0,
  artboardScrollLeft: 0,

  setZoom: (zoom) => set({ zoom: Math.max(0.1, Math.min(5.0, zoom)) }),

  setPan: (x, y) => set({ panX: x, panY: y }),

  setDevicePreset: (preset) => set({ devicePreset: preset, artboardScrollTop: 0, artboardScrollLeft: 0 }),

  setArtboardScroll: (top, left) => set({ artboardScrollTop: top, artboardScrollLeft: left }),

  zoomAtPoint: (newZoom, cursorX, cursorY) => {
    const state = get();
    const clamped = Math.max(0.1, Math.min(5.0, newZoom));
    const scale = clamped / state.zoom;
    set({
      zoom: clamped,
      panX: cursorX - scale * (cursorX - state.panX),
      panY: cursorY - scale * (cursorY - state.panY),
    });
  },

  fitToView: (viewportW, viewportH) => {
    const { devicePreset } = get();
    const scaleX = viewportW / devicePreset.width;
    const scaleY = viewportH / (devicePreset.height || 800);
    const zoom = Math.min(scaleX, scaleY) * 0.9;
    const panX = (viewportW - devicePreset.width * zoom) / 2;
    const panY = (viewportH - (devicePreset.height || 800) * zoom) / 2;
    set({ zoom: Math.max(0.1, Math.min(5.0, zoom)), panX, panY });
  },

  resetView: () => set({ zoom: 1.0, panX: 0, panY: 0, artboardScrollTop: 0, artboardScrollLeft: 0 }),
}));
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/components/canvas/canvas-store.ts
git commit -m "feat: add canvas store slice with zoom/pan/device-preset"
```

---

### Task 7: Create InfiniteCanvas and Artboard components

**Files:**
- Create: `poiesis/src/components/canvas/InfiniteCanvas.tsx`
- Create: `poiesis/src/components/canvas/CanvasGrid.tsx`
- Create: `poiesis/src/components/canvas/Artboard.tsx`
- Create: `poiesis/src/components/canvas/ArtboardHeader.tsx`
- Create: `poiesis/src/components/canvas/ArtboardContent.tsx`
- Create: `poiesis/src/components/canvas/ZoomControls.tsx`

**Interfaces:**
- Consumes: `useCanvasStore` (Task 6)
- Produces: Complete infinite canvas with zoom/pan, artboard, and zoom controls

- [ ] **Step 1: Create CanvasGrid.tsx**

```typescript
// poiesis/src/components/canvas/CanvasGrid.tsx
import React from "react";

export const CanvasGrid = React.memo(function CanvasGrid() {
  return (
    <div
      style={{
        position: "absolute",
        inset: 0,
        backgroundImage: `
          radial-gradient(circle, #d1d5db 1px, transparent 1px)
        `,
        backgroundSize: "20px 20px",
        pointerEvents: "none",
        zIndex: 0,
      }}
    />
  );
});
```

- [ ] **Step 2: Create ArtboardHeader.tsx**

```typescript
// poiesis/src/components/canvas/ArtboardHeader.tsx
import React from "react";
import { useCanvasStore, DEVICE_PRESETS } from "./canvas-store";

export const ArtboardHeader = React.memo(function ArtboardHeader() {
  const devicePreset = useCanvasStore((s) => s.devicePreset);
  const setDevicePreset = useCanvasStore((s) => s.setDevicePreset);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: "4px 12px",
        background: "#f1f5f9",
        borderBottom: "1px solid #e2e8f0",
        fontSize: 11,
        color: "#64748b",
        flexShrink: 0,
      }}
    >
      <select
        value={devicePreset.name}
        onChange={(e) => {
          const preset = DEVICE_PRESETS.find((p) => p.name === e.target.value);
          if (preset) setDevicePreset(preset);
        }}
        style={{
          border: "1px solid #e2e8f0",
          borderRadius: 4,
          padding: "2px 6px",
          fontSize: 11,
          fontFamily: "inherit",
          background: "#fff",
        }}
      >
        {DEVICE_PRESETS.map((p) => (
          <option key={p.name} value={p.name}>{p.name}</option>
        ))}
      </select>
      <span>
        {devicePreset.width} × {devicePreset.height || "auto"}
      </span>
    </div>
  );
});
```

- [ ] **Step 3: Create ArtboardContent.tsx**

```typescript
// poiesis/src/components/canvas/ArtboardContent.tsx
import React, { useRef, useEffect } from "react";
import { useCanvasStore } from "./canvas-store";
import { useCssScopePool } from "../design/useCssScopePool";

export interface ArtboardContentProps {
  children: React.ReactNode;
}

export const ArtboardContent = React.memo(function ArtboardContent({
  children,
}: ArtboardContentProps) {
  const setArtboardScroll = useCanvasStore((s) => s.setArtboardScroll);
  const scrollRef = useRef<HTMLDivElement>(null);
  const styleContainerRef = useRef<HTMLDivElement>(null);

  // Listen to scroll events on the artboard content
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const handler = () => {
      setArtboardScroll(el.scrollTop, el.scrollLeft);
    };
    el.addEventListener("scroll", handler, { passive: true });
    return () => el.removeEventListener("scroll", handler);
  }, [setArtboardScroll]);

  return (
    <div
      ref={scrollRef}
      style={{
        flex: 1,
        overflow: "auto",
        position: "relative",
      }}
    >
      {/* CSS scope pool container — managed by useCssScopePool */}
      <div ref={styleContainerRef} data-css-scopes="" />
      {children}
    </div>
  );
});
```

- [ ] **Step 4: Create Artboard.tsx**

```typescript
// poiesis/src/components/canvas/Artboard.tsx
import React from "react";
import { useCanvasStore } from "./canvas-store";
import { ArtboardHeader } from "./ArtboardHeader";
import { ArtboardContent } from "./ArtboardContent";

export interface ArtboardProps {
  children: React.ReactNode;
}

export const Artboard = React.memo(function Artboard({ children }: ArtboardProps) {
  const devicePreset = useCanvasStore((s) => s.devicePreset);

  return (
    <div
      style={{
        position: "absolute",
        // Center in canvas
        left: "50%",
        top: "50%",
        transform: "translate(-50%, -50%)",
        width: devicePreset.width || undefined,
        minHeight: devicePreset.height || undefined,
        maxHeight: devicePreset.height || undefined,
        background: "#fff",
        boxShadow: "0 0 0 1px #e2e8f0, 0 4px 24px rgba(0,0,0,0.08)",
        borderRadius: 4,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <ArtboardHeader />
      <ArtboardContent>
        {children}
      </ArtboardContent>
    </div>
  );
});
```

- [ ] **Step 5: Create ZoomControls.tsx**

```typescript
// poiesis/src/components/canvas/ZoomControls.tsx
import React, { useCallback } from "react";
import { useCanvasStore } from "./canvas-store";

export const ZoomControls = React.memo(function ZoomControls() {
  const zoom = useCanvasStore((s) => s.zoom);
  const setZoom = useCanvasStore((s) => s.setZoom);
  const fitToView = useCanvasStore((s) => s.fitToView);
  const resetView = useCanvasStore((s) => s.resetView);

  const handleFit = useCallback(() => {
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    fitToView(vw - 600, vh - 120); // account for panels
  }, [fitToView]);

  return (
    <div
      style={{
        position: "absolute",
        bottom: 12,
        right: 12,
        display: "flex",
        alignItems: "center",
        gap: 4,
        background: "#fff",
        border: "1px solid #e2e8f0",
        borderRadius: 8,
        padding: "4px 8px",
        boxShadow: "0 1px 4px rgba(0,0,0,0.06)",
        zIndex: 10,
        fontSize: 12,
        fontFamily: "inherit",
      }}
    >
      <button
        onClick={() => setZoom(zoom - 0.1)}
        style={btnStyle}
        title="Zoom out"
      >−</button>
      <span
        style={{ minWidth: 48, textAlign: "center", color: "#64748b", cursor: "pointer" }}
        onClick={resetView}
        title="Reset zoom"
      >
        {Math.round(zoom * 100)}%
      </span>
      <button
        onClick={() => setZoom(zoom + 0.1)}
        style={btnStyle}
        title="Zoom in"
      >+</button>
      <button
        onClick={handleFit}
        style={{ ...btnStyle, marginLeft: 4 }}
        title="Fit to view"
      >
        ⊞
      </button>
    </div>
  );
});

const btnStyle: React.CSSProperties = {
  width: 24,
  height: 24,
  border: "none",
  background: "transparent",
  cursor: "pointer",
  borderRadius: 4,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  fontSize: 14,
  color: "#64748b",
};
```

- [ ] **Step 6: Create InfiniteCanvas.tsx**

```typescript
// poiesis/src/components/canvas/InfiniteCanvas.tsx
import React, { useCallback, useRef } from "react";
import { useCanvasStore } from "./canvas-store";
import { CanvasGrid } from "./CanvasGrid";
import { Artboard } from "./Artboard";
import { ZoomControls } from "./ZoomControls";

export interface InfiniteCanvasProps {
  children: React.ReactNode;
}

export function InfiniteCanvas({ children }: InfiniteCanvasProps) {
  const zoom = useCanvasStore((s) => s.zoom);
  const panX = useCanvasStore((s) => s.panX);
  const panY = useCanvasStore((s) => s.panY);
  const setPan = useCanvasStore((s) => s.setPan);
  const zoomAtPoint = useCanvasStore((s) => s.zoomAtPoint);

  const isPanning = useRef(false);
  const lastPointer = useRef({ x: 0, y: 0 });
  const spaceHeld = useRef(false);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const rect = e.currentTarget.getBoundingClientRect();
        const newZoom = zoom * (1 - e.deltaY * 0.001);
        zoomAtPoint(newZoom, e.clientX - rect.left, e.clientY - rect.top);
      }
      // Plain scroll: let it bubble to ArtboardContent (overflow:auto)
    },
    [zoom, zoomAtPoint],
  );

  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      // Space+drag or middle-button drag = pan
      if (spaceHeld.current || e.button === 1) {
        e.preventDefault();
        isPanning.current = true;
        lastPointer.current = { x: e.clientX, y: e.clientY };
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
      }
    },
    [],
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!isPanning.current) return;
      const dx = e.clientX - lastPointer.current.x;
      const dy = e.clientY - lastPointer.current.y;
      lastPointer.current = { x: e.clientX, y: e.clientY };
      setPan(panX + dx, panY + dy);
    },
    [panX, panY, setPan],
  );

  const handlePointerUp = useCallback(() => {
    isPanning.current = false;
  }, []);

  // Keyboard: space for pan mode
  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "Space" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement)) {
        e.preventDefault();
        spaceHeld.current = true;
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "Space") spaceHeld.current = false;
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);

  return (
    <div
      style={{
        flex: 1,
        position: "relative",
        overflow: "hidden",
        cursor: spaceHeld.current ? "grab" : "default",
        background: "#f8fafc",
      }}
      onWheel={handleWheel}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      <CanvasGrid />

      {/* Transformed canvas layer */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          transform: `scale(${zoom}) translate(${panX / zoom}px, ${panY / zoom}px)`,
          transformOrigin: "0 0",
        }}
      >
        <Artboard>
          {children}
        </Artboard>
      </div>

      <ZoomControls />
    </div>
  );
}
```

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/canvas/
git commit -m "feat: add InfiniteCanvas with Artboard, zoom, pan, and device presets"
```

---

### Task 8: Integrate InfiniteCanvas into DesignPage

**Files:**
- Modify: `poiesis/src/pages/DesignPage.tsx`

**Interfaces:**
- Replaces `DesignCanvas` import with `InfiniteCanvas`
- OmniViewRenderer now renders inside ArtboardContent

- [ ] **Step 1: Update DesignPage.tsx**

In `poiesis/src/pages/DesignPage.tsx`:

Replace import:
```typescript
import { DesignCanvas } from "../components/design/DesignCanvas";
```
with:
```typescript
import { InfiniteCanvas } from "../components/canvas/InfiniteCanvas";
```

Replace usage (line 281):
```typescript
<DesignCanvas />
```
with:
```typescript
<InfiniteCanvas>
  <OmniViewRenderer />
</InfiniteCanvas>
```

The CodeFlowCanvas toggle behavior also needs adjustment — remove the Z-axis dual-layer from old DesignCanvas and make CodeFlow a separate overlay or drawer. For now, keep it accessible via the same double-click mechanism but as a modal/overlay instead of a Z-axis layer. This is a minimal change: the CodeFlow button on the toolbar still works. We'll handle CodeFlow integration in a follow-up task.

- [ ] **Step 2: Verify build and type-check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No new errors. Fix any missing imports.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/pages/DesignPage.tsx
git commit -m "feat: replace DesignCanvas with InfiniteCanvas in DesignPage"
```

---

### Task 9: Cleanup — remove old code

**Files:**
- Delete: `poiesis/src/components/design/DesignCanvas.tsx`
- Modify: `poiesis/src/components/design/render-utils.ts` (remove unused exports)
- Modify: `poiesis/src/services/codegen/expand-custom-components.ts` (remove canvas-specific unused code if any)

- [ ] **Step 1: Delete DesignCanvas.tsx**

```bash
git rm poiesis/src/components/design/DesignCanvas.tsx
```

- [ ] **Step 2: Clean render-utils.ts**

Remove `buildInlineStyleMap` if not already removed in Task 5. Also remove `getDefaultInlineStyles` and `resolveSegmentsValue` if no longer imported by any other file.

Verify no imports remain:
Run: `cd poiesis && grep -r "buildInlineStyleMap\|getDefaultInlineStyles" src/ --include="*.ts" --include="*.tsx"`

If no results, remove those functions. Otherwise, note which files still use them and handle individually.

- [ ] **Step 3: Verify build succeeds**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | tail -5`
Expected: No errors related to deleted files.

- [ ] **Step 4: Commit**

```bash
git add -A poiesis/src/
git commit -m "chore: remove old DesignCanvas and dead rendering code"
```

---

### Task 10: End-to-end verification

**Files:** None (verification only)

- [ ] **Step 1: Start poiesis and open a project**

Using start-poiesis skill, launch the editor and open a test project with components.

- [ ] **Step 2: Verify canvas rendering**

Expected:
- Components render inside a white artboard centered on a grid canvas
- Elements display with correct styles (from WASM CSS)
- Clicking an element selects it (purple outline, inspector updates)
- Ctrl+scroll zooms in/out
- Space+drag pans the canvas
- Device preset dropdown changes artboard size

- [ ] **Step 3: Verify no style flicker**

Actions to test:
1. Change an element's label text → only text updates, no style flash
2. Change a design token value → only design scope `<style>` updates
3. Add a new element → artboard grows, new element renders without flicker
4. Switch device preset → artboard resizes smoothly

- [ ] **Step 4: Verify compilation output unchanged**

Run: `cd omnilang && cargo test -p omnilang --features wasm,codegen-omniview`
Expected: All existing tests pass (OmniViewOutput serialization is backward-compatible with new optional fields).

- [ ] **Step 5: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final verification fixes"
```
