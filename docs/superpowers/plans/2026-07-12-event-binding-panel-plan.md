# Event Binding Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix event parsing/emission round-trip in the Poiesis pipeline, then build a visual event binding panel with function picker and inline expression editor.

**Architecture:** Phase 0 fixes two bugs in the parse→emit pipeline for events (parser only handles single identifiers, emitter drops `on` prefix). Phase 1 builds a new EventsSection with compact rows, a HandlerModal for choosing between binding a function reference or writing an inline closure via Monaco. Events are stored in `element.props` with `on*` keys and string values.

**Tech Stack:** React 19, Zustand 5, TypeScript, Monaco Editor, Vitest

## Global Constraints

- All event names in `element.props` MUST use `onXxx` format (e.g., `onClick`, `onInput`)
- All event names in `.omni` source MUST use `@onXxx` format (e.g., `@onClick={handler}`)
- The round-trip `@onClick={handler}` → parse → `props.onClick = "handler"` → emit → `@onClick={handler}` MUST be stable
- Handler values are raw expression strings (function ref, call expression, or inline closure)
- UI components follow existing patterns in `poiesis/src/components/panels/inspector/`
- No new npm dependencies

---

### Task 0: Fix parser — arbitrary expression handler + `on` prefix normalization

**Files:**
- Modify: `poiesis/src/services/codegen/parser-view.ts:293-301`

**Interfaces:**
- Consumes: `TokenStream` from omni-tokenizer.ts, `ViewElementAttrs` from parser-types.ts
- Produces: `result.events[normalizedEventName] = rawExpression`

- [ ] **Step 1: Replace the single-identifier event parser with a brace-depth-aware expression parser**

Replace lines 293-301 in `parser-view.ts`:

```typescript
    // Event binding: @name={expression}
    if (tok.kind === "AT") {
      internals.stream.consume(); // AT (@)
      const rawEventName = internals.stream.expect("IDENTIFIER").text;
      internals.stream.expect("EQUALS");
      internals.stream.expect("BRACE_OPEN");

      // Collect all tokens between { and } as a raw expression string,
      // using brace-depth counting to handle nested braces (closures, objects, etc.)
      let depth = 1;
      let handlerExpr = "";
      while (depth > 0 && !internals.stream.isEof()) {
        const t = internals.stream.peek();
        if (t.kind === "BRACE_OPEN") depth++;
        else if (t.kind === "BRACE_CLOSE") depth--;
        if (depth === 0) {
          internals.stream.consume(); // consume closing }
          break;
        }
        internals.stream.consume();
        handlerExpr += t.text;
      }
      handlerExpr = handlerExpr.trim();

      // Normalize event name: ensure it has "on" prefix
      // @click → onClick, @onClick → onClick
      const normalizedEventName = rawEventName.startsWith("on")
        ? rawEventName
        : "on" + rawEventName.charAt(0).toUpperCase() + rawEventName.slice(1);

      result.events[normalizedEventName] = handlerExpr;
      continue;
    }
```

- [ ] **Step 2: Run existing round-trip tests to verify no regression**

Run: `cd /d/space/OmniLang/poiesis && npx vitest run src/services/codegen/__tests__/component-members-roundtrip.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Verify the surviving event test cases still work**

The test at line 36 uses `@onPress={toggle}` — a single identifier handler. After the fix, this should still parse correctly as `props.onPress = "toggle"`.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/parser-view.ts
git commit -m "fix(parser): support arbitrary expressions in @event handlers, normalize on* prefix"
```

---

### Task 1: Fix emitter — preserve `on` prefix in event emission

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts:647-653`

**Interfaces:**
- Consumes: `elNode.props` with `on*` keys
- Produces: `@onClick={handler}` syntax in emitted .omni source

- [ ] **Step 1: Replace the fallback event emission to preserve `on` prefix**

Replace lines 647-653 in `source-emitter.ts`:

```typescript
    } else {
      for (const [key, val] of Object.entries(elNode.props)) {
        if (key.startsWith("on") && typeof val === "string" && val.length > 0) {
          parts.push(` @${key}={${val}}`);
        }
      }
    }
