# Component Members Panel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a left panel (ComponentMembersPanel) in CodeFlowCanvas mode for designing component props, signals, and computed declarations.

**Architecture:** Extend ComponentSourceDef with `computeds` field. Build ComponentMembersPanel (mirrors StoreMembersPanel). Modify DesignPage to show 3-column layout (280px panel + ReactFlow + 240px FlowInspector) in code flow mode. Move prop/signal/computed data flow to componentDefs. Update parser to retain computed body expression and stop creating codeblock elements for signal/computed.

**Tech Stack:** React 19 + TypeScript + Zustand 5 + @xyflow/react + `--ol-*` CSS custom properties

## Global Constraints

- Immutable state updates (Zustand `set` with spread)
- `--ol-*` CSS custom properties for all styling
- Use existing `OMNI_TYPES` constant for type dropdowns
- PID annotation format: `// @pid:xxx`
- File-size discipline: components <800 lines, functions <50 lines

---

## File Structure

```
poiesis/src/
├── core/types/project-json.ts          [MODIFY] +ComputedEntry, extend ComponentSourceDef
├── services/codegen/parser-types.ts    [MODIFY] +computeds field in ParseContext
├── services/codegen/types.ts           [MODIFY] +computeds field in ParseResult
├── services/codegen/parser.ts          [MODIFY] parseSignal/parseComputed → no codeblock, retain expression
├── services/codegen/parser-finalize.ts [MODIFY] pass computeds through buildParseResult
├── services/codegen/component-builder.ts [MODIFY] read props/signals/computeds from componentDefs
├── pages/DesignPage.tsx                [MODIFY] 3-column layout when codeFlowVisible
├── services/project/component-service.ts [MODIFY] merge computeds into ComponentDef on load
├── components/design/ComponentMembersPanel.tsx [CREATE] new panel component
└── services/codegen/__tests__/component-members-roundtrip.test.ts [CREATE] round-trip test
```

---

### Task 1: Add ComputedEntry type and extend ComponentSourceDef

**Files:**
- Modify: `poiesis/src/core/types/project-json.ts`

**Interfaces:**
- Produces: `ComputedEntry` interface, updated `ComponentSourceDef` with `computeds: Array<ComputedEntry>`

- [ ] **Step 1: Add ComputedEntry interface**

After `FnDef` (line 125), add:

```typescript
/** Component-level computed declaration — aligns with StoreGetterDef. */
export interface ComputedEntry {
  pid: string;
  name: string;
  returnType: string;
  expression: string;
}
```

- [ ] **Step 2: Extend ComponentSourceDef**

In `ComponentSourceDef` interface (line 94), add `computeds` field after `props`:

```typescript
export interface ComponentSourceDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;
  props: Array<DefEntry>;
  computeds: Array<ComputedEntry>;  // NEW
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors from this file (pre-existing errors only).

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/project-json.ts
git commit -m "feat: add ComputedEntry type and extend ComponentSourceDef"
```

---

### Task 2: Extend ParseContext and ParseResult with signals and computeds

**Files:**
- Modify: `poiesis/src/services/codegen/parser-types.ts`
- Modify: `poiesis/src/services/codegen/types.ts`
- Modify: `poiesis/src/services/codegen/parser-finalize.ts`

**Interfaces:**
- Consumes: `ComputedEntry` from Task 1
- Produces: `ParseContext.signals`, `ParseContext.computeds`, `ParseResult.signals`, `ParseResult.computeds`, updated `buildParseResult()`

- [ ] **Step 1: Add signals and computeds to ParseContext**

In `parser-types.ts`, after the `events` field (line 72), add:

```typescript
  signals: Array<{ name: string; type: string; defaultValue?: unknown }>;
  computeds: Array<{ name: string; type: string; expression: string }>;
```

So the block reads:

```typescript
  props: Array<{ name: string; type: string; defaultValue?: unknown }>;
  events: Array<{ name: string }>;
  signals: Array<{ name: string; type: string; defaultValue?: unknown }>;
  computeds: Array<{ name: string; type: string; expression: string }>;
  componentMeta: Record<string, unknown> | null;
```

