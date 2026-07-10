# Store Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add full visual Store editing (signal/getter/action CRUD + CodeFlow action editing) to Poiesis, with .omni round-trip via `// @pid:xxx` annotations.

**Architecture:** Store definitions live in `storeDefs: Record<string, StoreDefinition>` in Zustand state. The parser's `parseStoreBody()` is rewritten from brace-counting-skip to actual member parsing. The emitter's `emitStore()` outputs `// @pid:xxx` annotations for round-trip stability. StorePage UI uses left panel (member list) + center (FlowCanvas for action editing), reusing existing CodeFlowCanvas without modification by swapping action data into `store.elements`/`store.wires` on action selection.

**Tech Stack:** TypeScript, React 19, Zustand 5, @xyflow/react (ReactFlow), CodeFlowCanvas (existing)

## Global Constraints

- File size: max 800 lines per file
- Function size: max 50 lines
- Immutable state updates (Zustand immer-style spread)
- `// @pid:xxx` annotations required on all store members and action body nodes
- TabData already supports `codeFlowVisible` / `codeFlowElementId` — add `selectedActionPid`
- `TabType` already includes `"store"` — no new enum values needed
- `PidBlockType` already includes `"store"`, `"getter"`, `"action"` — no new PidBlockType values needed
- Follow existing slice pattern: `createStoreCrudSlice(set, get)` returning action objects
- Use `uid()` from `../../core/utils/id` for new PIDs
- Store CRUD follows immutable pattern (same as `design-system.ts` slice)

---

### Task 1: Core store types

**Files:**
- Create: `poiesis/src/core/types/store.ts`
- Modify: `poiesis/src/core/types/index.ts`

**Interfaces:**
- Produces: `StoreSignalDef`, `StoreGetterDef`, `StoreActionDef`, `StoreDefinition` types

- [ ] **Step 1: Create store type definitions**

```typescript
// poiesis/src/core/types/store.ts
import type { CodeblockElementData } from "./element";
import type { WireData } from "./wire";

export interface StoreSignalDef {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface StoreGetterDef {
  pid: string;
  name: string;
  returnType: string;
  expression: string;
}

export interface StoreActionDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType?: string;
  isAsync: boolean;
  elements: CodeblockElementData[];
  wires: WireData[];
}

export interface StoreDefinition {
  pid: string;
  name: string;
  filePath: string;
  imports: string[];
  signals: StoreSignalDef[];
  getters: StoreGetterDef[];
  actions: StoreActionDef[];
}
```

- [ ] **Step 2: Add barrel export**

In `poiesis/src/core/types/index.ts`, add after line 17:

```typescript
export * from "./store";
```

- [ ] **Step 3: Type-check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -10`
Expected: no new errors from types (may show pre-existing errors in other files)

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/store.ts poiesis/src/core/types/index.ts
git commit -m "feat: add StoreSignalDef, StoreGetterDef, StoreActionDef, StoreDefinition types"
```

---

### Task 2: Store CRUD Zustand slice

**Files:**
- Create: `poiesis/src/store/slices/store-crud.ts`
- Modify: `poiesis/src/store/types.ts`
- Modify: `poiesis/src/store/index.ts`

**Interfaces:**
- Consumes: `StoreDefinition`, `StoreSignalDef`, `StoreGetterDef`, `StoreActionDef` from Task 1
- Consumes: `PidEntry` from `../../core/types/pid`, `uid` from `../../core/utils/id`
- Produces: `createStoreCrudSlice(set, get)` → `{ storeDefs, loadStoreDef, removeStoreDef, addStoreSignal, updateStoreSignal, removeStoreSignal, addStoreGetter, updateStoreGetter, removeStoreGetter, addStoreAction, updateStoreAction, removeStoreAction, registerStorePids, registerStoreRef, unregisterStoreRef, unregisterAllStoreRefs }`

- [ ] **Step 1: Create the store CRUD slice**

```typescript
// poiesis/src/store/slices/store-crud.ts
import type { PoiesisState } from "../types";
import type { StoreDefinition, StoreSignalDef, StoreGetterDef, StoreActionDef } from "../../core/types/store";
import type { PidEntry } from "../../core/types/pid";
import { uid } from "../../core/utils/id";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createStoreCrudSlice(set: SetFn, _get?: GetFn) {
  return {
    storeDefs: {} as Record<string, StoreDefinition>,
    storeReferences: {} as Record<string, string[]>,

    loadStoreDef: (def: StoreDefinition) => set((s) => ({
      storeDefs: { ...s.storeDefs, [def.name]: def },
    })),

    removeStoreDef: (name: string) => set((s) => {
      const { [name]: _removed, ...rest } = s.storeDefs;
      return { storeDefs: rest };
    }),

    // ── Signal CRUD ──
    addStoreSignal: (storeName: string, signal: StoreSignalDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, signals: [...store.signals, signal] },
        },
        isDirty: true,
      };
    }),

    updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            signals: store.signals.map((sig) =>
              sig.pid === pid ? { ...sig, ...updates } : sig
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreSignal: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            signals: store.signals.filter((sig) => sig.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── Getter CRUD ──
    addStoreGetter: (storeName: string, getter: StoreGetterDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, getters: [...store.getters, getter] },
        },
        isDirty: true,
      };
    }),

    updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            getters: store.getters.map((g) =>
              g.pid === pid ? { ...g, ...updates } : g
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreGetter: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            getters: store.getters.filter((g) => g.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── Action CRUD ──
    addStoreAction: (storeName: string, action: StoreActionDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, actions: [...store.actions, action] },
        },
        isDirty: true,
      };
    }),

    updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            actions: store.actions.map((a) =>
              a.pid === pid ? { ...a, ...updates } : a
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreAction: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            actions: store.actions.filter((a) => a.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── PID index registration ──
    registerStorePids: (storeDef: StoreDefinition) => set((s) => {
      const entries: Record<string, PidEntry> = {};
      const filePath = storeDef.filePath;
      entries[storeDef.pid] = { pid: storeDef.pid, blockType: "store", blockName: storeDef.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      for (const sig of storeDef.signals) {
        entries[sig.pid] = { pid: sig.pid, blockType: "signal", blockName: sig.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      for (const g of storeDef.getters) {
        entries[g.pid] = { pid: g.pid, blockType: "getter", blockName: g.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      for (const a of storeDef.actions) {
        entries[a.pid] = { pid: a.pid, blockType: "action", blockName: a.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      return { pidIndex: { ...s.pidIndex, ...entries } };
    }),

    // ── @store reference tracking ──
    registerStoreRef: (componentName: string, storeName: string) => set((s) => {
      const existing = s.storeReferences[storeName] || [];
      if (existing.includes(componentName)) return s;
      return { storeReferences: { ...s.storeReferences, [storeName]: [...existing, componentName] } };
    }),

    unregisterStoreRef: (componentName: string, storeName: string) => set((s) => {
      const existing = s.storeReferences[storeName];
      if (!existing) return s;
      return {
        storeReferences: {
          ...s.storeReferences,
          [storeName]: existing.filter((c) => c !== componentName),
        },
      };
    }),

    unregisterAllStoreRefs: (componentName: string) => set((s) => {
      const updated: Record<string, string[]> = {};
      let changed = false;
      for (const [storeName, refs] of Object.entries(s.storeReferences)) {
        const filtered = refs.filter((c) => c !== componentName);
        if (filtered.length !== refs.length) changed = true;
        updated[storeName] = filtered;
      }
      return changed ? { storeReferences: updated } : s;
    }),
  };
}
```

