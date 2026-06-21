# OmniGen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bidirectional `.omni` source code pipeline for the OmniLang editor — parse `.omni` → editor canvas state and editor canvas state → `.omni` source, with `compileToIR` as validation gate.

**Architecture:** Parse direction uses a line-level state machine driven by Editor annotation comments (`@code-tab`, `@edit`, `@chain`). Generate direction uses a three-phase pipeline (Phase A: structural analysis → Phase B: semantic lowering → Phase C: code emission) with `StageResult<T>` error propagation. Both directions pass through `rustCompiler.compileToIR()` as a pure validation gate.

**Tech Stack:** TypeScript, React (Zustand stores), Rust WASM compiler (via existing `rustCompiler`), Vitest.

## Global Constraints

- All modules are pure functions with value passing, zero side effects
- `.omni` source is the single source of truth (not `.omni.json`)
- Editor metadata embedded in strict comment format: `@code-tab`, `@edit`, `@chain`
- `compileToIR` is a pure validation gate — pass/fail only, not a data source
- ID strategy: `{compName}.{path}` for round-trip stability
- Event tabs inline into main.omni; logic tabs become separate files
- TDD: write test first, verify it fails, implement, verify pass, commit

---

### Task 1: Core types

**Files:**
- Create: `editor/client/src/omni-gen/types.ts`

**Interfaces:**
- Produces: `PipelineError`, `StageResult<T>`, `ValidationResult`, `Diagnostic`, `EmitInput`, `StoreDef`, `ComponentPart`, `EditAnnotation`, `AnnotationData`, `AnnotationStrategy`, `ViewNode`, `ResolvedStyle`, `ChainBlock`, `LoweredCode`, `BindingResult`, `ThemeDef`

- [ ] **Step 1: Write the file**

```typescript
// editor/client/src/omni-gen/types.ts
// Shared types for the OmniGen pipeline.

import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";

// ── Error types ──

export interface Diagnostic {
  severity: string;
  code: string;
  message: string;
  line: number;
  col: number;
  end_line: number;
  end_col: number;
}

export interface PipelineError {
  phase: "A" | "B" | "C";
  module: string;
  severity: "fatal" | "warning";
  elementId?: string;
  code: string;
  message: string;
}

export interface StageResult<T> {
  value: T | null;
  errors: PipelineError[];
}

export interface ValidationResult {
  valid: boolean;
  errors: Diagnostic[];
}

// ── EmitInput (SourceEmitter input) ──

export interface StoreDef {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{ name: string; type: string }>; body: string }>;
}

export interface ComponentPart {
  name: string;
  signals: Record<string, { type: string; default?: unknown }>;
  computeds: Record<string, { type: string; body: string }>;
  storeRefs: string[];
  fns: Array<{ name: string; params: Array<{ name: string; type: string }>; returnType: string; body: string }>;
  view: ViewNode | null;
  style: Record<string, Record<string, string>>;
}

export interface ThemeDef {
  name: string;
  categories: Record<string, Record<string, string>>;
}

export interface EmitInput {
  codeTabType: "logic" | "event";
  imports: string[];
  stores: StoreDef[];
  component: ComponentPart;
  themes?: ThemeDef[];
  customComponents?: Array<{ name: string; source: string }>;
}

// ── Annotation ──

export interface EditAnnotation {
  line: string;    // "// @edit" | "// @edit folded" | "// @edit render=canvas"
  indent: number;  // view tree depth
}

export interface AnnotationData {
  codeTabDirective: string;
  editAnnotations: Map<string, EditAnnotation>;
}

export interface AnnotationStrategy {
  beforeViewNode(node: ViewNode, depth: number): string | null;
  beforeFile(): string;
}

// ── ViewNode ──

export type ViewNodeKind = "element" | "text" | "interpolation" | "if" | "for" | "slot";

export interface ResolvedStyle {
  htmlTag: string;
  cssClassName: string;
  contentText?: string;
  htmlAttrs: Record<string, string>;
  eventBindings: Record<string, string>;
  bindings: Record<string, string>;
  cssRules: Record<string, string>;
  usedClassNames: string[];
}

export interface ViewNode {
  id: string;
  path: string;         // "{compName}.{path}" for round-trip stability
  tag: string;
  props: Record<string, unknown>;
  children: ViewNode[];
  kind: ViewNodeKind;
  resolvedStyle?: ResolvedStyle;
  isFolded?: boolean;
  renderMode?: "dom" | "canvas";
  condition?: string;
  branches?: { condition: string; body: ViewNode[] }[];
  fallback?: ViewNode[] | null;
  iterable?: string;
  itemBinding?: string;
  keyExpr?: string;
  emptyBody?: ViewNode[] | null;
  text?: string;
  expression?: string;
  slotName?: string;
}

// ── Intermediate types ──

export interface BindingResult {
  signalBindings: Map<string, Array<{ elementId: string; propName: string }>>;
  storeRefs: Map<string, string[]>;
  computedDeps: Map<string, string[]>;
  unwiredProps: Array<{ elementId: string; tag: string; propName: string }>;
}

export interface LoweredCode {
  expressions: Map<string, string>;
  functions: Array<{ name: string; params: Array<{ name: string; type: string }>; returnType: string; body: string }>;
  actions: Array<{ storeName: string; name: string; params: Array<{ name: string; type: string }>; body: string }>;
  getters: Array<{ name: string; returnType: string; body: string; deps: string[] }>;
}

export interface ChainBlock {
  functionName: string;
  nodeLines: string[];
  wireLines: string[];
}

// ── Parse output ──

export interface ParseResult {
  designElements: ElementData[] | null;
  designWires: WireData[];
  codeTabs: CodeTab[];
  imports: string[];
  errors: Diagnostic[];
}
```

- [ ] **Step 2: Commit**

```bash
git add editor/client/src/omni-gen/types.ts
git commit -m "feat(omni-gen): add core types for pipeline"
```