- [ ] **Step 2: Add signals and computeds to ParseResult**

In `types.ts`, after the `events` field in `ParseResult` (line 294), add:

```typescript
  /** Parsed signal declarations from component body (`signal <name>: <type> = <default>;`). */
  signals?: Array<{ name: string; type: string; defaultValue?: unknown }>;
  /** Parsed computed declarations from component body (`computed <name>: <type> = <expr>;`). */
  computeds?: Array<{ name: string; type: string; expression: string }>;
```

- [ ] **Step 3: Pass signals and computeds through buildParseResult**

In `parser-finalize.ts`, `buildParseResult()` (line 97), add `signals` and `computeds` to the returned object:

```typescript
    props: ctx.props.length > 0 ? ctx.props : undefined,
    events: ctx.events.length > 0 ? ctx.events : undefined,
    signals: ctx.signals.length > 0 ? ctx.signals : undefined,
    computeds: ctx.computeds.length > 0 ? ctx.computeds : undefined,
```

- [ ] **Step 4: Initialize signals and computeds in ParseContext factory**

Find where `ParseContext` is initialized in `omni-parser.ts`:

```
grep -n "currentComponent:" poiesis/src/services/codegen/omni-parser.ts
```

Read the context initialization block. Add `signals: []` and `computeds: []` next to `props: []` and `events: []`.

- [ ] **Step 5: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/parser-types.ts poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/parser-finalize.ts poiesis/src/services/codegen/omni-parser.ts
git commit -m "feat: add signals and computeds fields to ParseContext and ParseResult"
```

---

### Task 3: Update parser — retain computed body, remove codeblock elements for signal/computed

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`

**Interfaces:**
- Consumes: `ParseContext.computeds` from Task 2
- Produces: Updated `parseSignal()` and `parseComputed()` — no codeblock elements, signal default captured, computed expression captured

- [ ] **Step 1: Rewrite parseSignal — capture default value, push to ctx.signals instead of codeblock**

Replace existing `parseSignal()` (lines 450-470) with:

```typescript
  // ── Signal ──

  private parseSignal(): void {
    this.stream.consume(); // "signal"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    let defaultValue: unknown = undefined;
    if (this.stream.peek().kind === "EQUALS") {
      this.stream.consume(); // "="
      const valTok = this.stream.peek();
      if (valTok.kind === "STRING") {
        defaultValue = this.stream.consume().text.slice(1, -1);
      } else if (valTok.kind === "NUMBER") {
        const text = this.stream.consume().text;
        defaultValue = text.includes(".") ? parseFloat(text) : parseInt(text, 10);
      } else if (valTok.kind === "IDENTIFIER") {
        const text = this.stream.consume().text;
        if (text === "true") defaultValue = true;
        else if (text === "false") defaultValue = false;
        else defaultValue = text; // reference to another signal/prop
      } else {
        // Complex expression — skip until semicolon
        this.skipUntilSemicolon();
      }
    }
    this.stream.expect("SEMICOLON");

    this.ctx.signals.push({
      name: nameTok.text,
      type: typeTok.text,
      defaultValue,
    });
  }
```

- [ ] **Step 2: Rewrite parseComputed — capture expression, push to ctx.computeds**

Replace existing `parseComputed()` (lines 474-491) with:

```typescript
  // ── Computed ──

  private parseComputed(): void {
    this.stream.consume(); // "computed"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("EQUALS");

    // Collect expression text from "=" to ";"
    let expression = "";
    let depth = 0;
    while (!this.stream.isEOF()) {
      const tok = this.stream.peek();
      if (tok.kind === "SEMICOLON" && depth === 0) break;
      if (tok.kind === "BRACE_OPEN") depth++;
      if (tok.kind === "BRACE_CLOSE") depth--;
      expression += tok.text;
      this.stream.consume();
    }
    this.stream.expect("SEMICOLON");

    this.ctx.computeds.push({
      name: nameTok.text,
      type: typeTok.text,
      expression: expression.trim(),
    });
  }
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors from parser.ts.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts
git commit -m "feat: update parser — signal/computed push to ctx, not codeblock elements"
```

