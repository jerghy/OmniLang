# Integ 2 Report: Real .omni Scanner for Bubble Graph

## Status: COMPLETE

## Summary

Replaced the hardcoded demo data in `services/architecture/bubble-data.ts` with a real scanner that reads .omni files and builds the bubble graph.

## Implementation

**File:** `poiesis/src/services/architecture/bubble-data.ts`

### Algorithm

1. **Phase 1 - Parse all .omni files:**
   - Extract component names via regex (`component X {` at line start)
   - Extract store names via regex (`store X {` at line start)
   - Extract lib/pub fn names via regex (`pub fn x` at line start)
   - For each component file, scan the view block (using brace-counting) to find:
     - PascalCase tags (custom component references) for child composition
     - Lowercase tags (HTML elements) for `isLeaf` detection
     - `@store` references for store-read edges

2. **Phase 2 - Build bubble graph:**
   - 3 fixed top-level bubbles: 界面, Store, 代码 (depth 0)
   - Entry component from `main.omni` at depth 1 under 界面
   - Recursive component tree: PascalCase tags in view → child components
   - `isLeaf` = true if component has NO PascalCase child components
   - Store bubbles with signal/getter/action children
   - Lib bubbles with pub fn children

### Key design decisions

- **Source scanning over parser:** The OmniLang parser (`parseOmniSource`) crashes in standalone mode because it depends on the component registry (`useComponentRegistry`). Source scanning is more reliable for this use case.
- **Brace-counting for view blocks:** The `format...}{...}{...}\` regex approach doesn't handle nested `@if`/`@for` blocks. A proper brace-counting algorithm extracts the complete view block.
- **Case-sensitive HTML tag detection:** `isBasicHtmlTag` now only matches lowercase tags (e.g., `<header>` is HTML, `<Header>` is a custom component). This fixes the `Header` vs `header` ambiguity.
- **Multi-component file support:** `extractComponentNames()` returns ALL component names from a file (some files define child components alongside the main one).

### Test Results (learn-omniview, 85 .omni files)

| Metric | Value |
|--------|-------|
| Total bubbles | 225 |
| Total edges | 65 |
| Depth 1 components | 1 (App) |
| Depth 2 components | 51 |
| Stores | 3 (NotificationStore, ThemeStore, TodoStore) |
| Lib files | 20 |

**Hierarchy matches expected structure:**
```
界面 → App → Header [leaf], Dashboard [→ children], TodoPage [→ FilterBar, TodoList → AddTodo, TodoItem], ...
Store → NotificationStore, ThemeStore, TodoStore [→ items, filter, addTask, ...]
代码 → array_ext, datetime_demo, ... (20 lib files with pub fn)
```

### API

The function signature:
```typescript
export async function buildBubbleGraph(
  projectDir: string,
  omniFiles?: OmniFileInfo[]  // optional - falls back to demo data
): Promise<{ bubbles: BubbleNode[]; edges: BubbleEdge[] }>
```

- `OmniFileInfo`: `{ path: string; source: string }`
- When `omniFiles` is omitted/empty, returns the original hardcoded demo data (backward compatible)

## TypeScript Check

`npx tsc --noEmit` passes with **0 errors**.

## Concerns

1. **Parser dependency removed:** The integration with `parseOmniSource` was dropped because it crashes on PascalCase tags in most component files (due to missing component registry). The source-scanning approach is robust but doesn't leverage `parseOmniSource`'s structured output. A future improvement could pre-populate the component registry from source-scan results.
2. **No store-read/store-write edges:** The current implementation only creates composition edges. Store-read edges (component → store signal) and store-write edges (component → store action) require deeper analysis of view bindings and event handlers. These are placeholder for a future iteration.
3. **Positioning:** Bubble positions use simple arithmetic (stacked rows by depth). The `useBubbleLayout` hook in ArchitectureGraph.tsx handles dynamic positioning.
