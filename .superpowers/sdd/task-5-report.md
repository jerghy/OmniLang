# Task 5 Report: Card-to-Text Degradation Transform

**Status:** DONE

## Deliverables

| File | Status |
|------|--------|
| `poiesis/src/components/canvas/code/lexical/transforms/card-to-text.ts` | Created |
| `poiesis/src/components/canvas/code/lexical/transforms/__tests__/card-to-text.test.ts` | Created |

## Implementation

`$degradeCardToText(cardNode)` replaces a `CardElementNode` with a `ParagraphNode` containing the card's text content. Used when the user deletes all slots in a card (Backspace at boundary).

```typescript
export function $degradeCardToText(cardNode: CardElementNode): void {
  const textContent = cardNode.getTextContent();
  const paragraph = $createParagraphNode();
  paragraph.append($createTextNode(textContent));
  cardNode.replace(paragraph);
}
```

## Test Results

```
✓ 2 passed (2 tests)

Test Files  1 passed (1)
```

Both tests pass:
1. **replaces card with paragraph containing text representation** -- verifies the card is replaced by a `ParagraphNode`
2. **preserves text content from card slots** -- verifies slot text ("count") survives the transform

## Notes

- Used corrected import paths (`from "lexical"` and `from "../nodes/CardElementNode"`) per the brief's instruction, not the auto-generated node_modules path from the Plan.
- CardElementNode and SlotTextNode (Tasks 2-4) were already in place.