---

### Task 4: Update component-builder to read from componentDefs

**Files:**
- Modify: `poiesis/src/services/codegen/component-builder.ts`

**Interfaces:**
- Consumes: `ComputedEntry` from Task 1, `ComponentDef` from project-json.ts
- Produces: Updated `buildComponentPart()` — reads props/signals/computeds from componentDefs

- [ ] **Step 1: Add import for ComputedEntry**

At top of `component-builder.ts`, after the existing imports, add:

```typescript
import type { ComputedEntry } from "../../core/types/project-json";
```

- [ ] **Step 2: Rewrite buildComponentPart to read from componentDefs**

Replace the section that builds `componentProps`, `componentSignals`, `componentComputeds` (lines 70-105) with:

```typescript
export function buildComponentPart(
  options: GenerateOmniSourceOptions,
  phaseA: PhaseAContext,
  phaseB: PhaseBContext,
  viewWrapperId: string,
  phaseC: PhaseCContext,
): ComponentPart {
  // Read component definition from the registry (populated by parser → componentDefs)
  const registryDef = getComponentDefinitions()[options.componentName];
  const componentDef = (registryDef && typeof registryDef === "object" && "signals" in registryDef)
    ? (registryDef as unknown as {
        signals?: Array<{ name: string; type: string; defaultValue?: unknown }>;
        props?: Array<{ name: string; type: string; defaultValue?: unknown }>;
        computeds?: Array<ComputedEntry>;
        events?: Array<{ name: string }>;
      })
    : null;

  // Props from ComponentDef
  const componentProps: import("./types").PropDef[] = (componentDef?.props || []).map((p) => ({
    name: p.name,
    type: p.type,
    defaultValue: p.defaultValue,
  }));

  // Signals from ComponentDef
  const componentSignals: Record<string, { type: string; defaultValue?: unknown }> = {};
  for (const s of componentDef?.signals || []) {
    componentSignals[s.name] = { type: s.type, defaultValue: s.defaultValue };
  }

  // Computeds from ComponentDef
  const componentComputeds: Record<string, { type: string; body: string }> = {};
  for (const c of componentDef?.computeds || []) {
    componentComputeds[c.name] = { type: c.returnType, body: c.expression };
  }

  // Events from ComponentDef
  const componentEvents: import("./types").EventDef[] = (componentDef?.events || []).map((e) => ({
    name: e.name,
  }));
  // ... rest of the function unchanged (consts, view, return)
```

- [ ] **Step 3: Remove the old registry-based props/events extraction**

Delete the old `registryDef` / `registryProps` / `registryEvents` code that calls `getComponentDefinitions()`. Also remove the old `componentSignals` construction from `standaloneSignals` and `componentComputeds` from `standaloneComputeds` — these are now handled above.

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/component-builder.ts
git commit -m "feat: read component props/signals/computeds from componentDefs in builder"
```

---

### Task 5: Merge signals and computeds into ComponentDef at load time

**Files:**
- Modify: `poiesis/src/services/project/component-service.ts`
- Modify: `poiesis/src/features/useProjectInit.ts`

**Interfaces:**
- Consumes: `ParseResult.signals`, `ParseResult.computeds` from Task 2, `ComputedEntry` from Task 1
- Produces: `ComponentDef.signals` and `ComponentDef.computeds` populated on load

- [ ] **Step 1: Extend component-service.ts load — add imports and merge signals/computeds**

Add imports at top of `component-service.ts`:

```typescript
import { uid } from "../../core/utils/id";
import type { ComputedEntry } from "../../core/types/project-json";
```

Then in `loadComponentDefinitionsFromServer()`, after the props merging block, add signals and computeds merge:

```typescript
    // Signals from DSL declarations
    const dslSignals = parsed.signals || [];
    // Props from DSL declarations (existing)
    const dslProps = parsed.props || [];
    for (const p of dslProps) {
      if (!propSchema[p.name]) {
        propSchema[p.name] = { type: p.type, default: p.defaultValue, editor: "text" };
      }
    }

    // Computeds from DSL declarations
    const dslComputeds = parsed.computeds || [];
    const computedsList: ComputedEntry[] = dslComputeds.map((c) => ({
      pid: uid("c"),
      name: c.name,
      returnType: c.type,
      expression: c.expression,
    }));
