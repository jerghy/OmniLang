# Poiesis Omni Codegen Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix Poiesis-generated .omni code to produce clean component references instead of expanding components inline, eliminate the `forSave` dual-mode from the pipeline, and cleanup code quality issues (naming, imports, PID leakage, redundant wrappers).

**Architecture:** Eliminate the `expandCustomComponents` call from the pipeline — expansion becomes a canvas-rendering-only concern. SourceEmitter emits clean code always (PascalCase tags self-close, no inline children). WASM receives ALL component files (via existing `resolve_styles_multi`) so it resolves cross-file styles naturally. Canvas rendering reuses `expandCustomComponents` from codegen (deleting the duplicate in OmniViewRenderer) and translates WASM class IDs from original→expanded.

**Tech Stack:** TypeScript, React, WASM (Rust/OmniLang compiler)

## Global Constraints

- All generated `.omni` source must pass `omnic build --check` without errors
- Component references must be self-closing `<MyCard />`, never with inlined children
- PID annotations (`// @pid:...`) suppressed in code panel and saved output
- Component names must be PascalCase
- `import { Design } from "./design.omni"` only emitted when design classes are actually used in the view
- Single-root views must not be wrapped in a redundant `<div>`

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `poiesis/src/services/codegen/types.ts` | Modify | Add `ExpansionResult` interface |
| `poiesis/src/services/codegen/expand-custom-components.ts` | Modify | Return `idMap` + `customChildIds`, export for reuse |
| `poiesis/src/services/codegen/source-emitter.ts` | Modify | PascalCase self-close, PascalCase component name, suppressible PID |
| `poiesis/src/services/codegen/component-builder.ts` | Modify | Single-root skip wrapper |
| `poiesis/src/services/codegen/import-collector.ts` | Modify | Conditional design import |
| `poiesis/src/services/codegen/pipeline.ts` | Modify | Remove `forSave`, fix `generateOmniFragment` |
| `poiesis/src/services/codegen/pipeline-types.ts` | Modify | Remove `forSave` from type interfaces |
| `poiesis/src/components/design/OmniViewRenderer.tsx` | Modify | Multi-file WASM, reuse expansion, delete duplicate |
| `poiesis/src/hooks/useLiveCodePreview.ts` | Modify | Remove `forSave` |
| `poiesis/src/hooks/useSaveProject.ts` | Modify | Remove `forSave` |

---

### Task 1: `expand-custom-components.ts` — Return ID mappings

**Files:**
- Modify: `poiesis/src/services/codegen/types.ts:293` (append)
- Modify: `poiesis/src/services/codegen/expand-custom-components.ts:40-116`

**Interfaces:**
- Consumes: `ElementData`, `ComponentDef` (from store)
- Produces: `ExpansionResult` interface, updated `expandCustomComponents()` signature

- [ ] **Step 1: Add `ExpansionResult` type to types.ts**

Append to `poiesis/src/services/codegen/types.ts`:

```typescript
/** Result of expanding custom component references into a flat element set. */
export interface ExpansionResult {
  /** All elements including cloned component-internal elements. */
  elements: ElementData[];
  /** IDs of elements that are inside custom components (non-selectable in canvas). */
  customChildIds: Set<string>;
  /** Maps expanded element ID → original definition element ID (for WASM class translation). */
  idMappings: Map<string, string>;
}
```

Note: `ElementData` is already imported in types.ts from `../../core/types`. If not, add `import type { ElementData } from "../../core/types";`.

- [ ] **Step 2: Update `expandCustomComponents` signature and return value**

In `poiesis/src/services/codegen/expand-custom-components.ts`, replace the function signature (line 40) and return (line 115):