---

### Task 2: Validator

**Files:**
- Create: `editor/client/src/omni-gen/validator.ts`
- Test: `editor/client/src/omni-gen/__tests__/validator.test.ts`

**Interfaces:**
- Consumes: `rustCompiler.compileToIR` (existing, from `../api/rustCompiler`)
- Produces: `validate(source: string): ValidationResult`

- [ ] **Step 1: Write the test**

```typescript
// editor/client/src/omni-gen/__tests__/validator.test.ts
import { describe, it, expect, vi } from "vitest";
import { validate } from "../validator";
import type { ValidationResult } from "../types";

// Mock rustCompiler — we test the wrapper logic, not the WASM
vi.mock("../../api/rustCompiler", () => ({
  rustCompiler: {
    compileToIR: vi.fn(),
  },
}));

import { rustCompiler } from "../../api/rustCompiler";

describe("validate", () => {
  it("returns valid=true when compileToIR succeeds", () => {
    (rustCompiler.compileToIR as any).mockReturnValue({
      success: true,
      ir: { version: "1", components: {}, stores: {}, source_map: {} },
      errors: [],
    });

    const result = validate("component App { view { <div></div> } }");

    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });

  it("returns valid=false with errors when compileToIR fails", () => {
    (rustCompiler.compileToIR as any).mockReturnValue({
      success: false,
      errors: [{ severity: "error", code: "E001", message: "unexpected token", line: 1, col: 10, end_line: 1, end_col: 15 }],
    });

    const result = validate("invalid @@@ omni code");

    expect(result.valid).toBe(false);
    expect(result.errors).toHaveLength(1);
    expect(result.errors[0].code).toBe("E001");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd editor/client && npx vitest run src/omni-gen/__tests__/validator.test.ts
```
Expected: FAIL — module not found

- [ ] **Step 3: Write implementation**

```typescript
// editor/client/src/omni-gen/validator.ts
import { rustCompiler } from "../api/rustCompiler";
import type { ValidationResult } from "./types";

export function validate(source: string): ValidationResult {
  const result = rustCompiler.compileToIR(source);
  return {
    valid: result.success,
    errors: (result.errors || []) as ValidationResult["errors"],
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd editor/client && npx vitest run src/omni-gen/__tests__/validator.test.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-gen/validator.ts editor/client/src/omni-gen/__tests__/validator.test.ts
git commit -m "feat(omni-gen): add validator module wrapping compileToIR"
```

---

### Task 3: OmniParser — core state machine

**Files:**
- Create: `editor/client/src/omni-gen/omni-parser.ts`
- Test: `editor/client/src/omni-gen/__tests__/omni-parser.test.ts`

**Interfaces:**
- Consumes: `ElementData`, `WireData` from `../store/editor`; `CodeTab`, `SignalDefinition` from `../codegraph/types`
- Produces: `parseOmniSource(source: string): ParseResult`

- [ ] **Step 1: Write the test**

```typescript
// editor/client/src/omni-gen/__tests__/omni-parser.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

const SIMPLE_OMNI = `// @code-tab type=logic
import omniview;

component Counter {
    signal count: i32 = 0;

    fn inc() -> void {
        count = count + 1;
    }

    view {
        // @edit
        <div id="Counter.root" class: box>
            // @edit
            <button id="Counter.root.button.0" class: btn @onPress={inc}>+</button>
        </div>
    }
    style {
        .box { background: white; padding: 20px; }
        .btn { width: 40px; border-radius: 8px; }
    }
}
`;

describe("parseOmniSource", () => {
  it("parses @code-tab directive", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    expect(result.errors).toEqual([]);
    expect(result.codeTabs).toHaveLength(1);
    expect(result.codeTabs[0].type).toBe("logic");
  });

  it("parses component signal", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const signals = result.designElements!.filter(e => e.tag === "signal");
    expect(signals).toHaveLength(1);
    expect(signals[0].props.name).toBe("count");
    expect(signals[0].props.typeAnnotation).toBe("i32");
  });

  it("parses view elements with @edit annotations", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const views = result.designElements!.filter(e => e.nodeType === "view");
    expect(views).toHaveLength(2);
    expect(views[0].id).toBe("Counter.root");
    expect(views[0].tag).toBe("div");
    expect(views[1].id).toBe("Counter.root.button.0");
    expect(views[1].tag).toBe("button");
    expect(views[1].props.onPress).toBe("inc");
  });

  it("builds parent/child relationships", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const root = result.designElements!.find(e => e.id === "Counter.root")!;
    const btn = result.designElements!.find(e => e.id === "Counter.root.button.0")!;
    expect(root.children).toContain(btn.id);
    expect(btn.parentId).toBe(root.id);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd editor/client && npx vitest run src/omni-gen/__tests__/omni-parser.test.ts
```
Expected: FAIL

- [ ] **Step 3: Write the parser state machine**