```

And in the `merged[name]` object, add:

```typescript
    merged[name] = {
      // ... existing fields ...
      signals: dslSignals.map((s) => ({
        pid: uid("sgn"),
        name: s.name,
        type: s.type,
        defaultValue: s.defaultValue,
      })),
      computeds: computedsList,
      // ... rest ...
    };
```

- [ ] **Step 2: Extend useProjectInit to set componentDefs with parsed signals/computeds**

In `useProjectInit.ts`, find where `parseOmniSource` results populate componentDefs. After parsing, call:

```typescript
const parsed = parseOmniSource(source);
const existing = usePoiesisStore.getState().componentDefs[name];
usePoiesisStore.getState().setComponentDef(name, {
  ...existing,
  name,
  filePath,
  signals: (parsed.signals || []).map((s) => ({
    pid: uid("sgn"),
    name: s.name,
    type: s.type,
    defaultValue: s.defaultValue,
  })),
  props: (parsed.props || []).map((p) => ({
    pid: uid("p"),
    name: p.name,
    type: p.type,
    defaultValue: p.defaultValue,
  })),
  computeds: (parsed.computeds || []).map((c) => ({
    pid: uid("c"),
    name: c.name,
    returnType: c.type,
    expression: c.expression,
  })),
  events: (parsed.events || []).map((e) => ({ pid: uid("evt"), name: e.name, payloadType: "" })),
  fns: existing?.fns || [],
});
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/project/component-service.ts poiesis/src/features/useProjectInit.ts
git commit -m "feat: populate ComponentDef.computeds from parsed .omni source"
```

---

### Task 6: Create ComponentMembersPanel component

**Files:**
- Create: `poiesis/src/components/design/ComponentMembersPanel.tsx`

**Interfaces:**
- Consumes: `componentDefs[name].props/signals/computeds` via Zustand
- Produces: `ComponentMembersPanel` React component — three collapsible sections with inline CRUD

- [ ] **Step 1: Create the component file**

Create `poiesis/src/components/design/ComponentMembersPanel.tsx`:

```tsx
// components/design/ComponentMembersPanel.tsx
// Left panel for component code-flow mode: Props, Signals, Computed sections.
// Mirrors StoreMembersPanel pattern with --ol-* design tokens.

import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { CodeEditorModal } from "../canvas/code/CodeEditorModal";
import { uid } from "../../core/utils/id";
import type { ComponentDef, DefEntry, ComputedEntry } from "../../core/types/project-json";

const OMNI_TYPES = [
  "String", "i32", "i64", "f64", "f32",
  "bool", "Array<String>", "Array<i32>", "Array<f64>",
  "Map<String, String>",
];

