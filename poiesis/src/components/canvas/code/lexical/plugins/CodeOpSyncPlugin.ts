// poiesis/src/components/canvas/code/lexical/plugins/CodeOpSyncPlugin.ts
import { useEffect, useRef } from "react";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $getRoot,
  $createParagraphNode,
  $createTextNode,
} from "lexical";
import type { CodeOp } from "../../../../../core/types/code-op";
import { $createCardElementNode, $isCardElementNode } from "../nodes/CardElementNode";
import { $createSlotTextNode, $isSlotTextNode, type SlotType } from "../nodes/SlotTextNode";

interface Props {
  initialOps: CodeOp[];
  onSync: (ops: CodeOp[]) => void;
}

/**
 * Two-way sync between Lexical node tree and CodeOp[].
 *
 * - On mount: loads initialOps into the editor as CardElementNodes
 * - On change: extracts CodeOp[] from node tree and calls onSync
 */
export function CodeOpSyncPlugin({ initialOps, onSync }: Props): null {
  const [editor] = useLexicalComposerContext();
  const initializedRef = useRef(false);

  // ── Load initial ops into editor (once) ──
  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    editor.update(() => {
      const root = $getRoot();
      root.clear();

      if (initialOps.length === 0) {
        // Start with an empty paragraph for typing
        root.append($createParagraphNode());
        return;
      }

      for (const op of initialOps) {
        const node = codeOpToLexicalNode(op);
        root.append(node);
      }

      // Add trailing paragraph for new input
      root.append($createParagraphNode());
    });
  }, [editor, initialOps]);

  // ── Extract CodeOp[] on change and report ──
  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const root = $getRoot();
        const ops = extractCodeOps(root);
        onSync(ops);
      });
    });
  }, [editor, onSync]);

  return null;
}

// ── CodeOp → Lexical node ──

function codeOpToLexicalNode(op: CodeOp) {
  switch (op.kind) {
    case "declare_var": {
      const card = $createCardElementNode("declare_var", op.pid);
      card.append($createTextNode("let"));
      if (op.mutable) {
        card.append($createTextNode(" mut"));
      }
      card.append($createTextNode(" "));
      card.append($createSlotTextNode(op.name, "name"));
      if (op.varType) {
        card.append($createTextNode(" : "));
        card.append($createSlotTextNode(op.varType, "type"));
      }
      if (op.initValue !== undefined) {
        card.append($createTextNode(" = "));
        card.append($createSlotTextNode(op.initValue, "value"));
      }
      return card;
    }
    case "set_var": {
      const card = $createCardElementNode("set_var", op.pid);
      card.append($createSlotTextNode(op.name, "name"));
      card.append($createTextNode(` ${op.operator} `));
      card.append($createSlotTextNode(op.value, "value"));
      return card;
    }
    case "if_block": {
      // For simplicity, flatten if_block branches for initial load
      // Full block handling (body recursion) deferred to Phase 2
      const card = $createCardElementNode("if_block", op.pid);
      const firstBranch = op.branches[0];
      card.append($createTextNode("if "));
      card.append($createSlotTextNode(firstBranch?.condition ?? "", "condition"));
      card.append($createTextNode(" { ... }"));
      return card;
    }
    case "while": {
      const card = $createCardElementNode("while", op.pid);
      card.append($createTextNode("while "));
      card.append($createSlotTextNode(op.condition, "condition"));
      card.append($createTextNode(" { ... }"));
      return card;
    }
    case "for_in": {
      const card = $createCardElementNode("for_in", op.pid);
      card.append($createTextNode("for "));
      card.append($createSlotTextNode(op.itemName, "item"));
      card.append($createTextNode(" in "));
      card.append($createSlotTextNode(op.collection, "collection"));
      card.append($createTextNode(" { ... }"));
      return card;
    }
    case "return": {
      const card = $createCardElementNode("return", op.pid);
      card.append($createTextNode("return"));
      if (op.value) {
        card.append($createTextNode(" "));
        card.append($createSlotTextNode(op.value, "value"));
      }
      return card;
    }
    case "call_fn": {
      const card = $createCardElementNode("call_fn", op.pid);
      card.append($createSlotTextNode(op.fnName, "fnName"));
      card.append($createTextNode("("));
      if (op.args.length > 0) {
        card.append($createSlotTextNode(op.args.join(", "), "arg"));
      } else {
        card.append($createSlotTextNode("", "arg"));
      }
      card.append($createTextNode(")"));
      return card;
    }
    case "await": {
      const card = $createCardElementNode("await", op.pid);
      card.append($createTextNode("await "));
      card.append($createSlotTextNode(op.value, "value"));
      return card;
    }
    case "break": {
      const card = $createCardElementNode("break", op.pid);
      card.append($createTextNode("break"));
      return card;
    }
    case "continue": {
      const card = $createCardElementNode("continue", op.pid);
      card.append($createTextNode("continue"));
      return card;
    }
    case "comment": {
      const card = $createCardElementNode("comment", op.pid);
      card.append($createTextNode("// "));
      card.append($createSlotTextNode(op.text, "value"));
      return card;
    }
    case "raw": {
      const card = $createCardElementNode("raw", op.pid);
      card.append($createSlotTextNode(op.source, "value"));
      return card;
    }
    default:
      return $createParagraphNode();
  }
}