```typescript
// editor/client/src/omni-gen/omni-parser.ts
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type { ParseResult } from "./types";

type ParserState =
  | "ROOT"
  | "IN_COMPONENT"
  | "IN_STORE"
  | "IN_VIEW"
  | "IN_CHAIN"
  | "IN_FN"
  | "IN_STYLE";

interface ParseContext {
  state: ParserState;
  stateStack: ParserState[];
  elements: ElementData[];
  wires: WireData[];
  codeTabs: CodeTab[];
  imports: string[];
  errors: ParseResult["errors"];
  codeTabType: "logic" | "event";
  currentComponent: string;
  currentStore: string;
  chainName: string;
  chainElements: ElementData[];
  chainWires: WireData[];
  viewParentStack: string[];
  braceDepth: number;
  pendingEditFlags: string | null;
}

export function parseOmniSource(source: string): ParseResult {
  const ctx: ParseContext = {
    state: "ROOT",
    stateStack: [],
    elements: [],
    wires: [],
    codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "📄", graph: { elements: [], wires: [] }, variables: [] }],
    imports: [],
    errors: [],
    codeTabType: "logic",
    currentComponent: "",
    currentStore: "",
    chainName: "",
    chainElements: [],
    chainWires: [],
    viewParentStack: [],
    braceDepth: 0,
    pendingEditFlags: null,
  };

  const lines = source.split("\n");

  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    const trimmed = line.trim();

    if (trimmed === "") continue;

    switch (ctx.state) {
      case "ROOT":
        parseRootLine(trimmed, ctx);
        break;
      case "IN_COMPONENT":
        parseComponentLine(trimmed, line, ctx);
        break;
      case "IN_VIEW":
        parseViewLine(trimmed, line, ctx);
        break;
      case "IN_FN":
        parseFnBodyLine(trimmed, ctx);
        break;
      case "IN_STYLE":
        parseStyleBlockLine(trimmed, ctx);
        break;
      case "IN_CHAIN":
        parseChainLine(trimmed, ctx);
        break;
    }
  }

  // Flush any remaining chain elements
  flushChain(ctx);

  return {
    designElements: ctx.elements.length > 0 ? ctx.elements : null,
    designWires: ctx.wires,
    codeTabs: ctx.codeTabs,
    imports: ctx.imports,
    errors: ctx.errors,
  };
}

// ── ROOT state handlers ──

function parseRootLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed.startsWith("// @code-tab type=")) {
    const m = trimmed.match(/type=(logic|event)/);
    if (m) ctx.codeTabType = m[1] as "logic" | "event";
    return;
  }
  if (trimmed.startsWith("import ")) {
    ctx.imports.push(trimmed);
    return;
  }
  if (trimmed.startsWith("component ")) {
    const m = trimmed.match(/^component\s+(\w+)/);
    if (m) {
      ctx.currentComponent = m[1];
      ctx.braceDepth = 1;
      pushState(ctx, "IN_COMPONENT");
    }
    return;
  }
  if (trimmed.startsWith("store ")) {
    const m = trimmed.match(/^store\s+(\w+)/);
    if (m) {
      ctx.currentStore = m[1];
      ctx.braceDepth = 1;
      pushState(ctx, "IN_STORE");
    }
    return;
  }
}

// ── IN_COMPONENT state handlers ──

function parseComponentLine(trimmed: string, line: string, ctx: ParseContext): void {
  // Track brace depth
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) {
      popState(ctx);
      return;
    }
  }
  if (trimmed.endsWith("{")) ctx.braceDepth++;

  // @store
  if (trimmed.startsWith("@store ")) {
    const m = trimmed.match(/@store\s+(\w+)/);
    if (m) ctx.elements.push(codeblockEl(`${ctx.currentComponent}.store.${m[1]}`, "store", { name: m[1] }));
    return;
  }

  // signal
  if (trimmed.startsWith("signal ")) {
    const m = trimmed.match(/signal\s+(\w+):\s*(\w+)\s*=\s*(.+);/);
    if (m) {
      ctx.elements.push(codeblockEl(`${ctx.currentComponent}.signal.${m[1]}`, "signal", {
        name: m[1], typeAnnotation: m[2],
      }));
    }
    return;
  }

  // computed
  if (trimmed.startsWith("computed ")) {
    const m = trimmed.match(/computed\s+(\w+):\s*(\w+)\s*=\s*(.+);/);
    if (m) {
      ctx.elements.push(codeblockEl(`${ctx.currentComponent}.computed.${m[1]}`, "computed", {
        name: m[1], typeAnnotation: m[2],
      }));
    }
    return;
  }

  // fn
  if (trimmed.startsWith("fn ")) {
    pushState(ctx, "IN_FN");
    return;
  }

  // view
  if (trimmed === "view {") {
    pushState(ctx, "IN_VIEW");
    ctx.viewParentStack = [];
    return;
  }

  // style
  if (trimmed === "style {") {
    pushState(ctx, "IN_STYLE");
    return;
  }

  // @chain
  if (trimmed.startsWith("// @chain ")) {
    const m = trimmed.match(/\/\/\s*@chain\s+(\w+)/);
    if (m) {
      ctx.chainName = m[1];
      ctx.chainElements = [];
      ctx.chainWires = [];
      pushState(ctx, "IN_CHAIN");
    }
    return;
  }
}

// ── IN_VIEW state handlers ──

function parseViewLine(trimmed: string, line: string, ctx: ParseContext): void {
  if (trimmed === "}") {
    popState(ctx);
    return;
  }

  // @edit annotation
  if (trimmed.startsWith("// @edit")) {
    ctx.pendingEditFlags = trimmed.slice("// @edit".length).trim() || null;
    return;
  }

  // Opening tag
  if (trimmed.startsWith("<") && !trimmed.startsWith("</")) {
    const indent = line.length - line.trimStart().length;
    const el = parseElementLine(trimmed, ctx);

    // Parent inference by indent
    while (ctx.viewParentStack.length > 0) {
      const parentId = ctx.viewParentStack[ctx.viewParentStack.length - 1];
      const parent = ctx.elements.find(e => e.id === parentId);
      if (parent && (parent as any)._viewIndent !== undefined && (parent as any)._viewIndent >= indent) {
        ctx.viewParentStack.pop();
      } else {
        break;
      }
    }

    const parentId = ctx.viewParentStack.length > 0 ? ctx.viewParentStack[ctx.viewParentStack.length - 1] : null;
    el.parentId = parentId;
    (el as any)._viewIndent = indent;

    if (parentId) {
      const parent = ctx.elements.find(e => e.id === parentId);
      if (parent) parent.children.push(el.id);
    }

    // @edit flags
    if (ctx.pendingEditFlags) {
      if (ctx.pendingEditFlags.includes("folded")) el.isFolded = true;
      if (ctx.pendingEditFlags.includes("render=canvas")) el.renderMode = "canvas";
      ctx.pendingEditFlags = null;
    }

    ctx.elements.push(el);

    // Not self-closing → push to parent stack
    if (!trimmed.endsWith("/>")) {
      ctx.viewParentStack.push(el.id);
    }
    return;
  }

  // Closing tag
  if (trimmed.startsWith("</")) {
    if (ctx.viewParentStack.length > 0) ctx.viewParentStack.pop();
    return;
  }
}

function parseElementLine(trimmed: string, ctx: ParseContext): ElementData {
  const tagMatch = trimmed.match(/^<(\w+)/);
  const tag = tagMatch ? tagMatch[1] : "div";

  const idMatch = trimmed.match(/id="([^"]*)"/);
  const id = idMatch ? idMatch[1] : `${ctx.currentComponent}.${tag}.${ctx.elements.length}`;

  const classMatch = trimmed.match(/class:\s*(\w+)/);
  const className = classMatch ? classMatch[1] : "";

  const events: Record<string, string> = {};
  const eventRegex = /@(\w+)={(\w+)}/g;
  let evtMatch;
  while ((evtMatch = eventRegex.exec(trimmed)) !== null) {
    events[evtMatch[1]] = evtMatch[2];
  }

  const attrs: Record<string, string> = {};
  const attrRegex = /\b(\w+)="([^"]*)"/g;
  let attrMatch;
  while ((attrMatch = attrRegex.exec(trimmed)) !== null) {
    if (attrMatch[1] !== "id" && attrMatch[1] !== "class") {
      attrs[attrMatch[1]] = attrMatch[2];
    }
  }

  const textMatch = trimmed.match(/>([^<]*)<\//);
  const textContent = textMatch ? textMatch[1].trim() : "";

  return {
    id,
    nodeType: "view",
    tag,
    props: { ...attrs, ...events, className, ...(textContent ? { label: textContent } : {}) },
    children: [],
    parentId: null,
  };
}

// ── IN_FN — skip function body ──

function parseFnBodyLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed === "{" || trimmed.endsWith("{")) ctx.braceDepth++;
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) popState(ctx);
  }
}

// ── IN_STYLE — skip style block ──

function parseStyleBlockLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed === "{") ctx.braceDepth++;
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) popState(ctx);
  }
}

// ── IN_CHAIN — parse @node / @wire ──

function parseChainLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed.startsWith("// ═")) {
    flushChain(ctx);
    popState(ctx);
    return;
  }

  if (trimmed.startsWith("//   @node ")) {
    const parts = trimmed.slice(3).trim().split(/\s+/); // skip "//"
    const id = parts[1];
    const tag = parts[2];
    const attrs: Record<string, string> = {};
    let posX = 0, posY = 0;
    for (let i = 3; i < parts.length; i++) {
      if (parts[i].startsWith("pos=")) {
        const coords = parts[i].slice(4).split(",");
        posX = parseInt(coords[0]) || 0;
        posY = parseInt(coords[1]) || 0;
      } else if (parts[i].includes("=")) {
        const [k, v] = parts[i].split("=");
        attrs[k] = v;
      }
    }
    ctx.chainElements.push({
      id, nodeType: "codeblock", tag,
      props: attrs,
      position: { x: posX, y: posY },
      children: [], parentId: null,
    });
    return;
  }

  if (trimmed.startsWith("//   @wire ")) {
    const wireMatch = trimmed.match(/@wire\s+(\S+)\s+(\S+)\s*->\s*(\S+)/);
    if (wireMatch) {
      const [, wireId, from, to] = wireMatch;
      const [fromId, fromPort = ""] = from.includes(".") ? from.split(".") : [from];
      const [toId, toPort = ""] = to.includes(".") ? to.split(".") : [to];
      ctx.chainWires.push({ id: wireId, fromId, fromPort, toId, toPort });
    }
  }
}

function flushChain(ctx: ParseContext): void {
  if (ctx.chainElements.length === 0) return;
  const tab = ctx.codeTabs.find(t => t.id === "main") || ctx.codeTabs[0];
  for (const el of ctx.chainElements) tab.graph.elements.push(el);
  for (const w of ctx.chainWires) tab.graph.wires.push(w);
  ctx.chainElements = [];
  ctx.chainWires = [];
}

// ── Helpers ──

function pushState(ctx: ParseContext, state: ParserState): void {
  ctx.stateStack.push(ctx.state);
  ctx.state = state;
  ctx.braceDepth = 1;
}

function popState(ctx: ParseContext): void {
  ctx.state = ctx.stateStack.pop() || "ROOT";
}

function codeblockEl(id: string, tag: string, props: Record<string, unknown>): ElementData {
  return {
    id, nodeType: "codeblock", tag, props: props as Record<string, unknown>,
    children: [], parentId: null,
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd editor/client && npx vitest run src/omni-gen/__tests__/omni-parser.test.ts
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-gen/omni-parser.ts editor/client/src/omni-gen/__tests__/omni-parser.test.ts
git commit -m "feat(omni-gen): add omni-parser with line-level state machine"
```