```

The key change: emit `@${key}` instead of `@${evtName}` (where `evtName` stripped the `on` prefix).

Also add `val.length > 0` guard — empty handler strings should not be emitted.

- [ ] **Step 2: Run existing round-trip tests**

Run: `cd /d/space/OmniLang/poiesis && npx vitest run src/services/codegen/__tests__/component-members-roundtrip.test.ts`
Expected: All tests PASS

- [ ] **Step 3: Manually verify round-trip logic**

After both parser and emitter fixes:
- Source: `@onPress={toggle}` → parser → `props.onPress = "toggle"` → emitter → `@onPress={toggle}` ✓
- Source: `@onClick={() => { count = count + 1; }}` → parser → `props.onClick = "() => { count = count + 1; }"` → emitter → `@onClick={() => { count = count + 1; }}` ✓

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts
git commit -m "fix(emitter): preserve on* prefix in event emission for round-trip stability"
```

---

### Task 2: Create `useHandlerItems` hook

**Files:**
- Create: `poiesis/src/components/panels/inspector/useHandlerItems.ts`

**Interfaces:**
- Consumes: `usePoiesisStore` — `componentDefs`, `storeDefs`, `storeReferences`, `designPage.activeTabId`, `designPage.openTabs`
- Produces: `HandlerItem[]` where each item has `{ pid, name, displayName, params, source, category }`

```typescript
export interface HandlerItem {
  pid: string;
  name: string;           // function name only, e.g. "handleClick"
  displayName: string;    // display with params, e.g. "handleClick(e: InputEvent)"
  params: Array<{ name: string; type: string }>;
  source: string;         // "component" | "store"
  sourceName: string;     // component name or store name
  category: "fn" | "action";
}
```

- [ ] **Step 1: Write the hook file**

```typescript
// poiesis/src/components/panels/inspector/useHandlerItems.ts
// Collects available functions and store actions for the active component
// to populate the event handler picker.

import { useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { FnDef } from "../../../core/types/project-json";
import type { StoreActionDef } from "../../../core/types/store";

export interface HandlerItem {
  pid: string;
  name: string;
  displayName: string;
  params: Array<{ name: string; type: string }>;
  source: "component" | "store";
  sourceName: string;
  category: "fn" | "action";
}

export function useHandlerItems(): HandlerItem[] {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);

  return useMemo(() => {
    const activeTab = activeTabId
      ? openTabs.find((t) => t.id === activeTabId)
      : undefined;
    const compName = activeTab?.componentName;
    if (!compName) return [];

    const items: HandlerItem[] = [];
    const compDef = componentDefs[compName];

    // Component functions
    if (compDef?.fns) {
      for (const fn of compDef.fns) {
        items.push(formatFnItem(fn, compName, "component"));
      }
    }

    // Store actions from referenced stores
    const refs = storeReferences[compName] ?? [];
    for (const storeName of refs) {
      const storeDef = storeDefs[storeName];
      if (storeDef?.actions) {
        for (const action of storeDef.actions) {
          items.push(formatActionItem(action, storeName));
        }
      }
    }

    return items;
  }, [activeTabId, openTabs, componentDefs, storeDefs, storeReferences]);
}

function formatFnItem(fn: FnDef, compName: string, source: "component" | "store"): HandlerItem {
  const paramsStr = fn.params.map((p) => `${p.name}: ${p.type}`).join(", ");
  return {
    pid: fn.pid,
    name: fn.name,
    displayName: `${fn.name}(${paramsStr})`,
    params: fn.params,
    source,
    sourceName: compName,
    category: "fn",
  };
}

function formatActionItem(action: StoreActionDef, storeName: string): HandlerItem {
  const paramsStr = action.params.map((p) => `${p.name}: ${p.type}`).join(", ");
  return {
    pid: action.pid,
    name: action.name,
    displayName: `${storeName}.${action.name}(${paramsStr})`,
    params: action.params,
    source: "store",
    sourceName: storeName,
    category: "action",
  };
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep -E "useHandlerItems|HandlerItem" || echo "No errors in new file"`
Expected: No errors in the new hook file

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/inspector/useHandlerItems.ts
git commit -m "feat: add useHandlerItems hook for event handler picker"
```

---

### Task 3: Create `ExpressionEditor` component

**Files:**
- Create: `poiesis/src/components/panels/inspector/ExpressionEditor.tsx`

**Interfaces:**
- Consumes: `initialValue: string`, `onChange: (value: string) => void`
- Produces: Renders a Monaco editor for OmniLang inline closure editing

This component wraps Monaco editor initialization logic (extracted pattern from `CodeEditorModal.tsx`). It's a lightweight embed, NOT a full modal.

- [ ] **Step 1: Write the ExpressionEditor component**

```typescript
// poiesis/src/components/panels/inspector/ExpressionEditor.tsx
// Lightweight Monaco editor embed for editing inline event handler expressions.
// Shares Monaco initialization pattern with CodeEditorModal but is NOT a modal.

