# Task 3 Report: Define Blockly Block Definitions

**Status:** COMPLETE

**Commit:** `edd2e29` -- `feat: add Blockly block definitions for all 13 block types`

## Files Created

1. `poiesis/src/components/canvas/code/blocks/definitions.ts` -- Block JSON definitions
2. `poiesis/src/components/canvas/code/blocks/mutations.ts` -- if_block mutator
3. `poiesis/src/components/canvas/code/blocks/__tests__/definitions.test.ts` -- Tests

## What Was Done

### definitions.ts
- Defined 7-color palette (CONTROL, VARIABLE, FUNCTION, ASYNC, BREAK, COMMENT, RAW) using Zelos/Scratch 3.0 defaults
- 9 simple statement blocks: declare_var (let/mut dropdown), set_var (OP dropdown), return_stmt, call_fn, await_expr, break_stmt, continue_stmt, comment_stmt (field_input), raw_stmt (field_multilinetext)
- 3 C-shaped blocks: if_block (with mutator reference), while_loop, for_in
- All blocks have `previousStatement: null` and `nextStatement: null` for vertical stacking
- Exported `BLOCK_DEFINITIONS` record and `registerAllBlocks()` function

### mutations.ts
- Implemented `IF_BLOCK_MUTATOR_MIXIN` with:
  - `mutationToDom` -- serializes elif count and hasElse state to `<mutation>` element
  - `domToMutation` -- deserializes and calls `rebuildBranches_()`
  - `rebuildBranches_()` -- dynamically adds/removes elif condition+body inputs while preserving child blocks
  - `decompose`/`compose` -- for Blockly mutator editor UI, using container/elif_case/else_case helper blocks
- Registered `if_block_mutator` extension and 3 companion blocks: `if_block_container`, `if_block_elif_case`, `if_block_else_case`

### Tests
- 46 tests all PASS
- Verifies: all 12 block types exist, have type/colour fields, have proper previous/next connections, C-shaped blocks use input_statement, if_block has mutator, declare_var has MUT dropdown, set_var has OP dropdown, comment uses field_input, raw uses field_multilinetext, registerAllBlocks is exported

---

## Review Fixes (Commit `5069342`)

Applied 3 review findings:

1. **Finding 1 (HIGH):** Added `registerIfBlockMutator()` call at the end of `registerAllBlocks()` to ensure the if_block mutator extension is registered along with blocks.

2. **Finding 2 (CRITICAL):** Replaced two empty no-op branches in `rebuildBranches_()`:
   - Lines 80-82: Now properly unplugs else body's child blocks when `hasElse_` is false using `conn.targetBlock()!.unplug()`
   - Lines 142-145: Replaced with a clarifying comment; visibility is handled by the static def (message2), children reconnect based on `hasElse_` state

3. **Finding 3 (MINOR):** Fixed test description from "all 13 block types" to "all 12 block types"

**Additional fix:** Removed unused `blockToDom?` optional property from the mixin object (caused a parse error with the oxc parser — `?` is not valid in object literals).

**Test result:** 46/46 PASS