---

### Task 4: parseOmniSource entry point

**Files:**
- Create: `editor/client/src/omni-gen/index.ts`

**Interfaces:**
- Produces: `parseOmniSource(source: string): ParseResult`

- [ ] **Step 1: Write the file**

```typescript
// editor/client/src/omni-gen/index.ts
export { parseOmniSource } from "./omni-parser";
export { validate } from "./validator";
export type * from "./types";
```

- [ ] **Step 2: Commit**

```bash
git add editor/client/src/omni-gen/index.ts
git commit -m "feat(omni-gen): add public entry point"
```

---

### Task 5: ViewTreeBuilder

**Files:**
- Create: `editor/client/src/omni-gen/view-tree-builder.ts`
- Test: `editor/client/src/omni-gen/__tests__/view-tree-builder.test.ts`

**Interfaces:**
- Consumes: `ElementData` from `../store/editor`
- Produces: `buildViewTree(elements: ElementData[], compName: string): { roots: ViewNode[]; pascalCaseTags: string[] }`

- [ ] **Step 1: Write the test**

```typescript
// editor/client/src/omni-gen/__tests__/view-tree-builder.test.ts
import { describe, it, expect } from "vitest";
import { buildViewTree } from "../view-tree-builder";
import type { ElementData } from "../../store/editor";

describe("buildViewTree", () => {
  it("builds nested view tree from flat elements", () => {
    const elements: ElementData[] = [
      { id: "root", nodeType: "view", tag: "Container", props: {}, children: ["child1"], parentId: null },
      { id: "child1", nodeType: "view", tag: "Button", props: { label: "Click" }, children: [], parentId: "root" },
    ];

    const { roots } = buildViewTree(elements, "App");

    expect(roots).toHaveLength(1);
    expect(roots[0].path).toBe("App");
    expect(roots[0].children).toHaveLength(1);
    expect(roots[0].children[0].tag).toBe("Button");
    expect(roots[0].children[0].path).toBe("App.div.0");
  });

  it("collects PascalCase tags for imports", () => {
    const elements: ElementData[] = [
      { id: "r", nodeType: "view", tag: "Container", props: {}, children: ["h"], parentId: null },
      { id: "h", nodeType: "view", tag: "MyHeader", props: {}, children: [], parentId: "r" },
    ];

    const { pascalCaseTags } = buildViewTree(elements, "App");

    expect(pascalCaseTags).toContain("MyHeader");
    expect(pascalCaseTags).not.toContain("Container"); // in TAG_HTML_MAP
  });
});
```