import { useEffect, useRef, useCallback } from "react";
import * as monaco from "monaco-editor";

interface ExpressionEditorProps {
  initialValue: string;
  onChange: (value: string) => void;
  height?: number;
}

export function ExpressionEditor({
  initialValue,
  onChange,
  height = 260,
}: ExpressionEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const ignoreChangeRef = useRef(false);

  // Initialize Monaco editor
  useEffect(() => {
    if (!containerRef.current) return;

    const editor = monaco.editor.create(containerRef.current, {
      value: initialValue,
      language: "omnilang",
      theme: "vs-dark",
      fontSize: 13,
      minimap: { enabled: false },
      lineNumbers: "on",
      scrollBeyondLastLine: false,
      automaticLayout: true,
    });
    editorRef.current = editor;

    editor.onDidChangeModelContent(() => {
      if (!ignoreChangeRef.current) {
        onChange(editor.getValue());
      }
    });

    return () => {
      editor.dispose();
      editorRef.current = null;
    };
    // Only initialize once (empty deps — controlled via setValue)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sync external value changes into the editor
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) return;
    const currentValue = editor.getValue();
    if (initialValue !== currentValue) {
      ignoreChangeRef.current = true;
      editor.setValue(initialValue);
      ignoreChangeRef.current = false;
    }
  }, [initialValue]);

  return (
    <div
      ref={containerRef}
      style={{
        width: "100%",
        height,
        border: "1px solid #d1d5db",
        borderRadius: 6,
        overflow: "hidden",
      }}
    />
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep -E "ExpressionEditor" || echo "No errors"`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/inspector/ExpressionEditor.tsx
git commit -m "feat: add ExpressionEditor component with Monaco embed"
```

---

### Task 4: Create `FunctionPicker` component

**Files:**
- Create: `poiesis/src/components/panels/inspector/FunctionPicker.tsx`

**Interfaces:**
- Consumes: `items: HandlerItem[]`, `selectedName: string | null`, `onSelect: (item: HandlerItem) => void`, `onClose: () => void`
- Produces: Renders a searchable dropdown list of functions/actions

- [ ] **Step 1: Write the FunctionPicker component**

```typescript
// poiesis/src/components/panels/inspector/FunctionPicker.tsx
// Searchable function/action picker dropdown for event handler binding.

import { useState, useMemo, useCallback } from "react";
import type { HandlerItem } from "./useHandlerItems";

interface FunctionPickerProps {
  items: HandlerItem[];
  selectedName: string | null;
  onSelect: (item: HandlerItem) => void;
  onClose: () => void;
}

export function FunctionPicker({
  items,
  selectedName,
  onSelect,
  onClose,
}: FunctionPickerProps) {
  const [search, setSearch] = useState("");

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const q = search.toLowerCase();
    return items.filter(
      (item) =>
        item.name.toLowerCase().includes(q) ||
        item.displayName.toLowerCase().includes(q)
    );
  }, [items, search]);

  const componentFns = filtered.filter((item) => item.source === "component");
  const storeActions = filtered.filter((item) => item.source === "store");

  const handleSelect = useCallback(
    (item: HandlerItem) => {
      onSelect(item);
    },
    [onSelect]
  );

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 10000,
        }}
      />
      {/* Dropdown panel */}
      <div
        style={{
          position: "absolute",
          top: "100%",
          left: 0,
          right: 0,
          zIndex: 10001,
          marginTop: 4,
          background: "#fff",
          border: "1px solid #e2e8f0",
          borderRadius: 8,
          boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
          maxHeight: 280,
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Search */}
        <div style={{ padding: "8px 10px", borderBottom: "1px solid #f1f5f9" }}>
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="搜索函数/Action..."
            autoFocus
            style={{
              width: "100%",
              padding: "6px 10px",
              fontSize: 12,
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              outline: "none",
              fontFamily: "inherit",
              boxSizing: "border-box",
            }}
            onFocus={(e) => (e.target.style.borderColor = "#6366F1")}
            onBlur={(e) => (e.target.style.borderColor = "#e2e8f0")}
          />
        </div>

        {/* List */}
        <div style={{ overflowY: "auto", flex: 1 }}>
          {componentFns.length > 0 && (
            <>
              <GroupLabel>组件函数</GroupLabel>
              {componentFns.map((item) => (
                <PickerRow
                  key={item.pid}
                  item={item}
                  isSelected={item.name === selectedName}
                  onSelect={handleSelect}
                />
              ))}
            </>
          )}
          {storeActions.length > 0 && (
            <>
              <GroupLabel>Store Actions</GroupLabel>
              {storeActions.map((item) => (
                <PickerRow
                  key={item.pid}
                  item={item}
                  isSelected={item.name === selectedName}
                  onSelect={handleSelect}
                />
              ))}
            </>
          )}
          {filtered.length === 0 && (
            <div
              style={{
                padding: 16,
                textAlign: "center",
                fontSize: 12,
                color: "#94a3b8",
              }}
            >
              无匹配结果
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function GroupLabel({ children }: { children: React.ReactNode }) {
  return (
    <div
      style={{
        padding: "4px 12px",
        fontSize: 10,
        fontWeight: 600,
        color: "#94a3b8",
        textTransform: "uppercase",
        background: "#f8fafc",
        borderTop: "1px solid #f1f5f9",
      }}
    >
      {children}
    </div>
  );
}

function PickerRow({
  item,
  isSelected,
  onSelect,
}: {
  item: HandlerItem;
  isSelected: boolean;
  onSelect: (item: HandlerItem) => void;
}) {
  return (
    <button
      onClick={() => onSelect(item)}
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        width: "100%",
        padding: "6px 12px",
        border: "none",
        background: isSelected ? "#eef2ff" : "transparent",
        cursor: "pointer",
        textAlign: "left",
        fontFamily: "var(--dp-font-mono, monospace)",
        fontSize: 12,
        color: isSelected ? "#4f46e5" : "#1e293b",
      }}
    >
      <span style={{ fontWeight: isSelected ? 600 : 400 }}>
        {item.displayName}
      </span>
      <span style={{ fontSize: 10, color: "#94a3b8" }}>
        {item.sourceName}
      </span>
    </button>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep -E "FunctionPicker" || echo "No errors"`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/inspector/FunctionPicker.tsx
git commit -m "feat: add FunctionPicker component for event handler selection"
```

---

### Task 5: Create `HandlerModal` component

**Files:**
- Create: `poiesis/src/components/panels/inspector/HandlerModal.tsx`

**Interfaces:**
- Consumes: `eventName: string`, `initialHandler: string`, `handlerItems: HandlerItem[]`, `onConfirm: (handlerString: string) => void`, `onClose: () => void`
- Produces: Renders a floating modal with two tabs (绑定函数 / 写表达式) and confirm/cancel buttons

- [ ] **Step 1: Write the HandlerModal component**

```typescript
// poiesis/src/components/panels/inspector/HandlerModal.tsx
// Floating modal for editing an event handler. Two tabs:
// 1. 绑定函数 — select a function/action reference
// 2. 写表达式 — write an inline closure via Monaco editor

import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import { FunctionPicker } from "./FunctionPicker";
import { ExpressionEditor } from "./ExpressionEditor";
import type { HandlerItem } from "./useHandlerItems";

interface HandlerModalProps {
  eventName: string;
  initialHandler: string;
  handlerItems: HandlerItem[];
  onConfirm: (handlerString: string) => void;
  onClose: () => void;
}

type TabMode = "function" | "expression";

/** Detect if a handler string is likely an inline expression (starts with `(` or contains `=>`). */
function looksLikeExpression(handler: string): boolean {
  const trimmed = handler.trim();
  return trimmed.startsWith("(") || trimmed.includes("=>") || trimmed.includes("{");
}

export function HandlerModal({
  eventName,
  initialHandler,
  handlerItems,
  onConfirm,
  onClose,
}: HandlerModalProps) {
  // Determine initial tab
  const initialTab: TabMode =
    !initialHandler || looksLikeExpression(initialHandler) ? "expression" : "function";

  const [tab, setTab] = useState<TabMode>(initialTab);

  // Function picker state
  const [selectedFn, setSelectedFn] = useState<HandlerItem | null>(null);
  const [fnArgs, setFnArgs] = useState<Record<string, string>>({});
  const [pickerOpen, setPickerOpen] = useState(false);

  // Expression editor state
  const [expression, setExpression] = useState(
    initialTab === "expression" && initialHandler
      ? initialHandler
      : "() => {\n  \n}"
  );

  // Parse initialHandler to pre-fill function picker if in function mode
  useEffect(() => {
    if (initialTab === "function" && initialHandler) {
      // Try to match: handlerName or handlerName(args)
      const match = initialHandler.match(/^(\w[\w.]*)\(?(.*?)\)?$/);
      if (match) {
        const fnName = match[1];
        const found = handlerItems.find((item) => {
          // Match by name or by source.name pattern
          return item.name === fnName || item.displayName.startsWith(fnName);
        });
        if (found) {
          setSelectedFn(found);
          // Parse args if present
          if (match[2]) {
            const argParts = match[2].split(",").map((s) => s.trim()).filter(Boolean);
            const args: Record<string, string> = {};
            found.params.forEach((p, i) => {
              if (i < argParts.length) {
                args[p.name] = argParts[i].replace(/^"(.*)"$/, "$1");
              }
            });
            setFnArgs(args);
          }
        }
      }
    }
  }, [initialTab, initialHandler, handlerItems]);

  // Build the final handler string
  const buildHandlerString = useCallback((): string => {
    if (tab === "expression") {
      return expression.trim();
    }
    // Function mode
    if (!selectedFn) return "";
    if (selectedFn.params.length === 0) {
      return selectedFn.source === "store"
        ? `${selectedFn.sourceName}.${selectedFn.name}`
        : selectedFn.name;
    }
    // With args
    const argValues = selectedFn.params.map((p) => {
      const val = fnArgs[p.name] ?? "";
      return val;
    });
    const prefix =
      selectedFn.source === "store"
        ? `${selectedFn.sourceName}.${selectedFn.name}`
        : selectedFn.name;
    return `${prefix}(${argValues.join(", ")})`;
  }, [tab, expression, selectedFn, fnArgs]);

  // Escape key to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleConfirm = useCallback(() => {
    const str = buildHandlerString();
    if (str) onConfirm(str);
  }, [buildHandlerString, onConfirm]);

  const handleFnSelect = useCallback((item: HandlerItem) => {
    setSelectedFn(item);
    setPickerOpen(false);
    // Reset args when selecting a new function
    const args: Record<string, string> = {};
    item.params.forEach((p) => {
      args[p.name] = "";
    });
    setFnArgs(args);
  }, []);

  const handleArgChange = useCallback((paramName: string, value: string) => {
    setFnArgs((prev) => ({ ...prev, [paramName]: value }));
  }, []);

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 9998,
          background: "rgba(0,0,0,0.4)",
        }}
      />
      {/* Modal */}
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          position: "fixed",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          zIndex: 9999,
          background: "#fff",
          borderRadius: 12,
          boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          width: 560,
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Title */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "14px 20px",
            borderBottom: "1px solid #e2e8f0",
          }}
        >
          <span style={{ fontSize: 14, fontWeight: 600, color: "#1e293b" }}>
            编辑 {eventName} 处理器
          </span>
          <button
            onClick={onClose}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 18,
              color: "#94a3b8",
              padding: "4px 8px",
            }}
          >
            ✕
          </button>
        </div>

        {/* Tab switcher */}
        <div
          style={{
            display: "flex",
            gap: 0,
            borderBottom: "1px solid #e2e8f0",
            padding: "0 20px",
          }}
        >
          <TabButton
            active={tab === "function"}
            onClick={() => setTab("function")}
            label="绑定函数"
          />
          <TabButton
            active={tab === "expression"}
            onClick={() => setTab("expression")}
            label="写表达式"
          />
        </div>

        {/* Tab content */}
        <div style={{ padding: 16, overflowY: "auto", flex: 1 }}>
          {tab === "function" && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {/* Function/Action selector */}
              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: 11,
                    fontWeight: 600,
                    color: "#64748b",
                    marginBottom: 4,
                  }}
                >
                  函数/Action
                </label>
                <div style={{ position: "relative" }}>
                  <button
                    onClick={() => setPickerOpen(!pickerOpen)}
                    style={{
                      width: "100%",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      padding: "8px 12px",
                      fontSize: 13,
                      fontFamily: "var(--dp-font-mono, monospace)",
                      border: "1px solid #d1d5db",
                      borderRadius: 6,
                      background: "#fff",
                      cursor: "pointer",
                      textAlign: "left",
                      color: selectedFn ? "#1e293b" : "#94a3b8",
                    }}
                  >
                    <span>
                      {selectedFn ? selectedFn.displayName : "选择函数或 Action..."}
                    </span>
                    <span style={{ color: "#94a3b8" }}>▼</span>
                  </button>
                  {pickerOpen && (
                    <FunctionPicker
                      items={handlerItems}
                      selectedName={selectedFn?.name ?? null}
                      onSelect={handleFnSelect}
                      onClose={() => setPickerOpen(false)}
                    />
                  )}
                </div>
              </div>

              {/* Parameter inputs (shown when selected function has params) */}
              {selectedFn && selectedFn.params.length > 0 && (
                <div>
                  <label
                    style={{
                      display: "block",
                      fontSize: 11,
                      fontWeight: 600,
                      color: "#64748b",
                      marginBottom: 4,
                    }}
                  >
                    参数
                  </label>
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {selectedFn.params.map((p) => (
                      <div
                        key={p.name}
                        style={{ display: "flex", alignItems: "center", gap: 8 }}
                      >
                        <label
                          style={{
                            fontSize: 11,
                            color: "#64748b",
                            minWidth: 40,
                            fontFamily: "var(--dp-font-mono, monospace)",
                          }}
                        >
                          {p.name}
                        </label>
                        <input
                          type="text"
                          value={fnArgs[p.name] ?? ""}
                          onChange={(e) => handleArgChange(p.name, e.target.value)}
                          placeholder={p.type}
                          style={{
                            flex: 1,
                            padding: "6px 10px",
                            fontSize: 12,
                            border: "1px solid #d1d5db",
                            borderRadius: 4,
                            outline: "none",
                            fontFamily: "var(--dp-font-mono, monospace)",
                          }}
                          onFocus={(e) => (e.target.style.borderColor = "#6366F1")}
                          onBlur={(e) => (e.target.style.borderColor = "#d1d5db")}
                        />
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {tab === "expression" && (
            <ExpressionEditor
              initialValue={expression}
              onChange={setExpression}
              height={260}
            />
          )}
        </div>

        {/* Action buttons */}
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
            padding: "12px 20px",
            borderTop: "1px solid #e2e8f0",
          }}
        >
          <button
            onClick={onClose}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              border: "1px solid #d1d5db",
              borderRadius: 6,
              background: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            取消
          </button>
          <button
            onClick={handleConfirm}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              fontWeight: 600,
              border: "none",
              borderRadius: 6,
              background: "#6366F1",
              color: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            确认
          </button>
        </div>
      </div>
    </>
  );
}

function TabButton({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "10px 16px",
        fontSize: 13,
        fontWeight: active ? 600 : 400,
        border: "none",
        borderBottom: active ? "2px solid #6366F1" : "2px solid transparent",
        background: "none",
        cursor: "pointer",
        color: active ? "#6366F1" : "#64748b",
        fontFamily: "inherit",
      }}
    >
      {active ? "● " : "○ "}
      {label}
    </button>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep -E "HandlerModal" || echo "No errors"`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/components/panels/inspector/HandlerModal.tsx
git commit -m "feat: add HandlerModal with function picker and expression editor tabs"
```

---

### Task 6: Rewrite `EventsSection` with compact rows

**Files:**
- Modify: `poiesis/src/components/panels/inspector/EventsSection.tsx`

**Interfaces:**
- Consumes: `element: ElementData`, `handlerItems: HandlerItem[]` (new prop), `getComponentEvents()` from componentProtocol
- Produces: Renders compact event binding rows + triggers HandlerModal

- [ ] **Step 1: Write the rewritten EventsSection**

Replace the entire file content:

```typescript
// components/panels/inspector/EventsSection.tsx
// Event binding panel with compact rows. Each row: event dropdown + handler display + remove.
// Clicking the handler area opens HandlerModal for visual editing.

import { useState, useCallback, useMemo } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import type { EventMeta } from "../../../data/componentProtocol";
import { getComponentEvents } from "../../../data/componentProtocol";
import type { HandlerItem } from "./useHandlerItems";
import { HandlerModal } from "./HandlerModal";

interface EventsSectionProps {
  element: ElementData;
  handlerItems: HandlerItem[];
}

/** Event groups for optgroup rendering in the event dropdown */
const EVENT_GROUPS: Record<string, string[]> = {
  "点击": ["onPress", "onClick", "onDoubleClick", "onContextMenu"],
  "输入": ["onChange", "onInput"],
  "焦点": ["onFocus", "onBlur"],
  "键盘": ["onKeyDown", "onKeyUp"],
  "表单": ["onSubmit"],
  "滚动": ["onScroll"],
  "鼠标": ["onMouseEnter", "onMouseLeave", "onMouseDown", "onMouseUp", "onMouseMove"],
  "拖拽": ["onDragStart", "onDragOver", "onDragEnd", "onDrop"],
  "触摸": ["onTouchStart", "onTouchEnd", "onTouchMove"],
  "滚轮": ["onWheel"],
  "媒体": ["onLoad", "onError", "onPlay", "onPause", "onEnded"],
  "动画": ["onAnimationStart", "onAnimationEnd", "onAnimationIteration", "onTransitionEnd"],
  "切换": ["onToggle"],
  "剪贴板": ["onCopy", "onCut", "onPaste"],
};

export function EventsSection({ element, handlerItems }: EventsSectionProps) {
  const setProp = usePoiesisStore((s) => s.setProp);

  // Which event is being edited in the modal (null = modal closed)
  const [editingEvent, setEditingEvent] = useState<string | null>(null);

  // Get available events from component protocol
  const tag = "tag" in element ? element.tag : "div";
  const protocolEvents = getComponentEvents(tag);

  // Build the list of bound events from element props
  const boundEvents = useMemo(() => {
    if (!("props" in element)) return [];
    return Object.entries(element.props)
      .filter(
        ([key, val]) =>
          key.startsWith("on") &&
          typeof val === "string" &&
          val.trim().length > 0
      )
      .map(([key, val]) => ({
        eventName: key,
        handler: String(val),
        meta: protocolEvents[key] as EventMeta | undefined,
      }));
  }, [element.props, protocolEvents]);

  const boundEventNames = new Set(boundEvents.map((b) => b.eventName));

  // Event options for the "add" dropdown (exclude already bound)
  const availableEvents = useMemo(() => {
    return Object.entries(protocolEvents)
      .filter(([name]) => !boundEventNames.has(name))
      .map(([name, meta]) => ({ name, ...meta }));
  }, [protocolEvents, boundEventNames]);

  // Group available events for optgroup rendering
  const groupedAvailable = useMemo(() => {
    const result: Array<{ group: string; events: typeof availableEvents }> = [];
    for (const [group, eventNames] of Object.entries(EVENT_GROUPS)) {
      const groupEvents = availableEvents.filter((ev) =>
        eventNames.includes(ev.name)
      );
      if (groupEvents.length > 0) {
        result.push({ group, events: groupEvents });
      }
    }
    return result;
  }, [availableEvents]);

  // ── Handlers ──

  const handleAddEvent = useCallback(
    (eventName: string) => {
      setProp(element.id, eventName, "");
    },
    [element.id, setProp]
  );

  const handleRemoveEvent = useCallback(
    (eventName: string) => {
      setProp(element.id, eventName, "");
    },
    [element.id, setProp]
  );

  const handleOpenModal = useCallback((eventName: string) => {
    setEditingEvent(eventName);
  }, []);

  const handleCloseModal = useCallback(() => {
    setEditingEvent(null);
  }, []);

  const handleConfirmHandler = useCallback(
    (handlerString: string) => {
      if (editingEvent) {
        setProp(element.id, editingEvent, handlerString);
      }
      setEditingEvent(null);
    },
    [element.id, editingEvent, setProp]
  );

  const editingHandler = editingEvent
    ? boundEvents.find((b) => b.eventName === editingEvent)?.handler ?? ""
    : "";

  return (
    <div style={{ marginBottom: 12 }}>
      {/* Section header */}
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span>事件</span>
        {availableEvents.length > 0 && (
          <select
            value=""
            onChange={(e) => {
              if (e.target.value) {
                handleAddEvent(e.target.value);
                e.target.value = "";
              }
            }}
            style={{
              fontSize: 10,
              border: "1px solid #e0e7ff",
              borderRadius: 4,
              background: "#eef2ff",
              color: "#4f46e5",
              cursor: "pointer",
              padding: "2px 6px",
              fontFamily: "inherit",
              maxWidth: 120,
            }}
          >
            <option value="">+ 添加事件</option>
            {groupedAvailable.map((g) => (
              <optgroup key={g.group} label={g.group}>
                {g.events.map((ev) => (
                  <option key={ev.name} value={ev.name}>
                    {ev.name}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
        )}
      </div>

      {/* Empty state */}
      {boundEvents.length === 0 && (
        <div style={{ fontSize: 11, color: "#94a3b8" }}>
          暂无事件绑定
        </div>
      )}

      {/* Event rows */}
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {boundEvents.map((ev) => (
          <div
            key={ev.eventName}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            {/* Event name — also a dropdown to change event type */}
            <select
              value={ev.eventName}
              onChange={(e) => {
                const newName = e.target.value;
                if (newName !== ev.eventName) {
                  // Move handler to new event name
                  const handler = ev.handler;
                  setProp(element.id, ev.eventName, "");
                  setProp(element.id, newName, handler);
                }
              }}
              style={{
                fontSize: 10,
                padding: "3px 4px",
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                background: "#fff",
                color: "#374151",
                fontFamily: "var(--dp-font-mono, monospace)",
                minWidth: 90,
                cursor: "pointer",
              }}
            >
              {Object.entries(EVENT_GROUPS).map(([group, names]) => (
                <optgroup key={group} label={group}>
                  {names.map((name) => {
                    const exists =
                      name !== ev.eventName && boundEventNames.has(name);
                    return (
                      <option
                        key={name}
                        value={name}
                        disabled={exists}
                      >
                        {name}
                      </option>
                    );
                  })}
                </optgroup>
              ))}
            </select>

            {/* Handler display — clickable to open modal */}
            <button
              onClick={() => handleOpenModal(ev.eventName)}
              title={ev.meta ? `${ev.meta.description} — ${ev.handler}` : ev.handler}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                padding: "3px 8px",
                fontSize: 11,
                fontFamily: "var(--dp-font-mono, monospace)",
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                background: "#fff",
                cursor: "pointer",
                textAlign: "left",
                color: ev.handler ? "#1e293b" : "#cbd5e1",
                minHeight: 24,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {ev.handler || "点击编辑..."}
            </button>

            {/* Remove */}
            <button
              onClick={() => handleRemoveEvent(ev.eventName)}
              title="移除事件"
              style={{
                border: "none",
                background: "none",
                cursor: "pointer",
                color: "#ef4444",
                fontSize: 14,
                padding: "2px 4px",
                lineHeight: 1,
                fontFamily: "inherit",
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Handler modal */}
      {editingEvent && (
        <HandlerModal
          eventName={editingEvent}
          initialHandler={editingHandler}
          handlerItems={handlerItems}
          onConfirm={handleConfirmHandler}
          onClose={handleCloseModal}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Update ContextInspector to pass handlerItems to EventsSection**

In `ContextInspector.tsx`, add `useHandlerItems` import and hook call, pass to EventsSection:

```typescript
// Add import:
import { useHandlerItems } from "./inspector/useHandlerItems";

// Inside ContextInspector function, add:
const handlerItems = useHandlerItems();

// Change EventsSection line to:
<EventsSection element={el} handlerItems={handlerItems} />
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep -E "EventsSection|HandlerModal|handlerItems" || echo "No errors"`
Expected: No errors related to the new code

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/panels/inspector/EventsSection.tsx poiesis/src/components/panels/ContextInspector.tsx
git commit -m "feat: rewrite EventsSection with compact rows and HandlerModal integration"
```

---

### Task 7: End-to-end verification

**Files:**
- Verify: all modified files work together

- [ ] **Step 1: Full TypeScript check**

Run: `cd /d/space/OmniLang/poiesis && npx tsc --noEmit --pretty 2>&1 | grep "error TS" | grep -v "DesignTabContent\|TokenEditor\|CssStylePanel" | head -20`
Expected: Only pre-existing errors (DesignTabContent, TokenEditor, CssStylePanel), no new errors

- [ ] **Step 2: Run existing tests**

Run: `cd /d/space/OmniLang/poiesis && npx vitest run 2>&1 | tail -15`
Expected: All existing tests PASS

- [ ] **Step 3: Run parser round-trip tests specifically**

Run: `cd /d/space/OmniLang/poiesis && npx vitest run src/services/codegen/__tests__/ 2>&1 | tail -20`
Expected: All parser/round-trip tests PASS

- [ ] **Step 4: Commit verification**

```bash
git status
```
Expected: Clean working tree (or only pre-existing changes unrelated to this plan)

- [ ] **Step 5: Final commit if any cleanup needed**

```bash
git add -A && git diff --cached --stat
```