- [ ] **Step 2: Add types to PoiesisState**

In `poiesis/src/store/types.ts`, add after the existing state fields (after `undoStacks`):

```typescript
import type { StoreDefinition, StoreSignalDef, StoreGetterDef, StoreActionDef } from "../core/types/store";

// In PoiesisState interface, add:
storeDefs: Record<string, StoreDefinition>;
storeReferences: Record<string, string[]>;

// Store lifecycle
loadStoreDef: (def: StoreDefinition) => void;
removeStoreDef: (name: string) => void;

// Signal CRUD
addStoreSignal: (storeName: string, signal: StoreSignalDef) => void;
updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => void;
removeStoreSignal: (storeName: string, pid: string) => void;

// Getter CRUD
addStoreGetter: (storeName: string, getter: StoreGetterDef) => void;
updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => void;
removeStoreGetter: (storeName: string, pid: string) => void;

// Action CRUD
addStoreAction: (storeName: string, action: StoreActionDef) => void;
updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => void;
removeStoreAction: (storeName: string, pid: string) => void;

// PID + references
registerStorePids: (storeDef: StoreDefinition) => void;
registerStoreRef: (componentName: string, storeName: string) => void;
unregisterStoreRef: (componentName: string, storeName: string) => void;
unregisterAllStoreRefs: (componentName: string) => void;
```

- [ ] **Step 3: Wire into Zustand store**

In `poiesis/src/store/index.ts`:

Add import:
```typescript
import { createStoreCrudSlice } from "./slices/store-crud";
```

Add initial state (after `undoStacks: {},`):
```typescript
storeDefs: {},
storeReferences: {},
```

Add to composed actions (after `...createSerializationSlice(set, get),`):
```typescript
...createStoreCrudSlice(set, get),
```

- [ ] **Step 4: Build check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: no new type errors from the store slice

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/store/slices/store-crud.ts poiesis/src/store/types.ts poiesis/src/store/index.ts
git commit -m "feat: add store CRUD Zustand slice with signal/getter/action management"
```

---

### Task 3: TabData extension for store

**Files:**
- Modify: `poiesis/src/store/tab-helpers.ts`

**Interfaces:**
- Consumes: None new
- Produces: `TabDataEntry` with new `selectedActionPid?: string | null` field

- [ ] **Step 1: Add selectedActionPid to TabDataEntry**

In `poiesis/src/store/tab-helpers.ts`, add to `TabDataEntry` interface:

```typescript
export interface TabDataEntry {
  elements: ElementData[];
  wires: WireData[];
  rootOrder: string[];
  selectedElementIds: string[];
  filePath?: string;
  codeFlowVisible?: boolean;
  codeFlowElementId?: string | null;
  /** Store tab: currently selected action PID */
  selectedActionPid?: string | null;
}
```

In the `buildTabSaveEntry` function, add to the return value:

```typescript
return {
  elements: source.elements,
  wires: source.wires,
  rootOrder: source.rootOrder,
  selectedElementIds: source.selectedElementIds,
  filePath: source.filePath,
  codeFlowVisible: source.designPage.codeFlowVisible,
  codeFlowElementId: source.designPage.codeFlowElementId,
  selectedActionPid: (source as { selectedActionPid?: string | null }).selectedActionPid ?? null,
};
```

- [ ] **Step 2: Preserve selectedActionPid during tab saves**

In `poiesis/src/store/operations/tab-lifecycle.ts`, in the `switchTab` function, ensure `selectedActionPid` is preserved from existing tabData when saving. After `buildTabSaveEntry(...)`, merge in the existing entry's `selectedActionPid`:

```typescript
// In switchTab, after buildTabSaveEntry:
const existingEntry = s.tabData[oldTabId!];
const saveEntry = buildTabSaveEntry({...});
if (existingEntry?.selectedActionPid) {
  saveEntry.selectedActionPid = existingEntry.selectedActionPid;
}
```

Same pattern for `closeTab` and `openTab` where `buildTabSaveEntry` is called.

- [ ] **Step 3: Build check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -10`
Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/store/tab-helpers.ts poiesis/src/store/operations/tab-lifecycle.ts
git commit -m "feat: add selectedActionPid to TabDataEntry for store tab persistence"
```

---

### Task 4: Parser — rewrite parseStoreBody

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`
- Modify: `poiesis/src/services/codegen/types.ts`
- Modify: `poiesis/src/services/codegen/parser-types.ts` (if new parse context fields needed)