- [ ] **Step 2: Run test — FAIL**

- [ ] **Step 3: Write implementation**

```typescript
// editor/client/src/omni-gen/view-tree-builder.ts
import type { ElementData } from "../store/editor";
import type { ViewNode } from "./types";

const TAG_HTML_MAP = new Set([
  "Container", "Flex", "Grid", "Card", "Text", "Heading",
  "Button", "Input", "TextArea", "Select", "Checkbox",
  "List", "ListItem", "Image", "Spinner", "Progress",
  "Alert", "Link", "Form",
]);

export function buildViewTree(
  elements: ElementData[],
  compName: string,
): { roots: ViewNode[]; pascalCaseTags: string[] } {
  const byId = new Map<string, ElementData>();
  const byParent = new Map<string | null, ElementData[]>();
  const pascalCaseTags: string[] = [];

  for (const el of elements) {
    if (el.nodeType !== "view") continue;
    byId.set(el.id, el);
    const key = el.parentId;
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key)!.push(el);
  }

  function convert(el: ElementData, parentPath: string, siblingIndex: number): ViewNode {
    const path = `${parentPath}.${el.tag.toLowerCase()}.${siblingIndex}`;
    const isPascal = /^[A-Z]/.test(el.tag) && !TAG_HTML_MAP.has(el.tag);
    if (isPascal && !pascalCaseTags.includes(el.tag)) {
      pascalCaseTags.push(el.tag);
    }

    const childElements = byParent.get(el.id) || [];
    const children = childElements.map((c, i) => convert(c, path, i));

    return {
      id: el.id,
      path,
      tag: el.tag,
      props: { ...el.props },
      children,
      kind: "element",
      isFolded: el.isFolded,
      renderMode: el.renderMode,
    };
  }

  const roots = (byParent.get(null) || []).map((el, i) => convert(el, compName, i));

  return { roots, pascalCaseTags };
}
```

- [ ] **Step 4: Run test — PASS**

- [ ] **Step 5: Commit**

---

### Task 6: WireAnalyzer

**Files:**
- Create: `editor/client/src/omni-gen/wire-analyzer.ts`

- [ ] **Step 1: Write test + implementation** (compact)

```typescript
// wire-analyzer.ts
import type { ElementData, WireData } from "../store/editor";
import type { BindingResult } from "./types";

export function analyzeWires(elements: ElementData[], wires: WireData[]): BindingResult {
  const signalBindings = new Map<string, Array<{ elementId: string; propName: string }>>();
  const storeRefs = new Map<string, string[]>();
  const computedDeps = new Map<string, string[]>();

  const elMap = new Map(elements.map(e => [e.id, e]));

  for (const w of wires) {
    const src = elMap.get(w.fromId);
    const tgt = elMap.get(w.toId);
    if (!src || !tgt) continue;

    // signal.value → view.prop
    if ((src.tag === "signal" || src.tag === "computed") && w.fromPort === "value" && tgt.nodeType === "view") {
      const name = src.props.name as string;
      if (!signalBindings.has(name)) signalBindings.set(name, []);
      signalBindings.get(name)!.push({ elementId: tgt.id, propName: w.toPort });
    }
    // store.items → view
    if (src.tag === "store" && w.fromPort === "items" && tgt.nodeType === "view") {
      const name = src.props.name as string;
      if (!storeRefs.has(name)) storeRefs.set(name, []);
      storeRefs.get(name)!.push(tgt.id);
    }
    // → computed.deps
    if (tgt.tag === "computed" && w.toPort === "deps") {
      const srcName = src.props.name as string;
      const tgtName = tgt.props.name as string;
      if (!computedDeps.has(tgtName)) computedDeps.set(tgtName, []);
      computedDeps.get(tgtName)!.push(srcName);
    }
  }

  // unwired props
  const unwiredProps: BindingResult["unwiredProps"] = [];
  const wiredProps = new Set<string>();
  for (const [, bindings] of signalBindings) {
    for (const b of bindings) wiredProps.add(`${b.elementId}:${b.propName}`);
  }
  for (const el of elements.filter(e => e.nodeType === "view")) {
    for (const key of Object.keys(el.props)) {
      if (!wiredProps.has(`${el.id}:${key}`) && !key.startsWith("on")) {
        unwiredProps.push({ elementId: el.id, tag: el.tag, propName: key });
      }
    }
  }

  return { signalBindings, storeRefs, computedDeps, unwiredProps };
}
```

