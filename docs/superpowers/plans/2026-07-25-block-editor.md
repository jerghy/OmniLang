# Block Editor v4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace Lexical-based CodeCard Editor v3 with a Blockly-based block (积木) editor using keyboard-driven interaction, fuzzy search, and game-controller-style cursor navigation.

**Architecture:** Blockly handles block rendering and connection topology. Custom layers on top handle cursor navigation (SelectionManager), gap row management (GapManager), keyboard input interception (KeyboardHandler), and fuzzy search (FuzzySearchPopup). CodeOp[] remains SSOT.

**Tech Stack:** Blockly v12+, Zelos renderer, React thin wrapper, Vitest + Playwright

---

## Global Constraints

- **No drag-and-drop** — all interaction via keyboard
- **CodeOp[] is SSOT** — Blockly workspace renders from it, never owns data independently
- **SlotValue** — all string fields use `{ kind: "text" | "expr" | "empty" }`
- **Backward compatible** — old CodeOp string fields auto-convert via `fromLegacyString()`
- **`\` key opens raw text input** — text stored as RawCodeOp, never parsed
- **Zelos default colors** — no custom block coloring in v4
- **Direct replacement** — old Lexical code deleted; `@blockly/keyboard-navigation` NOT used (custom navigation)
- **TypeScript strict** — no `any`, no `as` casts without narrowing

---

## Task 1: Install Blockly + Remove Lexical

**Files:**
- Modify: `poiesis/package.json`
- Modify: `poiesis/pnpm-lock.yaml` (auto)

**Produces:** `blockly` available as npm dependency; `lexical` / `@lexical/react` removed.

- [ ] **Step 1: Install Blockly**

```bash
cd poiesis && pnpm add blockly
```

(Note: `@blockly/keyboard-navigation` is NOT needed — we build custom navigation.)

- [ ] **Step 2: Remove Lexical packages**

```bash
cd poiesis && pnpm remove lexical @lexical/react
```

- [ ] **Step 3: Verify install**

```bash
cd poiesis && pnpm ls blockly
```

Expected: blockly listed. `lexical` and `@lexical/react` absent from dependencies.

- [ ] **Step 4: Commit**

```bash
git add poiesis/package.json poiesis/pnpm-lock.yaml
git commit -m "chore: swap lexical for blockly"
```

---

## Task 2: Extend CodeOp Data Model

**Files:**
- Create: `poiesis/src/core/types/slot-value.ts`
- Create: `poiesis/src/core/types/inline-op.ts`
- Modify: `poiesis/src/core/types/code-op.ts`
- Create: `poiesis/src/core/types/__tests__/slot-value.test.ts`
- Create: `poiesis/src/core/types/__tests__/code-op-backward-compat.test.ts`

**Produces:**
- `SlotValue = { kind: "text"; value: string } | { kind: "expr"; children: InlineOp[] } | { kind: "empty" }`
- `InlineOp` union type (7 variants: var_ref, literal, binary, unary, call_fn, member_access, parens)
- Updated `CodeOp` interfaces — all string fields → `SlotValue`; block types gain `bodyId: string`
- `fromLegacyString(s: string | undefined): SlotValue` for backward compat
- `slotToText(sv: SlotValue): string` for flattening to source emission
- `createBodyId(): string` helper

(Same detailed implementation steps as previous plan v1 — code unchanged from here.)

- [ ] **Step 1–9:** Follow the identical steps from v1 plan Task 2 (create slot-value.ts, inline-op.ts, update code-op.ts, write tests, commit).

---

## Task 3: Define Blockly Block Definitions

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/definitions.ts`
- Create: `poiesis/src/components/canvas/code/blocks/mutations.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/definitions.test.ts`

**Produces:**
- `BLOCK_DEFINITIONS: Record<string, BlockDefinition>` — JSON for all 13 block types
- `registerAllBlocks(): void` — registers blocks + if_block mutator with Blockly
- `ifBlockMutator` — Blockly mutator object for dynamic elif/else branches

- [ ] **Step 1: Create block definitions** — same as v1 plan Task 3, plus the `ifBlockMutator`:

