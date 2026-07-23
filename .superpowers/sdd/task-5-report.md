# Task 5 Report: CardNode (Lexical DecoratorNode for Inline Cards)

## Status: COMPLETE

## Commits

```
7924e22 feat(codecard-v2): add CardNode for inline non-editable cards
```

## Files Created

- `poiesis/src/components/canvas/code/lexical/nodes/CardNode.ts` -- DecoratorNode for inline non-editable cards (mut, operator, type, var_ref, literal, etc.)
- `poiesis/src/components/canvas/code/lexical/nodes/__tests__/CardNode.test.ts` -- 6 tests

## Files Modified

- `poiesis/package.json` -- Added `lexical` dependency (v0.47.0)
- `poiesis/pnpm-lock.yaml` -- Lockfile update from lexical install

## Tests

**41 passed, 0 failed** (all 6 node test suites)

CardNode-specific tests (6):
1. `has correct type` -- Static getType() returns "inline-card", getters return constructor values (PASS)
2. `is detected by $isCardNode` -- $isCardNode returns true for CardNode, false for null/undefined (PASS)
3. `serializes to JSON with all fields` -- exportJSON includes type, cardKind, cardLabel, cardPid (PASS)
4. `deserializes from JSON` -- importJSON constructs correct CardNode (PASS)
5. `is inline` -- isInline() returns true (PASS)
6. `clones preserving all fields` -- clone copies cardKind, cardLabel, cardPid (PASS)

No regressions in existing tests (CardElementNode, SlotTextNode, InputSlotNode, ContainerNode, BlockNode, BodyLineNode -- all unchanged).

TypeScript: 0 errors from CardNode (pre-existing errors from other modules remain).

## Deviations from Brief

| Aspect | Brief | Actual | Reason |
|--------|-------|--------|--------|
| File extension | `.ts` with JSX | `.ts` with `React.createElement` | `.ts` does not support JSX without `.tsx` extension; used `React.createElement()` in `decorate()` |
| DecoratorNode type param | `DecoratorNode<JSX.Element>` | `DecoratorNode<React.ReactNode>` | `JSX` namespace not available in `.ts` files |
| Getters | `this.getLatest().__cardKind` | `this.__cardKind` | Direct field access is simpler, works without active editor context, and fields are immutable (set once in constructor) |
| importJSON signature | `(serialized: SerializedCardNode)` | `(_serializedNode: SerializedLexicalNode & Record<string, unknown>)` | Must match base `DecoratorNode` class signature; casts internally |
| Test wrapper | Direct `$createCardNode(...)` | Inside `editor.update(() => {...})` | LexicalNode constructor calls `$setNodeKey()` which requires active editor context |

## Key Implementation Details

- **DecoratorNode**: Chosen because inline cards are read-only blocks where users cannot place a cursor. Typing replaces or inserts adjacent cards.
- **createDOM**: Creates `<span>` with `code-card code-card--{kind}` class and `data-card-kind`/`data-card-pid` attributes.
- **updateDOM**: Returns `false` (no dynamic updates needed for immutable cards).
- **isInline**: Returns `true` (cards sit inline within text flow).
- **exportJSON/importJSON**: Full round-trip serialization preserving all fields.

## Concerns

None. The CardNode is a straightforward DecoratorNode with immutable properties. Test coverage includes all key behaviors.