- [ ] **Step 2: Test + commit**

---

### Task 7: ImportCollector

**Files:**
- Create: `editor/client/src/omni-gen/import-collector.ts`

- [ ] **Step 1: Write**

```typescript
// import-collector.ts
import type { CodeTab } from "../codegraph/types";

export function collectImports(
  pascalCaseTags: string[],
  storeRefs: string[],
  codeTabs: CodeTab[],
): string[] {
  const imports: string[] = ["import omniview;"];

  for (const tag of pascalCaseTags) {
    imports.push(`import { ${tag} } from "./components/${tag}";`);
  }

  for (const storeName of storeRefs) {
    imports.push(`import { ${storeName} } from "./stores/${storeName}";`);
  }

  // Cross-tab function references
  const logicFnMap = new Map<string, string>(); // fnName → tabLabel
  for (const tab of codeTabs) {
    if (tab.type === "logic" && tab.id !== "main") {
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName) {
          logicFnMap.set(el.config.fnName as string, tab.label);
        }
      }
    }
  }

  return imports;
}
```

- [ ] **Step 2: Test + commit**

---

### Task 8: StyleResolver

**Files:**
- Create: `editor/client/src/omni-gen/style-resolver.ts`

- [ ] **Step 1: Write TAG_HTML_MAP + resolve function**

```typescript
// style-resolver.ts
import type { ResolvedStyle, ViewNode } from "./types";

interface TagMapping {
  tag: string;
  propsToContent?: string;
  tagByProp?: string;
}

const TAG_HTML_MAP: Record<string, TagMapping> = {
  Container: { tag: "div" }, Flex: { tag: "div" }, Grid: { tag: "div" },
  Card: { tag: "div" }, Text: { tag: "p", propsToContent: "label" },
  Heading: { tag: "h1", propsToContent: "label", tagByProp: "level" },
  Button: { tag: "button", propsToContent: "label" },
  Input: { tag: "input" }, TextArea: { tag: "textarea" },
  Select: { tag: "select" }, Checkbox: { tag: "input" },
  List: { tag: "ul" }, ListItem: { tag: "li" },
  Image: { tag: "img" }, Spinner: { tag: "div" },
  Progress: { tag: "progress" }, Alert: { tag: "div" },
  Link: { tag: "a", propsToContent: "label" }, Form: { tag: "form" },
};

const STYLE_PROP_CSS_MAP: Record<string, string> = {
  background_color: "background", border_radius: "border-radius",
  font_size: "font-size", font_weight: "font-weight",
  text_align: "text-align", shadow: "box-shadow",
  padding: "padding", margin: "margin", gap: "gap",
  width: "width", height: "height", color: "color",
};

const STYLE_PROPS: Record<string, string[]> = {
  Container: ["padding","margin","gap","background_color","border_radius","width","height"],
  Button: ["width","padding","background_color","border_radius","font_size"],
  Flex: ["padding","margin","width","height"],
  Card: ["padding","margin","border_radius","shadow","background_color","width","height"],
  Text: ["font_size","font_weight","color","text_align"],
  Heading: ["font_size","font_weight","color"],
  Image: ["width","height","border_radius"],
  Input: ["width","padding","border_radius","font_size"],
  TextArea: ["width","padding"],
  Select: ["width","padding"],
  List: ["padding","margin","gap"],
  ListItem: ["padding","margin"],
  Alert: ["padding","border_radius"],
  Link: ["color","font_size"],
  Form: ["padding","gap","width"],
  Spinner: ["width","height"],
  Progress: ["width"],
};

export function resolveStyle(node: ViewNode): ResolvedStyle {
  const mapping = TAG_HTML_MAP[node.tag];
  const htmlTag = mapping ? mapping.tag : node.tag;

  const styleProps = STYLE_PROPS[node.tag] || [];
  const cssClassName = sanitizeClassName(node.path);

  const cssRules: Record<string, string> = {};
  const htmlAttrs: Record<string, string> = {};
  const eventBindings: Record<string, string> = {};
  let contentText: string | undefined;

  for (const [key, val] of Object.entries(node.props)) {
    if (val === undefined || val === null || val === "") continue;

    // Events
    if (key.startsWith("on") && typeof val === "string") {
      eventBindings[key] = val;
      continue;
    }

    // Style props
    if (styleProps.includes(key)) {
      const cssProp = STYLE_PROP_CSS_MAP[key] || key;
      cssRules[cssProp] = String(val);
      continue;
    }

    // Props to content
    if (mapping?.propsToContent === key) {
      contentText = String(val);
      continue;
    }

    // Regular attrs
    htmlAttrs[key] = String(val);
  }

  // tagByProp: level="2" → <h2>
  let finalTag = htmlTag;
  if (mapping?.tagByProp && node.props[mapping.tagByProp]) {
    const level = String(node.props[mapping.tagByProp]!);
    if (mapping.tagByProp === "level") finalTag = `h${level}`;
  }

  return {
    htmlTag: finalTag,
    cssClassName,
    contentText,
    htmlAttrs,
    eventBindings,
    bindings: {},
    cssRules,
    usedClassNames: [cssClassName],
  };
}

function sanitizeClassName(path: string): string {
  return path.replace(/[^a-zA-Z0-9_-]/g, "-").replace(/^-+/, "");
}
```

- [ ] **Step 2: Test + commit**

---

### Task 9: CodeWriter

**Files:**
- Create: `editor/client/src/omni-gen/source-emitter.ts` (CodeWriter class first)

- [ ] **Step 1: Write CodeWriter**