```typescript
import type { ExpansionResult } from "./types";

export function expandCustomComponents(elements: ElementData[]): ExpansionResult {
  const registry = getComponentDefinitions();
  const customChildIds = new Set<string>();
  // idMappings: expandedId → original definition element ID
  const idMappings = new Map<string, string>();

  const hasRefs = elements.some(
    (el) =>
      ("componentRef" in el && el.componentRef) ||
      ("tag" in el && registry[el.tag]),
  );
  if (!hasRefs) return { elements, customChildIds, idMappings };

  const elementMap = new Map<string, ElementData>();
  for (const el of elements) {
    elementMap.set(el.id, { ...el });
  }

  const expandedRefs = new Set<string>();
  let foundNew = true;
  while (foundNew) {
    foundNew = false;
    for (const [id, el] of [...elementMap]) {
      const compRef: string | undefined =
        ("componentRef" in el ? el.componentRef : undefined) ||
        ("tag" in el && registry?.[el.tag] ? el.tag : undefined);
      if (!compRef || expandedRefs.has(id)) continue;
      expandedRefs.add(id);

      const def = registry?.[compRef];
      const defElements = def ? getDefElements(def) : null;
      if (!defElements || defElements.length === 0) continue;

      const idMap = new Map<string, string>();
      let idx = 0;
      for (const defEl of defElements) {
        const newId = `${el.id}__${idx}`;
        idMap.set(defEl.id, newId);
        // Record reverse mapping for WASM class translation
        idMappings.set(newId, defEl.id);
        idx++;
      }

      const rootIds: string[] = [];
      for (const defEl of defElements) {
        const newId = idMap.get(defEl.id)!;
        const isRoot = defEl.parentId === null;

        const cloned: ElementData = {
          ...defEl,
          id: newId,
          nodeType: "view" as const,
          tag: defEl.tag ?? "div",
          props: defEl.props ?? {},
          parentId: isRoot ? el.id : idMap.get(defEl.parentId ?? "") || defEl.parentId,
          children: defEl.children.map((cid) => idMap.get(cid) || cid),
        } as unknown as ElementData;

        elementMap.set(newId, cloned);
        customChildIds.add(newId);
        foundNew = true;
        if (isRoot) rootIds.push(newId);
      }

      const container = elementMap.get(id)!;
      const updatedContainer = { ...container };
      let changed = false;
      if ("children" in updatedContainer) {
        for (const rootId of rootIds) {
          if (!updatedContainer.children.includes(rootId)) {
            updatedContainer.children = [...updatedContainer.children, rootId];
            changed = true;
          }
        }
      }
      if (changed) elementMap.set(id, updatedContainer);
    }
  }

  return { elements: [...elementMap.values()], customChildIds, idMappings };
}
```

Key changes from original:
- `customChildIds` is now populated inside this function (moved from OmniViewRenderer)
- `idMappings: Map<expandedId, originalDefId>` returned for WASM class translation
- `idMap` (local variable) renamed but logic unchanged

- [ ] **Step 3: Verify TypeScript compiles**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

Expected: no new errors from `expand-custom-components.ts` or `types.ts`.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/expand-custom-components.ts
git commit -m "refactor: return idMappings from expandCustomComponents for WASM class translation"
```

---

### Task 2: `source-emitter.ts` — PascalCase self-close + PascalCase name + suppressible PID

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts`

**Interfaces:**
- Consumes: `EmitInput`, `AnnotationStrategy`, `ViewNode`
- Produces: Clean `.omni` source with component tags self-closing

- [ ] **Step 1: PascalCase component tags self-close in `emitViewNode`**

In `source-emitter.ts`, inside `emitViewNode()`, right after computing `hasContent` (line ~431), add a PascalCase check that short-circuits before child emission:

```typescript
// Determine if this is a custom component reference (PascalCase tag)
const isComponentTag = /^[A-Z]/.test(tag);

if (isVoid || !hasContent) {
  w.emit(isVoid ? `${openTag} />` : `${openTag}></${tag}>`);
  return;
}

// Custom components: always self-close — their internal template
// is defined in their own .omni file. Expanded children are a
// canvas-rendering artifact, not part of the component's source API.
if (isComponentTag) {
  w.emit(`${openTag} />`);
  return;
}

w.emit(`${openTag}>`);
// ... rest of existing code (children emission)
```