**Interfaces:**
- Consumes: `StoreDefinition`, `StoreSignalDef`, `StoreGetterDef`, `StoreActionDef` from Task 1
- Consumes: `parseCodeFlowGraph` from `./graph-parser` (already imported at parser.ts:9)
- Consumes: `uid` from `../../core/utils/id` (already imported at parser.ts:21)
- Produces: Updated `Parser.parseStoreBody()` that returns `StoreDefinition` instead of skipping
- Produces: `ParseResult.storeDef?: StoreDefinition`

- [ ] **Step 1: Add parse context fields**

In `poiesis/src/services/codegen/parser-types.ts`, add to `ParseContext`:

```typescript
// In ParseContext interface, add:
currentStore: string;
currentStorePid: string;
storeDef: import("../core/types/store").StoreDefinition | null;
storeSignalCounter: number;
storeActionCounter: number;
storeGetterCounter: number;
parsedStorePid: string | null;
```

In the context creation function (in parser.ts, `createContext`), initialize these:

```typescript
currentStore: "",
currentStorePid: "",
storeDef: null,
storeSignalCounter: 0,
storeActionCounter: 0,
storeGetterCounter: 0,
parsedStorePid: null,
```

- [ ] **Step 2: Replace parseStoreBody with actual parser**

Replace the current brace-counting `parseStoreBody()` in `poiesis/src/services/codegen/parser.ts` (lines ~503-511):

```typescript
private parseStoreBody(): void {
  const storeDef: import("../core/types/store").StoreDefinition = {
    pid: this.ctx.currentStorePid || `s${this.ctx.storeDef ? Object.keys(this.ctx.storeDef).length + 1 : 1}`,
    name: this.ctx.currentStore,
    filePath: "",
    imports: [],
    signals: [],
    getters: [],
    actions: [],
  };
  let pendingPid: string | null = this.ctx.parsedStorePid;

  while (!this.stream.isEof()) {
    this.stream.skipNewlines();
    if (this.stream.isEof()) break;

    const tok = this.stream.peek();

    // Close store
    if (tok.kind === "BRACE_CLOSE") {
      this.stream.consume();
      this.ctx.storeDef = storeDef;
      this.popState();
      return;
    }

    // Parse @pid comment
    if (tok.kind === "COMMENT_LINE" && tok.text.trim().startsWith("// @pid:")) {
      pendingPid = tok.text.trim().slice(8).trim();
      this.stream.consume();
      continue;
    }

    // Parse import
    if (tok.kind === "IDENTIFIER" && tok.text === "import") {
      const importLine = this.collectUntilNewline();
      storeDef.imports.push("import" + importLine);
      continue;
    }

    // Parse signal
    if (tok.kind === "IDENTIFIER" && tok.text === "signal") {
      this.stream.consume(); // "signal"
      const nameTok = this.stream.expect("IDENTIFIER");
      const colon = this.stream.consume(); // ":"
      const typeStr = this.collectType();
      let defaultValue: unknown = undefined;
      const next = this.stream.peek();
      if (next.kind === "OPERATOR" && next.text === "=") {
        this.stream.consume(); // "="
        defaultValue = this.collectDefaultValue();
      }
      this.expectSemicolon();
      const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeSignalCounter}`;
      pendingPid = null;
      storeDef.signals.push({ pid, name: nameTok.text, type: typeStr, defaultValue });
      continue;
    }

    // Parse getter
    if (tok.kind === "IDENTIFIER" && tok.text === "getter") {
      this.stream.consume(); // "getter"
      const nameTok = this.stream.expect("IDENTIFIER");
      this.stream.consume(); // ":"
      const returnType = this.collectType();
      this.stream.consume(); // "="
      const expression = this.collectExpressionUntilSemicolon();
      this.expectSemicolon();
      const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeGetterCounter}`;
      pendingPid = null;
      storeDef.getters.push({ pid, name: nameTok.text, returnType, expression });
      continue;
    }

    // Parse action
    if (tok.kind === "IDENTIFIER" && tok.text === "action") {
      this.stream.consume(); // "action"
      const nameTok = this.stream.expect("IDENTIFIER");
      this.stream.consume(); // "("
      const params = this.collectActionParams();
      this.stream.consume(); // ")"
      let returnType: string | undefined;
      let isAsync = false;
      // Optional ": ReturnType"
      if (this.stream.peek().kind === "OPERATOR" && this.stream.peek().text === ":") {
        this.stream.consume(); // ":"
        returnType = this.collectType();
      }
      // Optional "async"
      if (this.stream.peek().kind === "IDENTIFIER" && this.stream.peek().text === "async") {
        this.stream.consume();
        isAsync = true;
      }
      this.stream.expect("BRACE_OPEN");
      // Collect action body (until matching close brace at same indentation)
      const actionBody = this.collectActionBody();
      const { elements, wires } = parseCodeFlowGraph(actionBody);
      const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeActionCounter}`;
      pendingPid = null;
      storeDef.actions.push({ pid, name: nameTok.text, params, returnType, isAsync, elements, wires });
      continue;
    }

    // Skip unknown tokens
    this.stream.consume();
  }
}
```

- [ ] **Step 3: Add helper methods**

Add these private methods to the `Parser` class:

```typescript
/** Collect tokens until NEWLINE — used for import lines. */
private collectUntilNewline(): string {
  let result = "";
  while (!this.stream.isEof() && this.stream.peek().kind !== "NEWLINE") {
    result += this.stream.consume().text;
  }
  return result;
}

/** Collect a type annotation string (e.g. "String", "Todo[]", "Array<String>"). */
private collectType(): string {
  let typeStr = "";
  let depth = 0;
  while (!this.stream.isEof()) {
    const t = this.stream.peek();
    if (depth === 0 && (t.kind === "SEMICOLON" || t.kind === "OPERATOR" && t.text === "=" || t.kind === "NEWLINE" || t.kind === "BRACE_CLOSE" || t.kind === "BRACE_OPEN" || t.kind === "NEWLINE")) break;
    if (t.kind === "OPERATOR" && t.text === "<") depth++;
    if (t.kind === "OPERATOR" && t.text === ">") depth--;
    typeStr += this.stream.consume().text;
  }
  return typeStr.trim();
}

