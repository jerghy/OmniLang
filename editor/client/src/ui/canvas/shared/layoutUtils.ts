/** Auto-layout for the Rete.js node editor canvas.
 *
 *  Uses dagre for hierarchical directed-graph layout of code blocks.
 *  View elements are stacked on the right side.
 *  Data-flow sources (signals, stores) naturally appear on the left;
 *  data-flow sinks (actions, computed results) appear on the right. */

import dagre from "dagre";
import type { ElementData, WireData } from "../../../core/types";

// ── Node dimensions (matched to ACTUAL Rete node rendering) ──
// Real rendered sizes measured via getBoundingClientRect():
//   Code blocks: ~180×162 (signals/actions/computed), ~180×126 (stores)
//   View elements: ~180×198 (inputs), ~180×306 (buttons)
const CB_WIDTH = 200;   // slightly larger than actual 180 for padding
const CB_HEIGHT = 170;  // slightly larger than actual 162, covers store (126) too
const VIEW_WIDTH = 200;
const VIEW_HEIGHT = 200; // covers input (198), tight for button (306) but ok

// ── Layout spacing ──
const RANK_SEP = 300;   // horizontal spacing between ranks (data-flow steps)
const NODE_SEP = 140;   // vertical spacing between nodes in the same rank
const MARGIN = 80;      // margin from canvas edges

// ── Type ──
export interface LayoutResult {
  [elementId: string]: { x: number; y: number };
}

/** Compute a hierarchical auto-layout.
 *
 *  Strategy:
 *  1. Separate code blocks from view elements
 *  2. Run dagre on ALL code blocks (connected + disconnected),
 *     disconnected ones form isolated nodes dagre places at origin-relative
 *  3. Stack view elements vertically on the right side of the canvas */
export function computeAutoLayout(
  elements: ElementData[],
  wires: WireData[],
): LayoutResult {
  const codeBlocks = elements.filter((e) => e.nodeType === "codeblock");
  const viewElements = elements.filter((e) => e.nodeType === "view");

  const result: LayoutResult = {};

  // ── 1. Layout ALL code blocks with dagre ──────────
  //    dagre naturally handles connected nodes. Disconnected nodes
  //    (no edges) become isolated vertices — dagre places them
  //    at reasonable default positions. This is simpler and avoids
  //    the overlap risk of a separate grid.
  const dagreResult = layoutWithDagre(codeBlocks, wires);
  Object.assign(result, dagreResult);

  // ── 2. Layout view elements on the right side ────────
  // Determine the right edge of the rightmost code block
  const maxCBX = codeBlocks.length > 0
    ? Math.max(...codeBlocks.map((el) => (result[el.id]?.x ?? 0) + CB_WIDTH))
    : MARGIN;
  const viewStartX = maxCBX + RANK_SEP;

  // Sort view elements by tag for predictable grouping
  const tagOrder = [
    "Card", "Flex", "Container", "Heading", "Text", "Button", "Input",
    "Select", "Checkbox", "Form", "List", "ListItem", "Image",
    "Spinner", "Progress", "Alert", "Link", "TextArea",
  ];
  const sortedViews = [...viewElements].sort((a, b) => {
    const ai = tagOrder.indexOf(a.tag);
    const bi = tagOrder.indexOf(b.tag);
    return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
  });

  sortedViews.forEach((el, i) => {
    result[el.id] = {
      x: viewStartX,
      y: MARGIN + i * (VIEW_HEIGHT + 20),
    };
  });

  // ── 3. Overlap check: ensure no two code blocks overlap ──
  preventOverlap(result, codeBlocks);

  return result;
}

/** Run dagre on code blocks to produce a left-to-right hierarchical layout. */
function layoutWithDagre(
  elements: ElementData[],
  wires: WireData[],
): LayoutResult {
  if (elements.length === 0) return {};

  const g = new dagre.graphlib.Graph({ compound: false, directed: true, multigraph: false });
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: "LR",        // left-to-right data flow
    nodesep: NODE_SEP,    // vertical spacing within a rank
    ranksep: RANK_SEP,    // horizontal spacing between ranks
    marginx: MARGIN,
    marginy: MARGIN,
  });

  // Add nodes with estimated sizes matching actual Rete rendering
  for (const el of elements) {
    // store nodes are slightly shorter, but use uniform size for safety
    const w = CB_WIDTH;
    const h = CB_HEIGHT;
    g.setNode(el.id, { width: w, height: h });
  }

  // Add edges (only between code blocks, filter out view-element wires)
  const elementIds = new Set(elements.map(e => e.id));
  for (const w of wires) {
    if (elementIds.has(w.fromId) && elementIds.has(w.toId)) {
      g.setEdge(w.fromId, w.toId);
    }
  }

  dagre.layout(g);

  const result: LayoutResult = {};
  interface DagreNode { width: number; height: number; x: number; y: number; }
  for (const el of elements) {
    const node = g.node(el.id);
    if (node) {
      // dagre returns CENTER coordinates; convert to top-left for Rete
      const dagreNode = node as DagreNode;
      const nw = dagreNode.width ?? CB_WIDTH;
      const nh = dagreNode.height ?? CB_HEIGHT;
      result[el.id] = {
        x: dagreNode.x - nw / 2,
        y: dagreNode.y - nh / 2,
      };
    }
  }
  return result;
}

/** Post-processing: detect and fix overlapping code blocks.
 *  If two code blocks overlap, shift the lower one down. */
function preventOverlap(
  result: LayoutResult,
  codeBlocks: ElementData[],
): void {
  // Sort by x, then y
  const sorted = [...codeBlocks]
    .filter(cb => result[cb.id])
    .sort((a, b) => {
      const ax = result[a.id].x;
      const bx = result[b.id].x;
      return ax !== bx ? ax - bx : result[a.id].y - result[b.id].y;
    });

  for (let i = 0; i < sorted.length; i++) {
    for (let j = i + 1; j < sorted.length; j++) {
      const ai = result[sorted[i].id];
      const aj = result[sorted[j].id];
      if (!ai || !aj) continue;

      // Check horizontal overlap: if they're in roughly the same column
      const xOverlap = Math.abs(ai.x - aj.x) < CB_WIDTH;

      if (xOverlap) {
        // Check vertical overlap
        const minGap = 20; // minimum gap between nodes
        const aiBottom = ai.y + CB_HEIGHT;
        if (aj.y < aiBottom + minGap) {
          // Shift the lower node down
          aj.y = aiBottom + minGap;
        }
      }
    }
  }
}

/** Compute the original sequential layout (fallback for undo reference). */
export function computeSequentialLayout(
  elements: ElementData[],
): LayoutResult {
  let codeIdx = 0;
  let viewIdx = 0;
  const result: LayoutResult = {};
  for (const el of elements) {
    const isC = el.nodeType === "codeblock";
    result[el.id] = {
      x: isC ? MARGIN : MARGIN + 600,
      y: isC
        ? MARGIN + codeIdx * (CB_HEIGHT + 30)
        : MARGIN + viewIdx * (VIEW_HEIGHT + 20),
    };
    if (isC) codeIdx++;
    else viewIdx++;
  }
  return result;
}