```typescript
// poiesis/src/components/canvas/code/blocks/mutations.ts
import * as Blockly from "blockly";

export const IF_BLOCK_MUTATOR: Blockly.MutatorIcon = {
  // If block mutation for managing elif/else branches
  // mutationToDom: serialize elif/else count to XML
  // domToMutation: restore elif/else branches from XML
  // compose: add/remove elif/else inputs based on mutation
  // decompose: create a dummy block flyout for branch management
};
```

(The full mutator implementation follows Blockly's documented mutation pattern with `mutationToDom`/`domToMutation`/`compose`/`decompose` methods.)

- [ ] **Step 2–4:** Follow v1 plan Task 3 steps for tests and commit.

---

## Task 4: Build BlockAdapter (CodeOp ↔ Blockly)

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/BlockAdapter.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/BlockAdapter.test.ts`

**Produces:**
- `class BlockAdapter { toBlocks(ops, ws): Block[]; fromBlocks(ws): CodeOp[] }`
- Full if/elif/else mutation support in toBlocks/fromBlocks
- Roundtrip: `fromBlocks(toBlocks(ops))` ≈ ops

Key difference from v1 plan: the `// TODO: elif/else` is resolved here with real mutation handling.

- [ ] **Step 1: Write failing roundtrip test** (same as v1 plan)
- [ ] **Step 2: Run test → FAIL**
- [ ] **Step 3: Implement BlockAdapter.toBlocks()** — same as v1 plan + elif/else mutation for if_block:

```typescript
case "if_block": {
  block = workspace.newBlock("if_block");
  // First branch: if + condition + body
  const branch0 = op.branches[0];
  if (branch0) {
    this.setSlotInInput(block, "CONDITION", branch0.condition);
    this.createBodyBlocks(block, "BODY", branch0.body, workspace);
  }
  // Set elif/else count via mutation
  const elifCount = Math.max(0, op.branches.length - 2); // exclude if and else
  const hasElse = op.branches.length > 1 && op.branches[op.branches.length - 1].condition.kind === "empty";
  block.mutationToDom = () => {
    const m = document.createElement("mutation");
    m.setAttribute("elif", String(elifCount));
    m.setAttribute("else", hasElse ? "1" : "0");
    return m;
  };
  // Apply mutation to create elif/else inputs
  block.domToMutation(document.createElement("mutation")); // triggers mutator compose
  // Fill elif branches
  for (let i = 1; i < op.branches.length; i++) {
    const b = op.branches[i];
    const isElse = i === op.branches.length - 1 && b.condition.kind === "empty";
    const inputName = isElse ? "ELSE_BODY" : `ELIF_${i - 1}_CONDITION`;
    if (!isElse) {
      this.setSlotInInput(block, inputName, b.condition);
    }
    const bodyName = isElse ? "ELSE_BODY" : `ELIF_${i - 1}_BODY`;
    this.createBodyBlocks(block, bodyName, b.body, workspace);
  }
  break;
}
```

- [ ] **Step 4: Implement BlockAdapter.fromBlocks()** — same as v1 plan + elif/else reconstruction.

- [ ] **Step 5: Write comprehensive roundtrip tests** including if/elif/else.

- [ ] **Step 6: Run tests → PASS**

- [ ] **Step 7: Commit**

---

## Task 5: Build SelectionManager (Cursor Model)

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/SelectionManager.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/SelectionManager.test.ts`

**Produces:**
- `SelectionTarget` union type: `{ type: "block"; blockId: string } | { type: "slot"; blockId: string; inputName: string } | { type: "gap"; parentBlockId: string | null; position: number }`
- `class SelectionManager` — tracks current cursor position, provides `moveLeft/Right/Up/Down/enterSlot/exitSlot` methods, renders a visual selection DOM element overlaid on the Blockly SVG
- Visual: rectangle outline on blocks, dashed outline on slots, horizontal blinking line on gaps

**Design:**

The SelectionManager is the single source of truth for where the "game controller cursor" is. It:
1. Computes a flat spatial grid of all selectable positions (blocks at row N, slots within blocks, gaps between blocks)
2. Exposes `move(direction)` methods that update position by spatial adjacency
3. Renders a `<div>` overlay positioned absolutely at the current target's bounding rect
4. Fires an `onChange` callback when position changes (for BlockEditor to update UI state)

```
Grid model (recomputed when blocks change):
┌─────────────────────────────────────────────────┐
│ Row 0: [declare_var block]  [gap 0.5]  [set_var block]  │
│ Row 1: [if_block]  ◇CONDITION slot◇                            │
│ Row 2:   [gap 2.0]  [call_fn block]                            │
│ Row 3: [raw_stmt block]  [gap 3.5]                              │
│ Row 4:   [gap 4.0]                                               │
└─────────────────────────────────────────────────┘
```

`← →` moves within the same row; `↑ ↓` moves to the nearest column-aligned position in the prev/next row.

Slots are "inside" their parent block's row — they appear directly below the block header row. Gap positions are between blocks in the same column.

- [ ] **Step 1: Write SelectionManager unit tests** (grid model, spatial navigation, position querying)
- [ ] **Step 2: Implement grid model** — scan workspace, build row/column map
- [ ] **Step 3: Implement move methods** — `moveLeft()`, `moveRight()`, `moveUp()`, `moveDown()`
- [ ] **Step 4: Implement selection box rendering** — absolute-positioned `<div>` on Blockly SVG
- [ ] **Step 5: Commit**

---

## Task 6: Build GapManager (Gap Row Management)

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/GapManager.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/GapManager.test.ts`

**Produces:**
- `class GapManager` — creates, deletes, and merges gap rows
- Gap rows are rendered as invisible spacer blocks or extra vertical space between real blocks
- Provides: `insertGapAt(parentBlockId, position)` → creates a gap; `deleteGapAt(parentBlockId, position)` → removes gap and merges adjacent; `mergeGapUp(parentBlockId, position)` → merges gap into previous row

**Design:**

A "gap" in Blockly is a position where you can insert a new block. Since Blockly doesn't have native gap rows, we model them as entries in the SelectionManager grid that don't correspond to a real block.

When the user presses Enter on a gap:
1. GapManager shifts all blocks below the gap down by one row's worth of vertical space
2. The gap becomes a "real" position — SelectionManager positions the cursor there
3. Typing pops up FuzzySearchPopup; selecting an item creates a real block at that position

When the user presses Delete on a gap:
1. If gap is between two blocks, those blocks connect directly (gap removed)
2. If gap has a block above and nothing below, the gap row is removed (vertical space collapsed)

Implementation: gaps are tracked as `{ parentBlockId, index }` key in a Set. The SelectionManager consults GapManager when building its grid to include gap rows at the right positions.

- [ ] **Step 1: Write GapManager tests** — insert gap, delete gap, merge gap
- [ ] **Step 2: Implement gap state management** — Set-based, keyed by `parentBlockId||index`
- [ ] **Step 3: Implement gap ↔ Blockly connection integration** — when a block is created at a gap, connect it to the workspace or parent
- [ ] **Step 4: Commit**

---

## Task 7: Build KeyboardHandler (Input Interception)

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/KeyboardHandler.ts`
- Create: `poiesis/src/components/canvas/code/blocks/__tests__/KeyboardHandler.test.ts`

**Produces:**
- `class KeyboardHandler` — attaches keydown listener to Blockly workspace SVG
- Dispatches to: navigation (SelectionManager), fuzzy search (FuzzySearchPopup), deletion (GapManager + Blockly), raw text input, copy/paste
- Prevents Blockly's default keyboard behavior

**Design:**

The KeyboardHandler intercepts ALL keyboard events at the Blockly workspace SVG level. Based on `SelectionManager.getCurrentTarget()`:

| Current target | Character key | Enter | Delete | Backspace | `\` |
|---|---|---|---|---|---|
| block | no-op | no-op | delete block | delete block | no-op |
| slot | open fuzzy search + filter | open fuzzy search (all items) | clear slot → empty | remove last slot item | open raw input |
| gap | open fuzzy search + filter | insert gap row + open fuzzy search | delete gap row | merge gap up | open raw input |

Arrow keys always dispatch to SelectionManager.move().

`Ctrl+C/V/X` dispatch to Blockly clipboard operations.

The fuzzy search popup opens with `KeyboardHandler.openFuzzySearch(query, context)` which shows `FuzzySearchPopup` (Task 8).

- [ ] **Step 1: Write KeyboardHandler tests** — verify dispatch based on current target type
- [ ] **Step 2: Implement keyboard event interception** — attach to workspace SVG, preventDefault + stopPropagation for handled keys
- [ ] **Step 3: Implement dispatch logic** — per-target-type behavior table
- [ ] **Step 4: Wire into BlockEditor via React effect** — create KeyboardHandler on mount, dispose on unmount
- [ ] **Step 5: Commit**

---

## Task 8: Build FuzzySearchPopup

**Files:**
- Create: `poiesis/src/components/canvas/code/blocks/fields/FuzzySearchPopup.ts`
- Create: `poiesis/src/components/canvas/code/blocks/fields/__tests__/FuzzySearchPopup.test.ts`

**Produces:**
- `class FuzzySearchPopup` — standalone popup (NOT a Blockly Field subclass anymore)
- `open(query: string, context: "slot" | "gap" | "expr")` — shows popup with filtered items
- `close()` / `selectCurrent()` / `moveUp()` / `moveDown()` — keyboard-driven navigation
- `fuzzyScore(query, target): number` — scoring function with test coverage
- Context-aware item list:
  - `"gap"` context → all 13 block types
  - `"slot"` (text slot) → var_ref, literal, type_name
  - `"expr"` (expression slot) → all InlineOp types (7 variants)

**Design:**

The popup is a fixed-position `<div>` rendered near the current cursor position. It contains: a search `<input>` at the top, then a scrollable list of matching items. Items show: label (bold) + description (muted).

When the user selects an item:
- Gap context: create the selected block type via BlockAdapter, insert at gap position
- Text slot: set the slot's value to the selected text
- Expression slot: create an InlineOp block and insert into the expression slot's value input

The popup sends its result via a callback (`onSelect: (item: SearchItem) => void`) which the KeyboardHandler binds.

- [ ] **Step 1: Write fuzzyScore tests** — exact, prefix, substring, case-insensitive, consecutive bonus
- [ ] **Step 2: Implement fuzzyScore**
- [ ] **Step 3: Implement FuzzySearchPopup class** — DOM creation, filtering, keyboard handling
- [ ] **Step 4: Write integration test** — open popup, type "wh", arrow-down, enter → verifies callback
- [ ] **Step 5: Commit**

---

## Task 9: Build BlockEditor React Component

**Files:**
- Create: `poiesis/src/components/canvas/code/BlockEditor.tsx`
- Create: `poiesis/src/components/canvas/code/BlockEditor.module.css`

**Produces:**
- `export const BlockEditor: React.FC<Props>` — full editor component
- Wires together: Blockly workspace + BlockAdapter + SelectionManager + GapManager + KeyboardHandler
- Raw text input modal (triggered by `\`)
- "文本模式" fallback button → CodeEditorModal

- [ ] **Step 1: Create CSS** — same as v1 plan Task 6, plus `.rawInputModal` styles
- [ ] **Step 2: Create BlockEditor component** — similar structure to v1 plan, plus:

```typescript
// Key additions over v1 plan:

// 1. After workspace creation, initialize custom layers:
const selectionMgr = new SelectionManager(ws);
const gapMgr = new GapManager(ws);
const keyboardHandler = new KeyboardHandler(ws, selectionMgr, gapMgr, {
  onFuzzySearch: (query, context) => { /* open FuzzySearchPopup */ },
  onRawInput: () => setShowRawInput(true),
  onOpsChanged: () => { /* sync to CodeOp[] */ },
});

// 2. Raw input modal state:
const [showRawInput, setShowRawInput] = useState(false);

// 3. Raw input modal rendering:
{showRawInput && (
  <div className={styles.rawInputModal}>
    <textarea ... />
    <button onClick={commitRawInput}>Enter 提交</button>
    <button onClick={() => setShowRawInput(false)}>Esc 取消</button>
  </div>
)}
```

- [ ] **Step 3: Wire into FlowCanvas** — replace `import { CodeCardEditor }` with `import { BlockEditor }`
- [ ] **Step 4: Verify TypeScript compiles**
- [ ] **Step 5: Commit**

---

## Task 10: Delete Old Lexical Code

**Files to delete:**
- `poiesis/src/components/canvas/code/lexical/` (entire directory)
- `poiesis/src/components/canvas/code/CodeCardEditor.tsx`
- `poiesis/src/components/canvas/code/CodeCardEditor.module.css`
- `poiesis/src/components/canvas/code/CodeCardEditor.stories.tsx`
- `poiesis/src/core/types/editor-node.ts`

- [ ] **Step 1: Delete files**
- [ ] **Step 2: Verify no remaining references** — grep for `CodeCardEditor`, `editor-node`
- [ ] **Step 3: Run remaining tests** — `vitest run --exclude="lexical"`
- [ ] **Step 4: Commit**

---

## Task 11: Integration & Visual Verification

**Method:** Start Poiesis via Victauri, open BlockEditor, verify interaction flow.

- [ ] **Step 1: Start app + navigate to an element with CodeOps**
- [ ] **Step 2: Verify:** Blockly workspace renders, blocks visible with Zelos theme
- [ ] **Step 3: Verify:** Arrow keys navigate between blocks/slots/gaps
- [ ] **Step 4: Verify:** Type `i` on gap → fuzzy search popup shows "if"
- [ ] **Step 5: Verify:** Enter on search result → block inserted at gap
- [ ] **Step 6: Verify:** Delete on block → block removed
- [ ] **Step 7: Verify:** Enter on gap → gap row added
- [ ] **Step 8: Verify:** `\` → raw text input modal → Enter → RawCodeOp stored
- [ ] **Step 9: Verify:** Save button → correct CodeOp[] output
- [ ] **Step 10: Fix issues, commit fixes**

---

## Task 12: Update Storybook Stories

- [ ] **Step 1: Create BlockEditor.stories.tsx** — Empty, Let+Set, IfBlock, WhileLoop, ForLoop
- [ ] **Step 2: Verify stories render in Storybook**
- [ ] **Step 3: Commit**

---

## Self-Review

| Requirement | Task |
|-------------|------|
| CodeOp SSOT | Task 2 (model), Task 4 (adapter), Task 9 (sync on change) |
| SlotValue model | Task 2 (data model), Task 4 (adapter) |
| Backward compat | Task 2 (fromLegacyString), Task 4 (roundtrip tests) |
| 13 block types | Task 3 (definitions), Task 4 (adapter) |
| if/elif/else mutation | Task 3 (mutations.ts), Task 4 (adapter elif/else support) |
| C-shaped blocks | Task 3 (definitions with input_statement) |
| Zelos default colors | Task 3 (color constants) |
| No drag-and-drop | Task 7 (KeyboardHandler prevents drag events) |
| Arrow key navigation | Task 5 (SelectionManager spatial grid), Task 7 (KeyboardHandler dispatch) |
| Cursor on block → no typing | Task 7 (dispatch table: block + character = no-op) |
| Cursor on slot → fuzzy search | Task 7 (dispatch) + Task 8 (FuzzySearchPopup) |
| Cursor on gap → fuzzy search | Task 7 (dispatch) + Task 8 (FuzzySearchPopup) |
| Enter on gap → add gap row | Task 6 (GapManager) + Task 7 (dispatch) |
| Delete on gap → delete gap | Task 6 (GapManager) + Task 7 (dispatch) |
| Backspace on block → delete | Task 7 (dispatch → block.dispose()) |
| `\` → raw text input | Task 9 (raw input modal) |
| Raw text → RawCodeOp (no parse) | Task 9 (commitRawInput creates RawCodeOp) |
| Ctrl+C/V/X | Task 7 (Blockly clipboard intercept) |
| Fuzzy search (VS Code palette) | Task 8 (fuzzyScore + popup filtering) |
| Context-aware search results | Task 8 (slot vs gap vs expr context) |
| All old code deleted | Task 10 |
| Selection box visual | Task 5 (DOM overlay rendering) |