/** Collect a default value expression after "=", stopping at ";". */
private collectDefaultValue(): unknown {
  let val = "";
  while (!this.stream.isEof() && this.stream.peek().kind !== "SEMICOLON") {
    val += this.stream.consume().text;
  }
  const trimmed = val.trim();
  // Return as string (caller can JSON.parse if needed)
  return trimmed;
}

/** Collect an expression until ";" (possibly multi-line). */
private collectExpressionUntilSemicolon(): string {
  let expr = "";
  while (!this.stream.isEof() && this.stream.peek().kind !== "SEMICOLON") {
    expr += this.stream.consume().text;
  }
  return expr.trim();
}

/** Expect and consume a semicolon. */
private expectSemicolon(): void {
  if (this.stream.peek().kind === "SEMICOLON") {
    this.stream.consume();
  }
}

/** Collect action params inside "(...)", comma-separated. */
private collectActionParams(): Array<{ name: string; type: string }> {
  const params: Array<{ name: string; type: string }> = [];
  while (!this.stream.isEof() && this.stream.peek().kind !== "PAREN_CLOSE") {
    const name = this.stream.expect("IDENTIFIER").text;
    this.stream.consume(); // ":"
    const pType = this.collectType();
    params.push({ name, type: pType });
    if (this.stream.peek().kind === "OPERATOR" && this.stream.peek().text === ",") {
      this.stream.consume(); // ","
    }
  }
  return params;
}

/** Collect action body between braces, preserving @pid comments for parseCodeFlowGraph. */
private collectActionBody(): string {
  let body = "";
  let depth = 1;
  while (depth > 0 && !this.stream.isEof()) {
    const tok = this.stream.consume();
    if (tok.kind === "BRACE_OPEN") depth++;
    else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth === 0) break; }
    if (tok.kind === "NEWLINE") body += "\n";
    else body += tok.text + (tok.kind === "COMMENT_LINE" ? "" : " ");
  }
  return body.trim();
}
```

- [ ] **Step 4: Handle store PID in root-level parsing**

In `parseRootKeyword()` where `store` is encountered (around line 247), update to set `currentStorePid`:

```typescript
if (kw === "store") {
  this.stream.consume();
  const nameTok = this.stream.expect("IDENTIFIER");
  if (nameTok.kind === "EOF") {
    this.pushError("error", "P003", "Invalid store declaration: missing name", nameTok);
    return;
  }
  this.ctx.currentStore = nameTok.text;
  this.ctx.currentStorePid = ""; // will be filled by @pid comment or auto-generated
  this.ctx.storeSignalCounter = 0;
  this.ctx.storeActionCounter = 0;
  this.ctx.storeGetterCounter = 0;
  this.ctx.parsedStorePid = null;
  this.stream.expect("BRACE_OPEN");
  this.pushState("IN_STORE");
  return;
}
```

- [ ] **Step 5: Add storeDef to ParseResult**

In `poiesis/src/services/codegen/types.ts`, add to `ParseResult`:

```typescript
/** Parsed store definition (when source is a store .omni file). */
storeDef?: import("../core/types/store").StoreDefinition;
```

- [ ] **Step 6: Build check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: resolve any type errors from the parser changes

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/parser-types.ts
git commit -m "feat: rewrite parseStoreBody to actually parse signal/getter/action members"
```

---

### Task 5: Emitter — enhance emitStore with @pid annotations

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts`
- Modify: `poiesis/src/services/codegen/pipeline.ts`

**Interfaces:**
- Consumes: `StoreDefinition` from Task 1
- Consumes: `compileCodeFlowGraph` from `./graph-compiler` (already available)
- Produces: Updated `emitStore()` method that outputs `// @pid:xxx` annotations and action CodeFlow bodies

- [ ] **Step 1: Rewrite emitStore method**

In `poiesis/src/services/codegen/source-emitter.ts`, replace the existing `emitStore` method (around lines 69-83):

```typescript
import type { StoreDefinition } from "../core/types/store";
import { compileCodeFlowGraph } from "./graph-compiler";

private emitStore(store: StoreDefinition, w: CodeWriter): void {
  // Imports
  for (const imp of store.imports) {
    w.emit(imp);
  }
  if (store.imports.length > 0) w.blank();

  // Store header
  w.emit(`// @pid:${store.pid}`);
  w.emit(`store ${store.name} {`);
  w.indent();

  // Signals
  for (const sig of store.signals) {
    const defStr = sig.defaultValue !== undefined && sig.defaultValue !== ""
      ? ` = ${sig.defaultValue}`
      : "";
    w.emit(`signal ${sig.name}: ${sig.type}${defStr}; // @pid:${sig.pid}`);
  }

  // Getters
  if (store.getters.length > 0 && store.signals.length > 0) w.blank();
  for (const g of store.getters) {
    w.emit(`getter ${g.name}: ${g.returnType} = ${g.expression}; // @pid:${g.pid}`);
  }

  // Actions
  for (const a of store.actions) {
    if (store.signals.length + store.getters.length > 0 || a !== store.actions[0]) w.blank();
    const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
    const retStr = a.returnType ? `: ${a.returnType}` : "";
    const asyncStr = a.isAsync ? " async" : "";
    w.emit(`action ${a.name}(${paramsStr})${retStr}${asyncStr} { // @pid:${a.pid}`);

    // Emit action body via CodeFlow compiler
    if (a.elements.length > 0) {
      w.indent();
      const actionFnPid = a.elements.find((e) => e.tag === "fn")?.id;
      if (actionFnPid) {
        const body = compileCodeFlowGraph(a.elements, a.wires, actionFnPid);
        // compileCodeFlowGraph includes fn { ... } wrapper, just emit inner body lines
        const lines = body.split("\n");
        for (const line of lines) {
          const trimmed = line.trim();
          // Skip the fn signature line and the closing }
          if (trimmed.startsWith("// @pid:") && trimmed.includes("fn ")) continue;
          if (trimmed.startsWith("fn ")) continue;
          if (trimmed === "}" && lines.indexOf(line) === lines.length - 1) continue;
          if (trimmed.startsWith("// @pid:") || trimmed.length > 0) {
            w.emit(trimmed);
          }
        }
      }
      w.dedent();
    }
    w.emit(`}`);
  }

  w.dedent();
  w.emit(`}`);
}
```

> **Note on action body emission:** The `compileCodeFlowGraph` outputs a complete `fn name(params) { ... }`. For store actions, we need to strip the outer fn wrapper and emit only the body lines with their `// @pid:xxx` comments. The implementation above handles this by filtering out the `fn name(params) {` header line and the final `}`. This is a pragmatic approach — a cleaner alternative would be to add a `compileCodeFlowGraphBody()` variant to `graph-compiler.ts` that only emits body content, but that can be done as a follow-up refactor.