```typescript
// CodeWriter (part of source-emitter.ts)
class CodeWriter {
  private lines: string[] = [];
  private _indent = 0;
  private readonly indentStep = "    "; // 4 spaces

  emit(line: string): void {
    this.lines.push(" ".repeat(this._indent * 4) + line);
  }

  indent(): void { this._indent++; }
  dedent(): void { this._indent = Math.max(0, this._indent - 1); }
  blank(): void { this.lines.push(""); }
  toString(): string { return this.lines.join("\n") + "\n"; }
}
```

- [ ] **Step 2: Test + commit**

---

### Task 10: SourceEmitter — view + style generation

**Files:**
- Modify: `editor/client/src/omni-gen/source-emitter.ts`

- [ ] **Step 1: Write emitDocument (without annotations)**

```typescript
// source-emitter.ts (continued)
import type { EmitInput, AnnotationStrategy, ChainBlock } from "./types";
import type { ViewNode } from "./types";

export class SourceEmitter {
  constructor(private annotationStrategy?: AnnotationStrategy) {}

  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string {
    const writer = new CodeWriter();

    // @code-tab
    if (this.annotationStrategy) {
      writer.emit(this.annotationStrategy.beforeFile());
    }

    // imports
    if (input.imports.length > 0) {
      for (const imp of input.imports) writer.emit(imp);
      writer.blank();
    }

    // stores
    for (const store of input.stores) {
      this.emitStore(store, writer, chains);
      writer.blank();
    }

    // component
    this.emitComponent(input, writer, chains);

    // themes
    for (const theme of (input.themes || [])) {
      writer.emit(`theme ${theme.name} {`);
      for (const [cat, entries] of Object.entries(theme.categories)) {
        writer.emit(`  ${cat} {`);
        for (const [k, v] of Object.entries(entries)) writer.emit(`    ${k}: "${v}";`);
        writer.emit(`  }`);
      }
      writer.emit(`}`);
    }

    return writer.toString();
  }

  private emitStore(store: EmitInput["stores"][0], w: CodeWriter, chains: Map<string, ChainBlock>): void {
    w.emit(`store ${store.name} {`);
    w.indent();
    for (const [name, sig] of Object.entries(store.signals)) {
      const def = sig.default !== undefined ? ` = ${JSON.stringify(sig.default)}` : "";
      w.emit(`signal ${name}: ${sig.type}${def};`);
    }
    for (const [name, g] of Object.entries(store.getters)) {
      w.emit(`getter ${name}: ${g.returnType} = ${g.body};`);
    }
    for (const a of store.actions) {
      const params = a.params.map(p => `${p.name}: ${p.type}`).join(", ");
      w.emit(`action ${a.name}(${params}) {`);
      w.indent();
      for (const line of a.body.split("\n")) w.emit(line);
      w.dedent();
      w.emit(`}`);
    }
    w.dedent();
    w.emit(`}`);
  }

  private emitComponent(input: EmitInput, w: CodeWriter, chains: Map<string, ChainBlock>): void {
    const comp = input.component;
    w.emit(`component ${comp.name} {`);
    w.indent();

    for (const sr of comp.storeRefs) w.emit(`@store ${sr};`);
    for (const [name, sig] of Object.entries(comp.signals)) {
      const def = sig.default !== undefined ? ` = ${JSON.stringify(sig.default)}` : "";
      w.emit(`signal ${name}: ${sig.type}${def};`);
    }
    for (const [name, c] of Object.entries(comp.computeds)) {
      w.emit(`computed ${name}: ${c.type} = ${c.body};`);
    }

    // Functions with chains
    for (const fn of comp.fns) {
      const chain = chains.get(fn.name);
      if (chain) w.emit(formatChainBlock(chain));
      const params = fn.params.map(p => `${p.name}: ${p.type}`).join(", ");
      w.emit(`fn ${fn.name}(${params}) -> ${fn.returnType} {`);
      w.indent();
      for (const line of fn.body.split("\n")) w.emit(line);
      w.dedent();
      w.emit(`}`);
      w.blank();
    }

    // View
    if (comp.view) {
      w.emit(`view {`);
      w.indent();
      this.emitViewNode(comp.view, w, 0);
      w.dedent();
      w.emit(`}`);
    }

    // Style
    if (Object.keys(comp.style).length > 0) {
      w.emit(`style {`);
      w.indent();
      for (const [cls, rules] of Object.entries(comp.style)) {
        const ruleStr = Object.entries(rules).map(([k, v]) => `${k}: ${v};`).join(" ");
        w.emit(`.${cls} { ${ruleStr} }`);
      }
      w.dedent();
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }

  private emitViewNode(node: ViewNode, w: CodeWriter, depth: number): void {
    // Annotation
    if (this.annotationStrategy) {
      const ann = this.annotationStrategy.beforeViewNode(node, depth);
      if (ann) w.emit(ann);
    }

    const style = node.resolvedStyle;
    if (!style) return;

    const parts: string[] = [];
    parts.push(`<${style.htmlTag}`);
    parts.push(` id="${node.path}"`);
    parts.push(` class: ${style.cssClassName}`);

    for (const [attr, val] of Object.entries(style.htmlAttrs)) {
      parts.push(` ${attr}="${escapeAttr(val)}"`);
    }
    for (const [evt, handler] of Object.entries(style.eventBindings)) {
      parts.push(` @${evt}={${handler}}`);
    }
    for (const [prop, sig] of Object.entries(style.bindings)) {
      parts.push(` bind:${prop}={${sig}}`);
    }

    const openTag = parts.join("");
    const hasContent = style.contentText || node.children.length > 0;

    if (!hasContent) {
      w.emit(`${openTag}></${style.htmlTag}>`);
      return;
    }

    w.emit(`${openTag}>`);
    w.indent();

    if (style.contentText) w.emit(style.contentText);

    for (const child of node.children) this.emitViewNode(child, w, depth + 1);

    w.dedent();
    w.emit(`</${style.htmlTag}>`);
  }
}

function formatChainBlock(chain: ChainBlock): string {
  const lines: string[] = [];
  lines.push(`// ═══════════════════════════════════════════════════════`);
  lines.push(`// @chain ${chain.functionName}`);
  for (const nl of chain.nodeLines) lines.push(nl);
  for (const wl of chain.wireLines) lines.push(wl);
  lines.push(`// ═══════════════════════════════════════════════════════`);
  return lines.join("\n");
}

function escapeAttr(val: string): string {
  return val.replace(/"/g, '\\"');
}

export { CodeWriter };
```

- [ ] **Step 2: Test + commit**

---

### Task 11: Pipeline — Phase A + C skeleton

**Files:**
- Create: `editor/client/src/omni-gen/pipeline.ts`

- [ ] **Step 1: Write generateOmniSource skeleton**

```typescript
// pipeline.ts
import { buildViewTree } from "./view-tree-builder";
import { analyzeWires } from "./wire-analyzer";
import { collectImports } from "./import-collector";
import { resolveStyle } from "./style-resolver";
import { SourceEmitter } from "./source-emitter";
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type { StageResult, EmitInput } from "./types";

export function generateOmniSource(options: {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
}): StageResult<string> {
  const errors: StageResult<string>["errors"] = [];

  // Phase A
  const { roots } = buildViewTree(
    options.designElements.filter(e => e.nodeType === "view"),
    options.componentName,
  );
  const bindings = analyzeWires(options.designElements, options.designWires);
  const pascalCaseTags = collectPascalCaseTags(roots);
  const imports = collectImports(
    pascalCaseTags,
    [...bindings.storeRefs.keys()],
    options.codeTabs,
  );

  // Phase B (stub)
  const mainTab = options.codeTabs.find(t => t.id === "main") || options.codeTabs[0];

  // Phase C — apply styles, emit
  applyStyles(roots);

  const componentStyle: Record<string, Record<string, string>> = {};
  collectStyles(roots, componentStyle);

  const emitInput: EmitInput = {
    codeTabType: mainTab.type as "logic" | "event",
    imports,
    stores: [],
    component: {
      name: options.componentName,
      signals: {},
      computeds: {},
      storeRefs: [...bindings.storeRefs.keys()],
      fns: [],
      view: roots[0] || null,
      style: componentStyle,
    },
  };

  const emitter = new SourceEmitter();
  const source = emitter.emitDocument(emitInput, new Map());

  return { value: source, errors };
}

function collectPascalCaseTags(roots: import("./types").ViewNode[]): string[] {
  const tags: string[] = [];
  function walk(n: import("./types").ViewNode): void {
    if (/^[A-Z]/.test(n.tag) && !TAG_HTML.has(n.tag)) {
      if (!tags.includes(n.tag)) tags.push(n.tag);
    }
    for (const c of n.children) walk(c);
  }
  for (const r of roots) walk(r);
  return tags;
}

const TAG_HTML = new Set([
  "Container","Flex","Grid","Card","Text","Heading","Button","Input",
  "TextArea","Select","Checkbox","List","ListItem","Image","Spinner",
  "Progress","Alert","Link","Form",
]);

function applyStyles(roots: import("./types").ViewNode[]): void {
  for (const r of roots) {
    walkApply(r);
  }
  function walkApply(n: import("./types").ViewNode): void {
    n.resolvedStyle = resolveStyle(n);
    for (const c of n.children) walkApply(c);
  }
}

function collectStyles(
  roots: import("./types").ViewNode[],
  out: Record<string, Record<string, string>>,
): void {
  for (const r of roots) {
    walkCollect(r);
  }
  function walkCollect(n: import("./types").ViewNode): void {
    if (n.resolvedStyle && Object.keys(n.resolvedStyle.cssRules).length > 0) {
      out[n.resolvedStyle.cssClassName] = n.resolvedStyle.cssRules;
    }
    for (const c of n.children) walkCollect(c);
  }
}
```

- [ ] **Step 2: Integration test**

```typescript
// integration-generate.test.ts
import { describe, it, expect } from "vitest";
import { generateOmniSource } from "../pipeline";
import { validate } from "../validator";

describe("generateOmniSource", () => {
  it("generates valid .omni from simple view tree", () => {
    const result = generateOmniSource({
      designElements: [
        { id: "root", nodeType: "view", tag: "Container", props: { padding: "16px" }, children: ["btn"], parentId: null },
        { id: "btn", nodeType: "view", tag: "Button", props: { label: "Click" }, children: [], parentId: "root" },
      ],
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "📄", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "App",
    });

    expect(result.errors).toEqual([]);
    expect(result.value).toContain("component App {");
    expect(result.value).toContain("class: App");
    expect(result.value).toContain("<button");
    expect(result.value).toContain("Click");
  });
});
```

- [ ] **Step 3: Run — expected PASS if WASM available, or comment out validate call**

- [ ] **Step 4: Commit**

---

### Task 12: StoreExtractor

*(Implement the two-phase extraction as described in the spec — extract stores + standalone signals from code blocks)*

### Task 13: ExpressionLowering

*(Implement recursive descent builder for code graph → omni expressions)*

### Task 14: ChainEmitter

*(Convert code graph elements+wires to @chain/@node/@wire annotation blocks)*

### Task 15: annotation-data.ts

*(Build AnnotationData from view tree + tab type — pure data transformation)*

### Task 16: SourceEmitter annotation integration

*(Wire up AnnotationStrategy in SourceEmitter; test that @edit lines appear correctly)*

### Task 17: Pipeline final — three-phase StageResult

*(Refactor pipeline.ts with full three-phase error collection; add generateOmniFiles for multi-file output)*

### Task 18: TokenEmitter + CustomComponentEmitter

*(theme block generation + custom component file generation)*

### Task 19: App.tsx integration

*(Wire parseOmniSource into openFile; wire generateOmniSource + validate into saveProject and handleCompile)*
