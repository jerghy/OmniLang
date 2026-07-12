# Task 7 Report: chain-emitter.ts — add buildCodeFlowGraphs + formatChainAsPidAnnotations

## Status: COMPLETED

## Changes Summary

Added two new exported functions to `poiesis/src/services/codegen/chain-emitter.ts` and updated imports. All existing exports (`emitChains`) are unchanged.

### New Imports
- `import { emitPidAnnotation } from "./annotation-registry"` — for emitting `// @pid:<id>` annotations
- `import type { CodeFlowGraph } from "../../core/types/project-json"` — for the JSON persistence type
- `import type { CodeblockElementData } from "../../core/types/element"` — for the code flow element type in `buildCodeFlowGraphs`

### New Export: `buildCodeFlowGraphs`

```typescript
export function buildCodeFlowGraphs(
  parsedGraphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>,
): Record<string, CodeFlowGraph>
```

- Converts parsed code flow data (from `ParseResult.codeFlowGraphs`) to the merge-friendly `project.json` persistence format
- Each fn/action entry maps: element IDs to `CodeFlowNode` objects with `nodeType: el.tag`, and wire data to `CodeFlowWire` objects with from/to/port fields
- Keyed by function name (serves as PID fallback)

**Adaptation from original brief**: The brief's signature used `Map<string, ChainBlock>`, but `ChainBlock` only contains formatted comment strings (`nodeLines`, `wireLines`) — it lacks the structured `pid`/`nodes`/`wires` fields needed to build `CodeFlowGraph`. Changed to accept the parsed code flow data format which naturally maps to `CodeFlowGraph`.

### New Export: `formatChainAsPidAnnotations`

```typescript
export function formatChainAsPidAnnotations(chain: ChainBlock): string
```

- Parses node IDs from `chain.nodeLines` (format: `//   @node <id> ...`)
- Emits `// @pid:<id>` for each node using `emitPidAnnotation()`
- Replaces the old `// @chain/@node/@wire` comment block format

### No Changes
- `emitChains` still works as before — same signature, same output
- No modifications to existing code, only additions at the bottom

## Files Changed

- `D:/space/OmniLang/poiesis/src/services/codegen/chain-emitter.ts` — imports + 2 new exports

## Verification

- `npx tsc --noEmit` passes with zero new errors related to chain-emitter.ts (pre-existing errors in other files unchanged)