- [ ] **Step 2: Update pipeline to use new emitStore**

In `poiesis/src/services/codegen/pipeline.ts`, update `generateOmniFiles()` (around lines 333-370): for store type files, read from `storeDefs` instead of re-extracting from designElements:

```typescript
// In generateOmniFiles, before the store re-extraction block, check if input is store type:
const store = usePoiesisStore.getState().storeDefs[options.componentName];
if (store) {
  const storeSource = emitter.emitDocument({
    codeTabType: "logic",
    imports: store.imports,
    stores: [store], // Will be handled by emitStore
    component: { status: "partial", name: store.name, signals: {}, storeRefs: [] },
  });
  // ... write store source to files[filePath]
}
```

For now, the simpler path: in the emitter, when `input.stores` has entries, call `emitStore` for each. Keep the existing code path for component emission.

- [ ] **Step 3: Build check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: resolve type errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts poiesis/src/services/codegen/pipeline.ts
git commit -m "feat: enhance emitStore with @pid annotations and CodeFlow action bodies"
```

---

### Task 6: StorePage UI — member panel + CodeFlow integration

**Files:**
- Create: `poiesis/src/pages/StorePage.tsx`
- Create: `poiesis/src/components/store/StoreMembersPanel.tsx`
- Modify: `poiesis/src/pages/DesignPage.tsx`

**Interfaces:**
- Consumes: `usePoiesisStore` with `storeDefs`, store CRUD actions from Task 2
- Consumes: `FlowCanvas` from `../components/canvas/code/FlowCanvas`
- Consumes: `CodeEditorModal` from `../components/canvas/code/CodeEditorModal`
- Produces: `StorePage` component (left members panel + center FlowCanvas)

- [ ] **Step 1: Create StoreMembersPanel**

```typescript
// poiesis/src/components/store/StoreMembersPanel.tsx
import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { uid } from "../../core/utils/id";
import type { StoreSignalDef, StoreGetterDef, StoreActionDef } from "../../core/types/store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";

interface Props {
  storeName: string;
  selectedActionPid: string | null;
  onSelectAction: (pid: string) => void;
}