// ── Lexical nodes → CodeOp[] ──

function extractCodeOps(root: ReturnType<typeof $getRoot>): CodeOp[] {
  const ops: CodeOp[] = [];
  const children = root.getChildren();

  for (const child of children) {
    if ($isCardElementNode(child)) {
      ops.push(extractCardOp(child));
    }
    // Skip empty paragraphs — they're just typing surfaces
  }

  return ops;
}

function extractCardOp(card: ReturnType<typeof $createCardElementNode>): CodeOp {
  const kind = card.__cardKind;
  const pid = card.__cardPid;
  const slotValues = extractSlotValues(card);

  switch (kind) {
    case "declare_var": {
      const nameText = slotValues.get("name") ?? "";
      // Detect mutable from card children
      const cardText = card.getTextContent();
      const mutable = cardText.includes("let mut");
      return {
        kind: "declare_var",
        pid,
        mutable,
        name: nameText,
        varType: slotValues.get("type"),
        initValue: slotValues.get("value"),
        scope: "fn",
      };
    }
    case "set_var": {
      const name = slotValues.get("name") ?? "";
      const valueText = slotValues.get("value") ?? "";
      // Detect operator from card text
      const cardText = card.getTextContent();
      const opMatch = cardText.match(/([+\-*/%])?=/);
      const operator = (opMatch?.[0] ?? "=") as Extract<CodeOp, { kind: "set_var" }>["operator"];
      return {
        kind: "set_var",
        pid,
        name,
        operator,
        value: valueText,
      };
    }
    case "if_block": {
      const bodyOps = extractChildCardOps(card);
      return {
        kind: "if_block",
        pid,
        branches: [{
          condition: slotValues.get("condition") ?? "",
          body: bodyOps,
        }],
      };
    }
    case "while": {
      const bodyOps = extractChildCardOps(card);
      return {
        kind: "while",
        pid,
        condition: slotValues.get("condition") ?? "",
        body: bodyOps,
      };
    }
    case "for_in": {
      const bodyOps = extractChildCardOps(card);
      return {
        kind: "for_in",
        pid,
        itemName: slotValues.get("item") ?? "item",
        collection: slotValues.get("collection") ?? "",
        body: bodyOps,
      };
    }
    case "return": {
      return {
        kind: "return",
        pid,
        value: slotValues.get("value"),
      };
    }
    case "call_fn": {
      return {
        kind: "call_fn",
        pid,
        fnName: slotValues.get("fnName") ?? "",
        args: (slotValues.get("arg") ?? "").split(",").map(a => a.trim()).filter(Boolean),
      };
    }
    case "await": {
      return {
        kind: "await",
        pid,
        value: slotValues.get("value") ?? "",
      };
    }
    case "break": return { kind: "break", pid };
    case "continue": return { kind: "continue", pid };
    case "comment": {
      return {
        kind: "comment",
        pid,
        text: slotValues.get("value") ?? "",
      };
    }
    case "raw": {
      return {
        kind: "raw",
        pid,
        source: slotValues.get("value") ?? "",
      };
    }
    default:
      return { kind: "raw", pid, source: card.getTextContent() };
  }
}

function extractChildCardOps(card: ReturnType<typeof $createCardElementNode>): CodeOp[] {
  const ops: CodeOp[] = [];
  const children = card.getChildren();
  for (const child of children) {
    if ($isCardElementNode(child)) {
      ops.push(extractCardOp(child));
    }
  }
  return ops;
}

function extractSlotValues(card: ReturnType<typeof $createCardElementNode>): Map<SlotType, string> {
  const map = new Map<SlotType, string>();
  const children = card.getChildren();
  for (const child of children) {
    if ($isSlotTextNode(child)) {
      map.set(child.getSlotType(), child.getTextContent());
    }
  }
  return map;
}
