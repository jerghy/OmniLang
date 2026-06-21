# Task 14 Report: ChainEmitter

**Status:** Complete

**Commit:** `b92267b6` — `feat(omni-gen): add ChainEmitter for @chain/@node/@wire blocks`

## Files Created

- `editor/client/src/omni-gen/chain-emitter.ts` (257 lines)
- `editor/client/src/omni-gen/__tests__/chain-emitter.test.ts` (250 lines)

## Implementation

`emitChains(tab, functions)` returns a `Map<string, ChainBlock>` keyed by function name. Core algorithm:

1. For each `customFn` in lowered functions, find the matching graph element by `config.fnName`
2. BFS from the fnNode collecting all reachable nodes + wires (undirected traversal)
3. `formatNode(el)`: looks up `NODE_ATTRS[tag]` for required/optional attributes; resolves from `el.config` (CustomFunctionMeta) or `el.props`; appends `pos=x,y`
4. `formatWire(w, elMap)`: formats as `from.port -> to.port`; customFn nodes skip port name on the from side (body port is implicit)
5. S5 validation: checks getVariable/setVariable names exist in `tab.variables`

Key design decisions:
- String literal values (literal node's `value`) are quoted (`value="hello"`) to distinguish from bare identifier attributes (`name=counter`)
- Signal `type` attribute resolves from `typeAnnotation` prop; `init` resolves from `init` or `default` prop
- customFn attributes (`fnName`, `returnType`) resolve from `el.config` (CustomFunctionMeta interface)
- Functions without a matching graph element are silently skipped

## Test Results

All 121 tests pass across 12 test files (7 specific to chain-emitter, 114 existing regressions):

| Test | Status |
|------|--------|
| basic chain with customFn + literal + wire | PASS |
| empty tab returns empty map | PASS |
| multiple functions produce multiple chain blocks | PASS |
| BFS captures all reachable nodes and wires | PASS |
| nodes formatted with required and optional attributes | PASS |
| function with no matching element returns empty | PASS |
| string literal values quoted in node attrs | PASS |

## Review Fixes (2025-06-21)

### Fix 1: `resolveAttr()` config fallback
**Problem:** `resolveAttr()` checked `el.props[attrName]` for "compare.op" but compare nodes store `op` in `el.config.op` (not `el.props.op`). When `el.props.op` was undefined, the attribute was silently dropped.
**Fix:** After checking `el.props[attrName]` for non-customFn nodes, also check `el.config[attrName]` before giving up. Changed the final line from `return el.props[attrName];` to a two-step check: first `el.props`, then `el.config` fallback.

### Fix 2: `expression-lowering.ts` compare.op read
**Problem:** `compare.op` was read as `el.props.op || "=="`. If op was stored in `el.config.op`, it would silently default to `"=="`.
**Fix:** Changed to `el.props.op || el.config.op || "=="`.

### Fix 3: Missing NODE_ATTRS entries
Added entries for:
- `store`: required `["name"]`, optional `[]`
- `computed`: required `["name"]`, optional `["returnType"]`
- `fn`: required `["source"]`, optional `[]`
- `updateUIProperty`: required `["targetId", "property"]`, optional `[]`

### Tests Added
- `compare node with op in config ➜ @node line includes op=>=`
- `store node with name attribute`
- `computed node with name and returnType`
- `fn node with source attribute`
- `updateUIProperty node with targetId and property`

**Commit:** `<SHA>` — `fix(omni-gen): address review — config fallback in resolveAttr, missing NODE_ATTRS, compare.op fix`

**Test Results:** 62 tests pass across 7 test files (no regressions).

## Concerns

None. The implementation follows the brief's algorithm precisely and integrates cleanly with existing `ChainBlock` and `LoweredCode` types in `types.ts`.
