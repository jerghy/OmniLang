# Task 14 Report: Integration test — full edit flow

**Status:** COMPLETE

## Commits

- `b8141b7` — `test(codecard-v2): add full pipeline integration roundtrip tests`

## Files Created

- `poiesis/src/components/canvas/code/lexical/__tests__/integration-v2.test.ts`

## Test Results

All **6 tests PASS** in `integration-v2.test.ts`:

| Test | Status |
|------|--------|
| declare_var roundtrips through EditorNode | PASS |
| set_var roundtrips through EditorNode | PASS |
| if_block with elif/else roundtrips | PASS |
| while loop roundtrips | PASS |
| for_in roundtrips | PASS |
| complex real-world code roundtrips | PASS |

Each test exercises the full pipeline: `CodeOp -> convertToNodes -> EditorNode -> convertToOps -> CodeOp -> emitOps -> source -> parseOps -> CodeOp`.

## Concerns

- React Doctor flagged `FlowCanvasInner` in `FlowCanvas.tsx` as a 300+ line component (maintainability warning). Pre-existing issue, unrelated to this task.
- No regressions introduced. All 6 new tests pass cleanly.