Insert after line 435 (`return;`) and before line 438 (`w.emit(\`\${openTag}>\`);`).

- [ ] **Step 2: PascalCase component name in `emitComponent`**

In `emitComponent()` (line ~96), replace:

```typescript
w.emit(`component ${comp.name} {`);
```

With:

```typescript
// PascalCase: ensure first character is uppercase
const pascalName = comp.name.charAt(0).toUpperCase() + comp.name.slice(1);
w.emit(`component ${pascalName} {`);
```

- [ ] **Step 3: Make PID annotations suppressible**

Modify `SourceEmitter` constructor to accept an `enableAnnotations` flag, and guard annotation emission in `emitViewNode`:

In the constructor (line ~22-28):

```typescript
export class SourceEmitter {
  constructor(
    private annotationStrategy?: AnnotationStrategy,
    private elementClassMap?: Map<string, string[]>,
    private elementInlineMap?: Map<string, Record<string, string>>,
    private elementBindings?: Map<string, Record<string, string>>,
    private enableAnnotations: boolean = false,  // ← new, default false
  ) {}
```

In `emitViewNode` (line ~236-238), guard the annotation emission:

```typescript
if (this.enableAnnotations && this.annotationStrategy && node.kind !== "text" && node.kind !== "interpolation") {
  const ann = this.annotationStrategy.beforeViewNode(node, depth);
  if (ann) w.emit(`// @pid:${ann.pid}`);
}
```

In `emitDocument` (line ~34-37), guard the file directive:

```typescript
if (this.enableAnnotations && this.annotationStrategy) {
  const directive = this.annotationStrategy.beforeFile();
  if (directive) w.emit(directive);
}
```

- [ ] **Step 4: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

Expected: no new errors.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts
git commit -m "feat: PascalCase self-close, PascalCase component name, suppressible PID annotations"
```

---

### Task 3: `component-builder.ts` — Skip root wrapper for single root

**Files:**
- Modify: `poiesis/src/services/codegen/component-builder.ts:118-140`

**Interfaces:**
- Consumes: `PhaseAContext.roots`
- Produces: `ComponentPart.view` (ViewNode)

- [ ] **Step 1: Single root — use directly, skip wrapper div**

In `buildComponentPart()`, replace the view construction block (lines 118-140):

```typescript
const view =
  phaseA.roots.length === 1
    ? phaseA.roots[0]  // ← single root: use as-is, no wrapping div
    : phaseA.roots.length > 0
      ? {
          id: viewWrapperId,
          path: viewWrapperId,
          tag: "div",
          props: {},
          children: phaseA.roots,
          kind: "element" as const,
          resolvedStyle: {
            htmlTag: "div",
            cssClassName: `${options.componentName.toLowerCase()}_view`,
            contentText: undefined,
            htmlAttrs: {},
            eventBindings: {},
            bindings: {},
            cssRules: {},
            usedClassNames: [],
          },
        } as ElementViewNode
      : null;
```

Logic change: `roots.length === 1` now checked first → uses root directly without wrapper. Only wraps when there are 2+ roots.

- [ ] **Step 2: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/component-builder.ts
git commit -m "refactor: skip root wrapper div when single root element"
```

---

### Task 4: `import-collector.ts` — Conditional design import

**Files:**
- Modify: `poiesis/src/services/codegen/import-collector.ts`

**Interfaces:**
- Consumes: `pascalCaseTags`, `storeRefs`, `codeTabs`, `designName`, new `hasDesignClassUsage`
- Produces: `string[]` (import statements)

- [ ] **Step 1: Add `hasDesignClassUsage` parameter**

Replace the function signature and body:

```typescript
export function collectImports(
  pascalCaseTags: string[],
  storeRefs: string[],
  codeTabs: CodeTab[],
  designName?: string,
  hasDesignClassUsage?: boolean,  // ← new param
): string[] {
  const imports: string[] = ["import omniview;"];

  // Only import design if name is provided AND it's actually used
  if (designName && hasDesignClassUsage) {
    imports.push(`import { ${designName} } from "./design.omni";`);
  }

  for (const tag of pascalCaseTags) {
    imports.push(`import { ${tag} } from "./components/${tag}";`);
  }

  for (const storeName of storeRefs) {
    imports.push(`import { ${storeName} } from "./stores/${storeName}";`);
  }

  // ... rest unchanged (cross-tab function references)
```

- [ ] **Step 2: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/import-collector.ts
git commit -m "feat: add hasDesignClassUsage parameter for conditional design import"
```

---

### Task 5: `pipeline.ts` — Remove `forSave`, fix `generateOmniFragment`

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`
- Modify: `poiesis/src/services/codegen/pipeline-types.ts`

**Interfaces:**
- Consumes: `GenerateOmniSourceOptions` (without `forSave`), `GenerateOmniFragmentOptions`
- Produces: `StageResult<string>`, `StageResult<Record<string, string>>`

- [ ] **Step 1: Update type interfaces in `pipeline-types.ts`**

Remove `forSave?: boolean;` from both `GenerateOmniSourceOptions` (line 18) and `GenerateOmniFilesOptions` (line 35).

Add `fromDefinition?: boolean;` to `GenerateOmniFragmentOptions` (after line 51):

```typescript
export interface GenerateOmniFragmentOptions {
  componentName: string;
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  rootOrder?: string[];
  existingPidMap?: PidMap;
  designName?: string;
  /** When true, elements already belong to this component (from ComponentDef). Skip filtering. */
  fromDefinition?: boolean;
}
```

- [ ] **Step 2: Remove `forSave` logic from `generateOmniSource`**

In `pipeline.ts` `generateOmniSource()` (line 241-294):

```typescript
export function generateOmniSource(options: GenerateOmniSourceOptions): StageResult<string> {
  // No expansion — components stay as references. Expansion is a canvas concern.
  const phaseA = runPhaseA(options, options.designElements);  // ← pass original elements, not expanded

  if (phaseA.errors.some((e) => e.severity === "fatal")) {
    return { value: null, errors: phaseA.errors };
  }

  const phaseB = runPhaseB(options);
  const phaseC = runPhaseC(options, phaseA.ctx);

  // ... rest unchanged
```

Remove lines 242-245 (the `shouldExpand` / `expandedElements` block) and pass `options.designElements` directly to `runPhaseA`.

- [ ] **Step 3: Remove `shouldExpand` from `runPhaseA` — keep all PascalCase tags**

Previously, when `forSave: false`, registered components were filtered OUT of `pascalCaseTags` because they were expanded inline. Now that expansion is removed, ALL PascalCase tags need imports — they reference components in separate .omni files.

Replace the function body:

```typescript
function runPhaseA(
  options: GenerateOmniSourceOptions,
  elements: ElementData[],
): { ctx: PhaseAContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];

  const a1 = runPhase("A", "view-tree-builder", () =>
    buildViewTree(elements, options.componentName, options.rootOrder),
  );
  errors.push(...a1.errors);
  const viewTreeResult = a1.value ?? defaultViewTreeResult();
  const { roots, pascalCaseTags } = viewTreeResult;
  // All PascalCase tags are kept — they're component references that need imports.
  // (No filtering of registry components — expansion no longer substitutes for imports.)

  const a2 = runPhase("A", "wire-analyzer", () =>
    analyzeWires(options.designElements, options.designWires),
  );
  // ... rest unchanged from original, using pascalCaseTags directly in collectImports
```

- [ ] **Step 4: Remove `expandCustomComponents` call from `runPhaseC`**

In `runPhaseC()` (line 178-237), remove lines 183-186 (the `shouldExpand` / `expandCustomComponents` block). Use `options.designElements` directly in the `collectStylesFromEntries` call:

```typescript
function runPhaseC(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
): { ctx: PhaseCContext; errors: PipelineError[] } {
  const errors: PipelineError[] = [];
  // No expansion — components stay as references, styles only from flat elements

  const c1 = runPhase("C", "style-resolver", () => {
    applyStyles(phaseA.roots);
  });
  errors.push(...c1.errors);

  // ... elementBindings block unchanged ...

  const styleCollection = c1.errors.length > 0
    ? defaultStyleCollection()
    : collectStylesFromEntries(
        options.designElements,  // ← use original elements, not expanded
        { variables: [], classes: [] },
        phaseA.roots,
        options.designName,
      );
  // ... rest unchanged
```

- [ ] **Step 5: Compute `hasDesignClassUsage` and pass to `collectImports`**

In the refactored `runPhaseA()` from Step 3, add the design class usage check before the `collectImports` call:

```typescript
// Check if any elements reference design classes (for conditional design import)
const hasDesignClassUsage = options.designElements.some(
  (el) => "classRefs" in el && Array.isArray(el.classRefs) && el.classRefs.length > 0,
);

const a3 = runPhase("A", "import-collector", () =>
  collectImports(
    pascalCaseTags,
    [...bindings.storeRefs.keys()],
    options.codeTabs,
    options.designName,
    hasDesignClassUsage,
  ),
);
```

- [ ] **Step 6: Fix `generateOmniFragment` for ComponentDef elements**

Replace the filtering logic in `generateOmniFragment` (lines 362-386):

```typescript
export function generateOmniFragment(options: {
  componentName: string;
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  rootOrder?: string[];
  /** When true, elements already belong to this component (from ComponentDef). Skip filtering. */
  fromDefinition?: boolean;
}): StageResult<string> {
  const { componentName, designElements, designWires, codeTabs, rootOrder, fromDefinition } = options;

  // If elements come from a ComponentDef, they already belong to this component — no filter
  const componentElements = fromDefinition
    ? designElements
    : designElements.filter((el) => {
        const ref = "componentRef" in el ? el.componentRef : undefined;
        if (componentName === "App" || componentName === "main") {
          return !ref || ref === componentName;
        }
        return ref === componentName;
      });

  if (componentElements.length === 0) {
    return { value: "", errors: [] };
  }

  return generateOmniSource({
    designElements: componentElements,
    designWires,
    codeTabs,
    componentName,
    rootOrder,
  });
}
```

- [ ] **Step 7: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

Expected: errors from callers that still pass `forSave` (will be fixed in Tasks 7-8).

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts poiesis/src/services/codegen/pipeline-types.ts
git commit -m "refactor: remove forSave dual-mode from pipeline, fix generateOmniFragment"
```

---

### Task 6: `OmniViewRenderer.tsx` — Multi-file WASM + reuse expansion

**Files:**
- Modify: `poiesis/src/components/design/OmniViewRenderer.tsx`

**Interfaces:**
- Consumes: `expandCustomComponents` (from codegen), `generateOmniFiles`, `generateOmniFragment`, `resolveStylesMulti`, `rustCompiler`
- Produces: Canvas rendering with translated WASM class map

- [ ] **Step 1: Update imports**

Replace the import block (line 1-20):

```typescript
import { useMemo, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { useElements, useRootOrder, useSelectedElementIds, useComponentDefs } from "../../store/hooks";
import { OmniViewElement } from "./OmniViewElement";
import { buildInlineStyleMap } from "./render-utils";
import { rustCompiler } from "../../commands/rustCompiler";
import { generateOmniFiles, generateOmniFragment } from "../../services/codegen/pipeline";
import { expandCustomComponents } from "../../services/codegen/expand-custom-components";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { ComponentDef } from "../../core/types/project-json";
import type { CodeTab } from "../../core/types/codegraph";
```

- [ ] **Step 2: Replace inline expansion with `expandCustomComponents`**

Delete the inline expansion loop (lines 38-96) and replace with:

```typescript
// Expand custom components for canvas rendering (reuses codegen's expansion)
const { expandedElements, customChildIds, idMappings } = useMemo(() => {
  const result = expandCustomComponents(elements);
  // Convert to Map for O(1) lookups in OmniViewElement
  const map = new Map<string, ElementData>(result.elements.map((e) => [e.id, e]));
  return { expandedElements: map, customChildIds: result.customChildIds, idMappings: result.idMappings };
}, [elements, componentDefs]);
```

- [ ] **Step 3: Generate ALL component sources for WASM**

Replace the `componentSource` useMemo (lines 101-120):

```typescript
// Generate ALL .omni sources for WASM multi-file resolution
const allOmniSources = useMemo(() => {
  if (elements.length === 0) return null;

  const codeTabs: CodeTab[] = [{
    id: "main", type: "logic", label: "Main", icon: "📄",
    graph: { elements, wires }, variables: [],
  }];

  try {
    // Main component
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

    // Collect main + store files
    if (mainResult.value) {
      Object.assign(sources, mainResult.value);
    }

    // Generate source for each custom component referenced in the tree
    if (componentDefs) {
      for (const [compName, compDef] of Object.entries(componentDefs)) {
        if (!compDef.elements || compDef.elements.length === 0) continue;
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

    return sources;
  } catch {
    return null;
  }
}, [elements, wires, rootOrder, activeDesign, componentDefs]);
```

- [ ] **Step 4: Add design source to WASM input**

Keep the existing `designSource` useMemo (lines 123-127). Then replace the `wasmStyleResult` useMemo (lines 130-142):

```typescript
// WASM CSS resolution with ALL component files
const wasmStyleResult = useMemo(() => {
  if (!allOmniSources || !rustCompiler.isReady) return null;

  const sources: Record<string, string> = { ...allOmniSources };

  // Add design.omni
  const designSrc = activeDesign?.name
    && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
    ? emitDesignSource(activeDesign)
    : null;
  if (designSrc) {
    sources["src/design.omni"] = designSrc;
  }

  // Build path mappings for cross-file imports
  const pathMapping: Record<string, string> = {
    "./design.omni": "src/design.omni",
  };
  if (componentDefs) {
    for (const compName of Object.keys(componentDefs)) {
      pathMapping[`./components/${compName}`] = `src/components/${compName}.omni`;
    }
  }

  const result = rustCompiler.resolveStylesMulti(sources, pathMapping);
  if (!result.success) return null;
  return result;
}, [allOmniSources, activeDesign, componentDefs]);
```

- [ ] **Step 5: Translate WASM class IDs from original → expanded**

Replace the `classMap` useMemo (lines 145-152):

```typescript
// Translate WASM class map from original element IDs → expanded canvas IDs
const classMap = useMemo(() => {
  if (!wasmStyleResult) return null;

  // Build reverse mapping: expandedId → originalDefId
  // (idMappings is: expandedId → originalDefId, same as expandCustomComponents returns)
  const map = new Map<string, string[]>();

  for (const [elId, classes] of Object.entries(wasmStyleResult.element_classes)) {
    // Direct lookup for non-component elements (IDs are unchanged)
    map.set(elId, classes as string[]);
  }

  // For expanded component-internal elements, add entries keyed by expanded ID
  for (const [expandedId, originalId] of idMappings) {
    const classes = wasmStyleResult.element_classes[originalId];
    if (classes) {
      map.set(expandedId, classes as string[]);
    }
  }

  return map;
}, [wasmStyleResult, idMappings]);
```

- [ ] **Step 6: Restore comment for canvas rendering**

The expanded tree + classMap are used identically in JSX. No changes needed after `classMap` — the existing render JSX (lines 191-223) works unchanged.

- [ ] **Step 7: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

Expected: may have errors from `useLiveCodePreview.ts` and `useSaveProject.ts` (still referencing `forSave`). If OmniViewRenderer-specific errors exist, fix them.

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/components/design/OmniViewRenderer.tsx
git commit -m "refactor: multi-file WASM input, reuse expandCustomComponents, translate class IDs"
```

---

### Task 7: Adapters — `useLiveCodePreview.ts` + `useSaveProject.ts`

**Files:**
- Modify: `poiesis/src/hooks/useLiveCodePreview.ts`
- Modify: `poiesis/src/hooks/useSaveProject.ts`

**Interfaces:**
- Consumes: `generateOmniFiles` (without `forSave` param)
- Produces: (unchanged)

- [ ] **Step 1: Remove `forSave` from `useLiveCodePreview.ts`**

In line 51, remove `forSave: false,`:

```typescript
const genResult = generateOmniFiles({
  designElements: elements,
  designWires: wires,
  codeTabs,
  componentName: activeTab.componentName ?? "main",
  rootOrder,
  // forSave removed — pipeline always emits clean code
  filePath: activeTab.filePath,
  designName: activeDesign?.name
    && (activeDesign.tokens?.length || activeDesign.classes?.length || activeDesign.variants?.length)
    ? activeDesign.name : undefined,
});
```

- [ ] **Step 2: Remove `forSave` from `useSaveProject.ts`**

In line 72, remove `forSave: true,`:

```typescript
const genResult = generateOmniFiles({
  designElements: store.elements,
  designWires: store.wires,
  codeTabs,
  componentName,
  rootOrder: store.rootOrder,
  // forSave removed — pipeline always emits clean code
  filePath: activeTab?.filePath,
  designName: store.activeDesign?.name
    && (store.activeDesign.tokens?.length || store.activeDesign.classes?.length || store.activeDesign.variants?.length)
    ? store.activeDesign.name : undefined,
});
```

- [ ] **Step 3: Verify global build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

Expected: zero new errors. All `forSave` references eliminated.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/hooks/useLiveCodePreview.ts poiesis/src/hooks/useSaveProject.ts
git commit -m "refactor: remove forSave references from hooks"
```

---

### Task 8: Integration verification via Victauri

**Files:**
- None (verification only)

- [ ] **Step 1: Build Poiesis**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | tail -5
```

Expected: clean build (pre-existing errors only).

- [ ] **Step 2: Start Poiesis**

```bash
bash .claude/skills/start-poiesis/start-poiesis.sh
```

- [ ] **Step 3: Verify Code panel output**

In Victauri, open the Code panel and verify:

1. **Component self-closing**: `<MyCard id="..." />` — no children inside
2. **Component PascalCase**: `component Main {` (not `component main`)
3. **No PID annotations**: No `// @pid:...` comments in view template
4. **No redundant wrapper**: Single root element not wrapped in extra `<div>`
5. **Conditional design import**: No `import { AppDesign }` if no design classes used
6. **Clean IDs**: Element IDs are semantic, not `inst-{timestamp}`

- [ ] **Step 4: Verify Canvas renders correctly**

Check that canvas visual output matches the Code panel structure — styles are applied correctly to component-internal elements.

- [ ] **Step 5: Verify Save produces correct files**

Press Ctrl+S in Poiesis. Check saved `main.omni` on disk:

```bash
cat /d/space/omni/demo2/src/main.omni
```

Expected: clean component reference, no inline expansion.

- [ ] **Step 6: Commit verification notes**

```bash
git add docs/superpowers/plans/2026-07-09-poiesis-omni-codegen-fix.md
git commit -m "docs: add poiesis omni codegen fix implementation plan"
```

---