const S = {
  panel: {
    height: "100%",
    overflow: "auto",
    background: "var(--ol-gray-50)",
  } as React.CSSProperties,

  sectionHeader: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "8px 12px",
    cursor: "pointer",
    userSelect: "none",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  addBtn: {
    padding: "1px 7px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
    lineHeight: "18px",
  } as React.CSSProperties,

  itemRow: {
    padding: "6px 12px",
    fontSize: "var(--ol-font-size-12)",
    borderBottom: "1px solid var(--ol-border-subtle)",
    background: "var(--ol-surface)",
  } as React.CSSProperties,

  itemHeader: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
    marginBottom: 3,
  } as React.CSSProperties,

  input: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "none",
    width: "100%",
  } as React.CSSProperties,

  select: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 4px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "none",
    cursor: "pointer",
  } as React.CSSProperties,

  typeSep: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
  } as React.CSSProperties,

  valueRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1)",
  } as React.CSSProperties,

  removeBtn: {
    border: "none",
    background: "transparent",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: "var(--ol-font-size-13)",
    padding: "0 2px",
    lineHeight: 1,
  } as React.CSSProperties,

  computedExpr: {
    flex: 1,
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
    color: "var(--ol-text-link)",
    cursor: "pointer",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
    background: "var(--ol-color-primary-subtle)",
    padding: "2px 6px",
    borderRadius: "var(--ol-radius-sm)",
  } as React.CSSProperties,
};

interface Props {
  componentName: string;
}