export function StoreMembersPanel({ storeName, selectedActionPid, onSelectAction }: Props) {
  const store = usePoiesisStore((s) => s.storeDefs[storeName]);
  const addStoreSignal = usePoiesisStore((s) => s.addStoreSignal);
  const updateStoreSignal = usePoiesisStore((s) => s.updateStoreSignal);
  const removeStoreSignal = usePoiesisStore((s) => s.removeStoreSignal);
  const addStoreGetter = usePoiesisStore((s) => s.addStoreGetter);
  const updateStoreGetter = usePoiesisStore((s) => s.updateStoreGetter);
  const removeStoreGetter = usePoiesisStore((s) => s.removeStoreGetter);
  const addStoreAction = usePoiesisStore((s) => s.addStoreAction);
  const updateStoreAction = usePoiesisStore((s) => s.updateStoreAction);
  const removeStoreAction = usePoiesisStore((s) => s.removeStoreAction);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);

  const [editingGetterPid, setEditingGetterPid] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  if (!store) return <div style={{ padding: 16, color: "#999" }}>Store not found: {storeName}</div>;

  const toggle = (key: string) => setCollapsed((c) => ({ ...c, [key]: !c[key] }));

  const handleAddSignal = () => {
    const storePid = store.pid;
    const idx = store.signals.length + 1;
    addStoreSignal(storeName, {
      pid: `${storePid}_n${idx}`,
      name: "newSignal",
      type: "String",
      defaultValue: "",
    });
  };

  const handleAddGetter = () => {
    const storePid = store.pid;
    const idx = store.getters.length + store.signals.length + 1;
    addStoreGetter(storeName, {
      pid: `${storePid}_n${idx}`,
      name: "newGetter",
      returnType: "String",
      expression: "",
    });
  };

  const handleAddAction = () => {
    const storePid = store.pid;
    const idx = store.actions.length + store.signals.length + store.getters.length + 1;
    const actionPid = `${storePid}_n${idx}`;
    const fnPid = `${actionPid}_c1`;
    addStoreAction(storeName, {
      pid: actionPid,
      name: "newAction",
      params: [],
      returnType: undefined,
      isAsync: false,
      elements: [{
        id: fnPid,
        nodeType: "codeblock",
        tag: "fn",
        parentId: null,
        props: { name: "newAction", params: "" },
        source: "",
      }],
      wires: [],
    });
  };

  const editingGetter = editingGetterPid
    ? store.getters.find((g) => g.pid === editingGetterPid)
    : null;

  return (
    <div style={{ height: "100%", overflow: "auto", padding: 8, borderRight: "1px solid #e5e7eb" }}>
      {/* Signals */}
      <div style={{ marginBottom: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }} onClick={() => toggle("signals")}>
          <strong>📁 Signals</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddSignal(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.signals && store.signals.map((sig) => (
          <div key={sig.pid} style={{ padding: "4px 8px", background: "#f9fafb", borderRadius: 4, marginTop: 4, fontSize: 13 }}>
            <input
              style={{ width: "40%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={sig.name}
              onChange={(e) => updateStoreSignal(storeName, sig.pid, { name: e.target.value })}
            />
            <span> : </span>
            <input
              style={{ width: "25%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={sig.type}
              onChange={(e) => updateStoreSignal(storeName, sig.pid, { type: e.target.value })}
            />
            {sig.defaultValue !== undefined && (
              <>
                <span> = </span>
                <input
                  style={{ width: "20%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
                  value={String(sig.defaultValue ?? "")}
                  onChange={(e) => updateStoreSignal(storeName, sig.pid, { defaultValue: e.target.value })}
                />
              </>
            )}
            <button onClick={() => removeStoreSignal(storeName, sig.pid)} style={{ marginLeft: 4, color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}>✕</button>
          </div>
        ))}
      </div>

      {/* Getters */}
      <div style={{ marginBottom: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }} onClick={() => toggle("getters")}>
          <strong>📁 Getters</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddGetter(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.getters && store.getters.map((g) => (
          <div key={g.pid} style={{ padding: "4px 8px", background: "#f9fafb", borderRadius: 4, marginTop: 4, fontSize: 13 }}>
            <input
              style={{ width: "35%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={g.name}
              onChange={(e) => updateStoreGetter(storeName, g.pid, { name: e.target.value })}
            />
            <span> : </span>
            <input
              style={{ width: "20%", border: "1px solid #e5e7eb", borderRadius: 3, padding: "2px 4px" }}
              value={g.returnType}
              onChange={(e) => updateStoreGetter(storeName, g.pid, { returnType: e.target.value })}
            />
            <span style={{ display: "block", marginTop: 2, fontSize: 12, color: "#6366f1", cursor: "pointer" }} onClick={() => setEditingGetterPid(g.pid)}>
              {g.expression ? g.expression.slice(0, 40) + (g.expression.length > 40 ? "..." : "") : "[click to edit expression]"}
            </span>
            <button onClick={() => removeStoreGetter(storeName, g.pid)} style={{ marginLeft: 4, color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}>✕</button>
          </div>
        ))}
      </div>

      {/* Actions */}
      <div style={{ marginBottom: 8 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", cursor: "pointer" }} onClick={() => toggle("actions")}>
          <strong>📁 Actions</strong>
          <button onClick={(e) => { e.stopPropagation(); handleAddAction(); }} style={{ padding: "2px 6px" }}>+</button>
        </div>
        {!collapsed.actions && store.actions.map((a) => {
          const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
          const retStr = a.returnType ? `: ${a.returnType}` : "";
          const asyncStr = a.isAsync ? " async" : "";
          const isSelected = selectedActionPid === a.pid;
          return (
            <div
              key={a.pid}
              onClick={() => onSelectAction(a.pid)}
              style={{
                padding: "6px 8px",
                background: isSelected ? "#eef2ff" : "#f9fafb",
                borderRadius: 4,
                marginTop: 4,
                fontSize: 13,
                cursor: "pointer",
                border: isSelected ? "2px solid #6366f1" : "1px solid #e5e7eb",
                fontWeight: isSelected ? 600 : 400,
              }}
            >
              {isSelected && "● "}{a.name}({paramsStr}){retStr}{asyncStr}
              <button
                onClick={(e) => { e.stopPropagation(); removeStoreAction(storeName, a.pid); }}
                style={{ float: "right", color: "#ef4444", border: "none", background: "none", cursor: "pointer" }}
              >✕</button>
            </div>
          );
        })}
      </div>

      {/* Getter Monaco Modal */}
      {editingGetter && (
        <CodeEditorModal
          title={`Edit Getter: ${editingGetter.name}`}
          initialCode={editingGetter.expression}
          onSave={(code: string) => {
            updateStoreGetter(storeName, editingGetter.pid, { expression: code });
            setEditingGetterPid(null);
          }}
          onClose={() => setEditingGetterPid(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create StorePage**

```typescript
// poiesis/src/pages/StorePage.tsx
import { useCallback, useEffect } from "react";
import { usePoiesisStore } from "../store";
import { StoreMembersPanel } from "../components/store/StoreMembersPanel";
import { FlowCanvas } from "../components/canvas/code/FlowCanvas";
import { ReactFlowProvider } from "@xyflow/react";

interface Props {
  storeName: string;
  tabId: string;
}

export function StorePage({ storeName, tabId }: Props) {
  const store = usePoiesisStore((s) => s.storeDefs[storeName]);
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const setElementsAndWires = usePoiesisStore((s) => s.setElementsAndWires);
  const tabData = usePoiesisStore((s) => s.tabData);
  const updateStoreAction = usePoiesisStore((s) => s.updateStoreAction);

  // Read selectedActionPid from tabData
  const selectedActionPid = tabData[tabId]?.selectedActionPid ?? null;

  const handleSelectAction = useCallback((pid: string) => {
    // Save current action's elements/wires back to storeDefs
    if (selectedActionPid && store) {
      const currentAction = store.actions.find((a) => a.pid === selectedActionPid);
      if (currentAction) {
        updateStoreAction(storeName, selectedActionPid, {
          elements: elements as any,
          wires: wires as any,
        });
      }
    }

    // Load selected action's elements/wires into store
    if (store) {
      const action = store.actions.find((a) => a.pid === pid);
      if (action) {
        setElementsAndWires(action.elements, action.wires);
      }
    }

    // Update tabData with new selection
    usePoiesisStore.setState((s) => ({
      tabData: {
        ...s.tabData,
        [tabId]: { ...s.tabData[tabId], selectedActionPid: pid },
      },
    }));
  }, [storeName, store, selectedActionPid, elements, wires, tabId, updateStoreAction, setElementsAndWires]);

  // Load first action on mount if none selected
  useEffect(() => {
    if (!selectedActionPid && store && store.actions.length > 0) {
      handleSelectAction(store.actions[0].pid);
    }
  }, []);

  if (!store) {
    return <div style={{ padding: 32, textAlign: "center", color: "#999" }}>Store not loaded</div>;
  }

  return (
    <div style={{ display: "flex", height: "100%", width: "100%" }}>
      {/* Left: Members Panel */}
      <div style={{ width: 300, minWidth: 200, height: "100%" }}>
        <StoreMembersPanel
          storeName={storeName}
          selectedActionPid={selectedActionPid}
          onSelectAction={handleSelectAction}
        />
      </div>

      {/* Center: CodeFlow Canvas */}
      <div style={{ flex: 1, height: "100%" }}>
        {selectedActionPid ? (
          <ReactFlowProvider>
            <FlowCanvas />
          </ReactFlowProvider>
        ) : (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#999" }}>
            Select an action to edit
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Route StorePage in DesignPage**

In `poiesis/src/pages/DesignPage.tsx`, find where `CodeFlowCanvas` is rendered (around line 329) and add the store routing. Add import:

```typescript
import { StorePage } from "./StorePage";
```

Then in the center area (around line 300, where the center column renders), add before the existing CodeFlowCanvas routing:

```typescript
{activeTabType === "store" && activeTab && (
  <StorePage storeName={activeTab.componentName} tabId={activeTab.id} />
)}
```

- [ ] **Step 4: Build and verify**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: resolve all type errors in the new components

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/pages/StorePage.tsx poiesis/src/components/store/StoreMembersPanel.tsx poiesis/src/pages/DesignPage.tsx
git commit -m "feat: add StorePage UI with member panel and CodeFlow action editing"
```

---

### Task 7: End-to-end round-trip verification

**Files:**
- Create: `poiesis/tests/store-roundtrip.test.ts`

**Interfaces:**
- Consumes: Parser from Task 4, Emitter from Task 5
- Produces: Test file verifying parse → emit → parse identity

- [ ] **Step 1: Write round-trip test**

```typescript
// poiesis/tests/store-roundtrip.test.ts
import { describe, it, expect } from "vitest";
// Note: import actual parse/emit functions based on project conventions

const sampleStoreSource = `import omniview;
import { storage_get, storage_set } from "std/storage";
// @pid:s1
store TodoStore {
    signal items: Todo[] = []; // @pid:s1_n1
    signal filterStatus: String = "all"; // @pid:s1_n2

    getter totalCount: i32 = items.len(); // @pid:s1_n3

    action add(text: String) { // @pid:s1_n4
        // @pid:s1_n4_c1
        items = [...items, { id: items.len(), text, done: false }];
        // @pid:s1_n4_c2
        lastAction = "added";
    }

    action clear() { // @pid:s1_n5
        // @pid:s1_n5_c1
        items = [];
    }
}
`;

describe("Store round-trip", () => {
  it("parses store source into StoreDefinition", () => {
    // Parse
    const result = parseOmniSource(sampleStoreSource);
    expect(result.storeDef).toBeDefined();
    expect(result.storeDef!.name).toBe("TodoStore");
    expect(result.storeDef!.pid).toBe("s1");
    expect(result.storeDef!.signals).toHaveLength(2);
    expect(result.storeDef!.getters).toHaveLength(1);
    expect(result.storeDef!.actions).toHaveLength(2);
  });

  it("parsed store signals have correct data", () => {
    const result = parseOmniSource(sampleStoreSource);
    const store = result.storeDef!;
    expect(store.signals[0].name).toBe("items");
    expect(store.signals[0].type).toBe("Todo[]");
    expect(store.signals[0].defaultValue).toBe("[]");
    expect(store.signals[1].name).toBe("filterStatus");
  });

  it("parsed store getter has correct data", () => {
    const result = parseOmniSource(sampleStoreSource);
    const getter = result.storeDef!.getters[0];
    expect(getter.name).toBe("totalCount");
    expect(getter.returnType).toBe("i32");
    expect(getter.expression).toContain("items.len()");
  });

  it("parsed store actions have CodeFlow data", () => {
    const result = parseOmniSource(sampleStoreSource);
    const addAction = result.storeDef!.actions.find((a) => a.name === "add");
    expect(addAction).toBeDefined();
    expect(addAction!.elements.length).toBeGreaterThan(0);
    expect(addAction!.wires.length).toBeGreaterThan(0);
    // Should have fn, code-block nodes
    expect(addAction!.elements.some((e) => e.tag === "fn")).toBe(true);
    expect(addAction!.elements.some((e) => e.tag === "code-block")).toBe(true);
  });

  it("emit reproduces parseable source with same structure", () => {
    const parsed = parseOmniSource(sampleStoreSource);
    const emitted = emitStoreBlock(parsed.storeDef!); // Assuming emitStoreBlock is exported
    const reparsed = parseOmniSource(emitted);
    expect(reparsed.storeDef!.name).toBe(parsed.storeDef!.name);
    expect(reparsed.storeDef!.signals.length).toBe(parsed.storeDef!.signals.length);
    expect(reparsed.storeDef!.getters.length).toBe(parsed.storeDef!.getters.length);
    expect(reparsed.storeDef!.actions.length).toBe(parsed.storeDef!.actions.length);
  });

  it("emitted source preserves @pid annotations", () => {
    const parsed = parseOmniSource(sampleStoreSource);
    const emitted = emitStoreBlock(parsed.storeDef!);
    expect(emitted).toContain("// @pid:s1");
    expect(emitted).toContain("// @pid:s1_n1");
    expect(emitted).toContain("// @pid:s1_n4");
  });
});
```

- [ ] **Step 2: Run tests**

Run: `cd poiesis && npx vitest run tests/store-roundtrip.test.ts --reporter=verbose`
Expected: all tests pass

- [ ] **Step 3: Fix any failures**

Adjust parse/emit implementations based on test failures. Iterate until all pass.

- [ ] **Step 4: Commit**

```bash
git add poiesis/tests/store-roundtrip.test.ts
git commit -m "test: add store parse-emit round-trip tests with @pid validation"
```

---

### Task 8: CodeFlowCanvas FN node params button

**Files:**
- Modify: `poiesis/src/components/canvas/code/nodes/FnNode.tsx`

**Interfaces:**
- Consumes: None new (uses existing @xyflow/react Handle, Position)
- Produces: Enhanced `FnNode` with inline params editor

- [ ] **Step 1: Add params editing to FnNode**

Replace `poiesis/src/components/canvas/code/nodes/FnNode.tsx`:

```typescript
import { useState } from "react";
import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    fnName?: string;
    params?: string;
    /** Callback when params change. Called with new params string like "text: String, id: i32". */
    onParamsChange?: (newParams: string) => void;
    onDoubleClick?: () => void;
  };
}

export function FnNode({ data }: Props) {
  const name = data.fnName || data.label || "fn";
  const params = data.params || "";
  const [showParams, setShowParams] = useState(false);

  // Parse current params string into array
  const parsedParams = params
    ? params.split(",").map((p) => {
        const [pName, pType] = p.trim().split(":").map((s) => s.trim());
        return { name: pName || "", type: pType || "" };
      })
    : [];

  const handleToggleParams = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowParams(!showParams);
  };

  const handleParamChange = (index: number, field: "name" | "type", value: string) => {
    const newParams = [...parsedParams];
    newParams[index] = { ...newParams[index], [field]: value };
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  const handleAddParam = () => {
    const newParams = [...parsedParams, { name: "param", type: "String" }];
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  const handleRemoveParam = (index: number) => {
    const newParams = parsedParams.filter((_, i) => i !== index);
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  return (
    <div
      className="flow-node"
      data-node-type="fn"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#EEF2FF",
        border: "2px solid #6366F1",
        borderRadius: "12px 12px 4px 4px",
        padding: "10px 18px",
        minWidth: 160,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
        color: "#3730A3",
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 4 }}>
        <span style={{ fontSize: 14 }}>ƒ</span>
        <span>{name}</span>
        {params && <span style={{ fontWeight: 400 }}>({params})</span>}
        <button
          onClick={handleToggleParams}
          title="Edit params"
          style={{
            marginLeft: 6,
            padding: "1px 5px",
            fontSize: 11,
            border: "1px solid #6366F1",
            borderRadius: 3,
            background: showParams ? "#6366F1" : "#EEF2FF",
            color: showParams ? "#fff" : "#6366F1",
            cursor: "pointer",
          }}
        >
          {showParams ? "✕" : "+params"}
        </button>
      </div>

      {showParams && (
        <div
          style={{ marginTop: 8, textAlign: "left", fontSize: 12, fontWeight: 400 }}
          onClick={(e) => e.stopPropagation()}
        >
          {parsedParams.map((p, i) => (
            <div key={i} style={{ display: "flex", gap: 4, marginBottom: 3, alignItems: "center" }}>
              <input
                style={{ width: 60, border: "1px solid #d4d4d8", borderRadius: 3, padding: "1px 4px" }}
                value={p.name}
                onChange={(e) => handleParamChange(i, "name", e.target.value)}
                placeholder="name"
              />
              <span>:</span>
              <input
                style={{ width: 55, border: "1px solid #d4d4d8", borderRadius: 3, padding: "1px 4px" }}
                value={p.type}
                onChange={(e) => handleParamChange(i, "type", e.target.value)}
                placeholder="type"
              />
              <button
                onClick={() => handleRemoveParam(i)}
                style={{ border: "none", background: "none", color: "#ef4444", cursor: "pointer", fontSize: 12 }}
              >
                ✕
              </button>
            </div>
          ))}
          <button
            onClick={handleAddParam}
            style={{ marginTop: 4, padding: "1px 8px", fontSize: 11, border: "1px dashed #6366F1", borderRadius: 3, background: "transparent", color: "#6366F1", cursor: "pointer" }}
          >
            + add param
          </button>
        </div>
      )}

      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#6366F1" }}
      />
    </div>
  );
}
```

- [ ] **Step 2: Wire onParamsChange in FlowCanvas**

In `poiesis/src/components/canvas/code/FlowCanvas.tsx`, when creating fn node data, add the `onParamsChange` callback:

Find the `handleAddNode` function (around line 90), update the fn case:

```typescript
if (tag === "fn") {
  props.name = "newFn";
  props.params = "";
}
```

Also, in the RF node creation (where `elements` are mapped to ReactFlow nodes), pass `onParamsChange`:

```typescript
// Find where FnNode data is constructed and add:
data: {
  ...el.props,
  fnName: el.props.name as string,
  params: el.props.params as string,
  onParamsChange: (newParams: string) => {
    setProp(el.id, "params", newParams);
  },
  onDoubleClick: () => { /* existing handler */ },
}
```

- [ ] **Step 3: Build check**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: no new errors

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/nodes/FnNode.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx
git commit -m "feat: add inline params editor to FnNode via +params button"
```

---

## Plan Summary

| Task | Files | Key Deliverable |
|------|-------|-----------------|
| 1 | `store.ts` (new), `index.ts` | Store type definitions |
| 2 | `store-crud.ts` (new), `types.ts`, `index.ts` | Zustand CRUD slice + @store references |
| 3 | `tab-helpers.ts` | `TabDataEntry.selectedActionPid` |
| 4 | `parser.ts`, `types.ts`, `parser-types.ts` | `parseStoreBody()` actual parser |
| 5 | `source-emitter.ts`, `pipeline.ts` | `emitStore()` with @pid + CodeFlow |
| 6 | `StorePage.tsx` (new), `StoreMembersPanel.tsx` (new), `DesignPage.tsx` | Store UI + action editing |
| 7 | `store-roundtrip.test.ts` (new) | Round-trip verification |
| 8 | `FnNode.tsx`, `FlowCanvas.tsx` | FN node params editor |

## Follow-up (not in this plan)

| Item | Reason |
|------|--------|
| Store-to-store validation (`NotificationStore.notify(...)` check) | Requires cross-file parsing infrastructure; data model (`storeReferences`) is in place |
| Inline store editor | Spec deferred to separate design |
| Store rename → update all `@store` references | Requires emitter integration for all referencing components |
| Getter Monaco syntax highlighting for OmniLang closures | Monaco tokenizer work, independent task |