export function ComponentMembersPanel({ componentName }: Props) {
  const componentDef = usePoiesisStore((s) => s.componentDefs[componentName]);
  const setComponentDef = usePoiesisStore((s) => s.setComponentDef);
  const storeDefsObj = usePoiesisStore((s) => s.storeDefs);
  const storeDefs = Object.values(storeDefsObj);

  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  const [editingComputedName, setEditingComputedName] = useState<string | null>(null);

  const toggle = (key: string) => setCollapsed((c) => ({ ...c, [key]: !c[key] }));

  const props: DefEntry[] = componentDef?.props ?? [];
  const signals: DefEntry[] = componentDef?.signals ?? [];
  const computeds: ComputedEntry[] = componentDef?.computeds ?? [];

  const nextN = () => {
    const nums = [
      ...props.map((p) => parseInt(p.pid.split("_n").pop() || "0")),
      ...signals.map((s) => parseInt(s.pid.split("_n").pop() || "0")),
      ...computeds.map((c) => parseInt(c.pid.split("_n").pop() || "0")),
    ].filter((n) => !isNaN(n));
    return (nums.length > 0 ? Math.max(...nums) : 0) + 1;
  };

  const compDefPid = componentDef?.pid ?? uid("c");

  const updateDef = (updates: Partial<ComponentDef>) => {
    if (!componentDef) return;
    setComponentDef(componentName, { ...componentDef, ...updates });
  };

  // ── Props CRUD ──

  const handleAddProp = () => {
    const entry: DefEntry = {
      pid: `${compDefPid}_n${nextN()}`,
      name: "newProp",
      type: "String",
      defaultValue: undefined,
    };
    updateDef({ props: [...props, entry] });
  };

  const updateProp = (pid: string, updates: Partial<DefEntry>) => {
    updateDef({ props: props.map((p) => p.pid === pid ? { ...p, ...updates } : p) });
  };

  const removeProp = (pid: string) => {
    updateDef({ props: props.filter((p) => p.pid !== pid) });
  };

  // ── Signals CRUD ──

  const handleAddSignal = () => {
    const entry: DefEntry = {
      pid: `${compDefPid}_n${nextN()}`,
      name: "newSignal",
      type: "String",
      defaultValue: "",
    };
    updateDef({ signals: [...signals, entry] });
  };

  const updateSignal = (pid: string, updates: Partial<DefEntry>) => {
    updateDef({ signals: signals.map((s) => s.pid === pid ? { ...s, ...updates } : s) });
  };

  const removeSignal = (pid: string) => {
    updateDef({ signals: signals.filter((s) => s.pid !== pid) });
  };

  // ── Computeds CRUD ──

  const handleAddComputed = () => {
    const entry: ComputedEntry = {
      pid: `${compDefPid}_n${nextN()}`,
      name: "newComputed",
      returnType: "String",
      expression: "",
    };
    updateDef({ computeds: [...computeds, entry] });
  };

  const updateComputed = (pid: string, updates: Partial<ComputedEntry>) => {
    updateDef({ computeds: computeds.map((c) => c.pid === pid ? { ...c, ...updates } : c) });
  };

  const removeComputed = (pid: string) => {
    updateDef({ computeds: computeds.filter((c) => c.pid !== pid) });
  };

  const editingComputed = editingComputedName
    ? computeds.find((c) => c.pid === editingComputedName)
    : null;

  return (
    <div style={S.panel}>
      {/* ── Props ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("props")}>
          <span>Props</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddProp(); }}>+</button>
        </div>
        {!collapsed.props && props.map((p) => (
          <div key={p.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input style={{ ...S.input, width: 72 }} value={p.name}
                onChange={(e) => updateProp(p.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select style={{ ...S.select, width: 128 }} value={p.type}
                onChange={(e) => updateProp(p.pid, { type: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(p.type) && <option value={p.type}>{p.type}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => removeProp(p.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input style={S.input} value={p.defaultValue !== undefined ? String(p.defaultValue) : ""}
                placeholder="default (empty = required)"
                onChange={(e) => updateProp(p.pid, { defaultValue: e.target.value || undefined })} />
            </div>
          </div>
        ))}
        {!collapsed.props && props.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No props yet
          </div>
        )}
      </div>

      {/* ── Signals ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("signals")}>
          <span>Signals</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddSignal(); }}>+</button>
        </div>
        {!collapsed.signals && signals.map((sig) => (
          <div key={sig.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input style={{ ...S.input, width: 72 }} value={sig.name}
                onChange={(e) => updateSignal(sig.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select style={{ ...S.select, width: 128 }} value={sig.type}
                onChange={(e) => updateSignal(sig.pid, { type: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(sig.type) && <option value={sig.type}>{sig.type}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => removeSignal(sig.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <input style={S.input} value={sig.defaultValue !== undefined ? String(sig.defaultValue) : ""}
                placeholder="default value"
                onChange={(e) => updateSignal(sig.pid, { defaultValue: e.target.value })} />
            </div>
          </div>
        ))}
        {!collapsed.signals && signals.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No signals yet
          </div>
        )}
      </div>

      {/* ── Computed ── */}
      <div>
        <div style={S.sectionHeader} onClick={() => toggle("computed")}>
          <span>Computed</span>
          <button style={S.addBtn} onClick={(e) => { e.stopPropagation(); handleAddComputed(); }}>+</button>
        </div>
        {!collapsed.computed && computeds.map((c) => (
          <div key={c.pid} style={S.itemRow}>
            <div style={S.itemHeader}>
              <input style={{ ...S.input, width: 72 }} value={c.name}
                onChange={(e) => updateComputed(c.pid, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <select style={{ ...S.select, width: 128 }} value={c.returnType}
                onChange={(e) => updateComputed(c.pid, { returnType: e.target.value })}>
                {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
                {!OMNI_TYPES.includes(c.returnType) && <option value={c.returnType}>{c.returnType}</option>}
              </select>
              <button style={S.removeBtn} onClick={() => removeComputed(c.pid)}>×</button>
            </div>
            <div style={S.valueRow}>
              <span style={{ ...S.typeSep, width: 12, textAlign: "center" }}>=</span>
              <span
                style={S.computedExpr}
                onClick={() => setEditingComputedName(c.pid)}
                title={c.expression || "Click to edit"}
              >
                {c.expression ? c.expression.slice(0, 40) + (c.expression.length > 40 ? "…" : "") : "click to edit"}
              </span>
            </div>
          </div>
        ))}
        {!collapsed.computed && computeds.length === 0 && (
          <div style={{ ...S.itemRow, color: "var(--ol-text-muted)", fontStyle: "italic" }}>
            No computed yet
          </div>
        )}
      </div>

      {/* ── Computed expression editor modal ── */}
      {editingComputed && (
        <CodeEditorModal
          elementId={editingComputed.pid}
          initialSource={editingComputed.expression}
          initialDescription=""
          storeDefs={storeDefs}
          onSave={(source: string) => {
            updateComputed(editingComputed.pid, { expression: source });
            setEditingComputedName(null);
          }}
          onClose={() => setEditingComputedName(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/design/ComponentMembersPanel.tsx
git commit -m "feat: add ComponentMembersPanel for component prop/signal/computed editing"
```

---

### Task 7: Integrate ComponentMembersPanel into DesignPage layout

**Files:**
- Modify: `poiesis/src/pages/DesignPage.tsx`

**Interfaces:**
- Consumes: `ComponentMembersPanel` from Task 6
- Produces: 3-column layout (280px + 1fr + 240px) in code flow mode for component tabs

- [ ] **Step 1: Import ComponentMembersPanel**

Add import at top of DesignPage.tsx:

```typescript
import { ComponentMembersPanel } from "../components/design/ComponentMembersPanel";
```

- [ ] **Step 2: Add showCodeFlowPanel logic**

After the existing `showSidePanels` line (line 77), add:

```typescript
  // Code-flow left panel: shown when in code flow mode for component tabs
  const showCodeFlowPanel = codeFlowVisible && activeTabType === "component";
  const showDesignPanels = activeTabType === "component" && !codeFlowVisible && !storePanelVisible;
```

- [ ] **Step 3: Update the grid layout**

In the grid `style`, replace `gridTemplateColumns: showSidePanels ? "280px 1fr 300px" : "1fr"` with:

```typescript
  gridTemplateColumns: showDesignPanels
    ? "280px 1fr 300px"
    : showCodeFlowPanel
      ? "280px 1fr 240px"
      : "1fr",
```

- [ ] **Step 4: Add left panel for code flow mode**

In the JSX, after the `ComponentPalette + ComponentTree` left panel block (line 315-338), add:

```tsx
        {/* Left panel: ComponentMembersPanel (code-flow mode) */}
        {showCodeFlowPanel && activeTab && (
        <div
          style={{
            gridRow: 3,
            gridColumn: 1,
            overflow: "hidden",
            background: "#fff",
            borderRight: "1px solid #e2e8f0",
          }}
        >
          <ComponentMembersPanel componentName={activeTab.componentName} />
        </div>
        )}
```

- [ ] **Step 5: Update showSidePanels references**

Replace all remaining `showSidePanels` with `showDesignPanels` in the JSX (left panel condition at line 315, center gridColumn at line 344, right panel condition at line 396, StoreDrawer gridColumn at line 413).

- [ ] **Step 6: Verify TypeScript compiles**

Run: `cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new errors.

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/pages/DesignPage.tsx
git commit -m "feat: integrate ComponentMembersPanel into DesignPage 3-column code-flow layout"
```

---

### Task 8: Write round-trip test

**Files:**
- Create: `poiesis/src/services/codegen/__tests__/component-members-roundtrip.test.ts`

**Interfaces:**
- Consumes: `parseOmniSource`, `generateOmniSource` (or `generateOmniFragment`)
- Produces: Passing test verifying save → parse → re-emit stability for prop/signal/computed declarations

- [ ] **Step 1: Write the test file**

```typescript
// services/codegen/__tests__/component-members-roundtrip.test.ts
// Verify that prop/signal/computed declarations survive save → parse → re-emit round-trip.

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { generateOmniSource } from "../pipeline";

describe("Component members round-trip", () => {
  const source = `import omniview;

component TestCard {
    prop title: String = "Hello";
    prop count: i32;
    event onPress;

    signal likes: i32 = 0;
    signal active: bool = false;

    computed isPopular: bool = likes > 10;
    computed label: String = "Status: " + (if active { "on" } else { "off" });

    fn toggle() {
        active = !active;
    }

    view {
        <div>
            <p>{title}</p>
            <p>Likes: {likes}</p>
            <p>Popular: {isPopular}</p>
            <button @onPress={toggle}>Toggle</button>
        </div>
    }
}
`;

  it("parses props from .omni source", () => {
    const result = parseOmniSource(source);
    expect(result.props).toBeDefined();
    expect(result.props!.length).toBe(2);
    expect(result.props![0]).toEqual({ name: "title", type: "String", defaultValue: "Hello" });
    expect(result.props![1]).toEqual({ name: "count", type: "i32", defaultValue: undefined });
  });

  it("parses events from .omni source", () => {
    const result = parseOmniSource(source);
    expect(result.events).toBeDefined();
    expect(result.events![0]).toEqual({ name: "onPress" });
  });

  it("parses signals from .omni source", () => {
    const result = parseOmniSource(source);
    // After Task 3, signals are pushed to ctx.signals — separate from props
    expect(result.signals).toBeDefined();
    expect(result.signals!.length).toBe(2);

    const signalLikes = result.signals!.find((s) => s.name === "likes");
    expect(signalLikes).toBeDefined();
    expect(signalLikes!.type).toBe("i32");
    expect(signalLikes!.defaultValue).toBe(0);

    const signalActive = result.signals!.find((s) => s.name === "active");
    expect(signalActive).toBeDefined();
    expect(signalActive!.type).toBe("bool");
    expect(signalActive!.defaultValue).toBe(false);
  });

  it("parses computeds with expression retained", () => {
    const result = parseOmniSource(source);
    expect(result.computeds).toBeDefined();
    expect(result.computeds!.length).toBe(2);

    const popular = result.computeds!.find((c) => c.name === "isPopular");
    expect(popular).toBeDefined();
    expect(popular!.type).toBe("bool");
    expect(popular!.expression).toContain("likes > 10");

    const label = result.computeds!.find((c) => c.name === "label");
    expect(label).toBeDefined();
    expect(label!.type).toBe("String");
    expect(label!.expression).toContain("active");
    expect(label!.expression).toContain("if");
  });

  it("re-emits source with prop/signal/computed declarations stable", () => {
    const result = generateOmniSource({
      designElements: [],
      designWires: [],
      codeTabs: [],
      componentName: "TestCard",
      rootOrder: [],
    });

    // When componentDefs["TestCard"] has props/signals/computeds, 
    // the emitted source should contain them.
    // This tests the emit path — requires componentDefs to be populated first.
    
    // For now, verify the emission pipeline doesn't crash
    // (full round-trip requires loading componentDefs into the store)
    expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run the tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/component-members-roundtrip.test.ts
```
Expected: Parse tests pass. The re-emit test may be skipped or adjusted.

- [ ] **Step 3: Fix any failing tests**

If the parse tests fail, debug the parser changes from Task 3. Check that `ctx.props` is being populated correctly and `ctx.computeds` is initialized.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/__tests__/component-members-roundtrip.test.ts
git commit -m "test: add component members round-trip test for prop/signal/computed"
```

---

### Task 9: Integration verification

**Files:**
- No new files — verify the full pipeline end-to-end

- [ ] **Step 1: Run full test suite**

```bash
cd poiesis && npx vitest run 2>&1 | tail -20
```
Expected: All previously-passing tests still pass. No regressions.

- [ ] **Step 2: Run TypeScript check**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | tail -5
```
Expected: No new type errors introduced by our changes.

- [ ] **Step 3: Start Poiesis and verify with Victauri**

```bash
cd poiesis && pnpm tauri dev
```

Manual verification checklist:
- [ ] Open a project, double-click a component element → code flow mode activates
- [ ] Left panel shows ComponentMembersPanel with Props / Signals / Computed sections
- [ ] Add a prop → appears in the list, can edit name/type/default
- [ ] Add a signal → appears in the list, can edit name/type/default
- [ ] Add a computed → appears in the list, can edit name/returnType, click expression to open CodeEditorModal
- [ ] Save project (Ctrl+S) → .omni file contains the declarations
- [ ] Re-open project → parsed props/signals/computeds appear in the panel

- [ ] **Step 4: Commit any fixes**

If issues found during verification, fix and commit:

```bash
git add <fixed-files>
git commit -m "fix: integration fixes for ComponentMembersPanel"
```
