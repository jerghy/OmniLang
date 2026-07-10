// poiesis/src/services/codegen/graph-parser.ts
// Parse an OmniView fn body string → code flow graph (nodes + wires).
// Reverse of GraphCompiler. Uses // @pid:xxx annotations to identify nodes.
// Handles proper branch wiring for condition nodes via brace-depth analysis.

import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { uid } from "../../core/utils/id";

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

interface ParsedSegment {
  pid: string;
  startLine: number;
  endLine: number;
  code: string;
  type: "fn" | "code-block" | "condition" | "return";
}

interface BranchDef {
  id: string;
  type: "if" | "elif" | "else";
  condition: string;
}

/**
 * Branch context assigned to segments that fall within a condition's brace scope.
 * Tells the wire reconstruction which condition node and which branch the segment
 * belongs to.
 */
interface BranchContext {
  conditionPid: string;
  branchIndex: number;
}

// ---------------------------------------------------------------------------
// splitByPid – split fn body into segments at // @pid:xxx comment boundaries
// ---------------------------------------------------------------------------

/**
 * Split fn body into segments delimited by // @pid:xxx comments.
 * Each segment starts at a @pid comment and ends before the next @pid comment.
 * Returns an empty array for empty or whitespace-only input.
 */
function splitByPid(fnBody: string): ParsedSegment[] {
  if (!fnBody || !fnBody.trim()) return [];

  const segments: ParsedSegment[] = [];
  const pidRe = /^\s*\/\/\s*@pid:(\S+)/;
  const lines = fnBody.split("\n");

  let currentPid = "";
  let currentStart = 0;
  let currentLines: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const match = line.match(pidRe);
    if (match) {
      // Save previous segment if we were tracking one
      if (currentPid && currentLines.length > 0) {
        segments.push({
          pid: currentPid,
          startLine: currentStart,
          endLine: i - 1,
          code: currentLines.join("\n").trimEnd(),
          type: detectSegmentType(currentLines),
        });
      }
      currentPid = match[1];
      currentStart = i;
      currentLines = [];
    } else {
      currentLines.push(line);
    }
  }
  // Save last segment
  if (currentPid && currentLines.length > 0) {
    segments.push({
      pid: currentPid,
      startLine: currentStart,
      endLine: lines.length - 1,
      code: currentLines.join("\n").trimEnd(),
      type: detectSegmentType(currentLines),
    });
  }

  return segments;
}

// ---------------------------------------------------------------------------
// detectSegmentType – classify a segment from its first non-empty line
// ---------------------------------------------------------------------------

/** Determine segment type from first non-empty line of code. */
function detectSegmentType(
  lines: string[],
): "fn" | "code-block" | "condition" | "return" {
  const firstLine = lines.find((l) => l.trim()) || "";
  const trimmed = firstLine.trim();

  if (trimmed.startsWith("fn ")) return "fn";
  if (
    trimmed.startsWith("if ") ||
    trimmed.startsWith("else if ") ||
    trimmed === "else" ||
    trimmed.startsWith("else {")
  )
    return "condition";
  if (trimmed.startsWith("return ") || trimmed === "return;" || trimmed === "return")
    return "return";
  return "code-block";
}

// ---------------------------------------------------------------------------
// parseConditionBranches – extract branch definitions from condition code
// ---------------------------------------------------------------------------

/**
 * Parse a condition segment's raw code to extract branch definitions.
 * Uses regex instead of brittle string slicing.
 */
function parseConditionBranches(code: string): BranchDef[] {
  const branches: BranchDef[] = [];
  const lines = code.split("\n");

  let currentType: "if" | "elif" | "else" = "if";
  let currentCond = "";
  let branchIdx = 0;

  const ifRe = /^if\s+(.*?)\s*\{?\s*$/;
  const elifRe = /^(?:\}\s*)?else\s+if\s+(.*?)\s*\{?\s*$/;
  const elseRe = /^(?:\}\s*)?else\s*\{?\s*$/;

  for (const line of lines) {
    const trimmed = line.trim();

    let match: RegExpMatchArray | null;

    if ((match = trimmed.match(ifRe))) {
      currentType = "if";
      currentCond = match[1].trim();
    } else if ((match = trimmed.match(elifRe))) {
      // Save previous if/elif
      branches.push({
        id: `br${branchIdx++}`,
        type: currentType,
        condition: currentCond,
      });
      currentType = "elif";
      currentCond = match[1].trim();
    } else if (elseRe.test(trimmed)) {
      // Save previous if/elif
      branches.push({
        id: `br${branchIdx++}`,
        type: currentType,
        condition: currentCond,
      });
      currentType = "else";
      currentCond = "";
    }
  }
  // Save last branch
  branches.push({
    id: `br${branchIdx}`,
    type: currentType,
    condition: currentCond,
  });

  return branches;
}

// ---------------------------------------------------------------------------
// computeLineDepths – per-line brace depth for the entire source
// ---------------------------------------------------------------------------

/**
 * Compute the brace depth at each line of the original fnBody.
 * PID comment lines are skipped (they don't contribute braces).
 * The depth at line N is the depth AFTER processing that line.
 */
function computeLineDepths(fnBody: string): number[] {
  const lines = fnBody.split("\n");
  const depths: number[] = [];
  let depth = 0;
  const pidRe = /^\s*\/\/\s*@pid:/;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // PID comment lines don't contribute to brace depth
    if (!pidRe.test(line)) {
      for (const ch of line) {
        if (ch === "{") depth++;
        if (ch === "}") depth--;
      }
    }
    depths.push(depth);
  }

  return depths;
}

// ---------------------------------------------------------------------------
// assignBranchContext – determine condition/branch membership for segments
// ---------------------------------------------------------------------------

/**
 * Iterate through segments tracking brace depth to determine which
 * condition (if any) each segment falls inside and which branch index
 * it belongs to. Handles:
 *
 *   - Simple linear code (no condition)
 *   - If/elif/else branches detected via code patterns
 *   - Branch transitions within a single segment (the `} else {` pattern)
 *   - Nested conditions
 *
 * Uses the per-line depth array from the original fnBody to determine
 * when a condition's brace scope opens and closes.
 */
function assignBranchContext(
  segments: ParsedSegment[],
  lineDepths: number[],
): Map<number, BranchContext> {
  const context = new Map<number, BranchContext>();

  if (segments.length === 0) return context;

  interface StackEntry {
    pid: string;
    baseDepth: number;
    branchCount: number;
    currentBranch: number;
    /** True when the immediately preceding segment contained a `} else {`
     *  pattern (the transition happens just after that segment, so the
     *  NEXT segment arriving here should advance the branch index). */
    pendingTransition: boolean;
  }

  const conditionStack: StackEntry[] = [];

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    const startDepth = lineDepths[seg.startLine] ?? 0;
    const trimmed = seg.code.trim();

    // ------------------------------------------------------------------
    // 1. Close conditions whose brace scope ended (depth returned to their base)
    // ------------------------------------------------------------------
    while (
      conditionStack.length > 0 &&
      startDepth <= conditionStack[conditionStack.length - 1].baseDepth
    ) {
      conditionStack.pop();
    }

    // ------------------------------------------------------------------
    // 2. Determine if this segment is a branch continuation of an open condition
    // ------------------------------------------------------------------
    let isBranchContinuation = false;

    if (conditionStack.length > 0) {
      const cond = conditionStack[conditionStack.length - 1];

      // 2a. Segment starts with `} else {` or `} else if` — transition via closing + opening brace
      if (/^\}\s*else\s*\{/.test(trimmed) || /^\}\s*else\s+if\b/.test(trimmed)) {
        cond.currentBranch++;
        cond.pendingTransition = false;
        isBranchContinuation = true;
      }
      // 2b. Segment starts with bare `else {` or `else if` (no leading `}`) —
      //     previous branch's closing `}` was in a preceding segment
      else if (/^else\s*\{/.test(trimmed) || /^else\s+if\b/.test(trimmed)) {
        cond.currentBranch++;
        cond.pendingTransition = false;
        isBranchContinuation = true;
      }
      // 2c. Previous segment indicated a pending transition after a `} else {`
      //     pattern that was NOT at the start of that segment.
      else if (cond.pendingTransition) {
        cond.currentBranch++;
        cond.pendingTransition = false;
      }
      // 2d. Check if THIS segment's code CONTAINS `} else {` or `} else if`
      //     but NOT at the start. The transition happens at the end of this
      //     segment, so the next segment should advance branch.
      else if (
        /\}\s*else\s*\{/.test(seg.code) ||
        /\}\s*else\s+if\b/.test(seg.code)
      ) {
        cond.pendingTransition = true;
      }

      // Record branch context for segments inside a condition
      context.set(i, {
        conditionPid: cond.pid,
        branchIndex: cond.currentBranch,
      });
    }

    // ------------------------------------------------------------------
    // 3. Push new condition nodes onto the stack.
    //    Branch continuations (else/elif) do NOT create new stack entries.
    // ------------------------------------------------------------------
    if (seg.type === "condition" && !isBranchContinuation) {
      const branches = parseConditionBranches(seg.code);
      conditionStack.push({
        pid: seg.pid,
        baseDepth: startDepth,
        branchCount: branches.length,
        currentBranch: 0,
        pendingTransition: false,
      });
    }
  }

  return context;
}

// ---------------------------------------------------------------------------
// collectConditionFullCode – gather all code across segments inside a condition
// ---------------------------------------------------------------------------

/**
 * Combine the condition segment's own code with code from all subsequent
 * segments that fall within the condition's brace scope. This gives
 * `parseConditionBranches` the full picture including elif/else keywords
 * that live in separate segments.
 */
function collectConditionBranches(
  segments: ParsedSegment[],
  condIdx: number,
  branchCtx: Map<number, BranchContext>,
): BranchDef[] {
  // Start with the condition segment's own code
  const codeParts: string[] = [segments[condIdx].code];
  const condPid = segments[condIdx].pid;

  // Append code from all subsequent segments inside this condition
  for (let j = condIdx + 1; j < segments.length; j++) {
    const ctx = branchCtx.get(j);
    if (!ctx || ctx.conditionPid !== condPid) break;
    codeParts.push(segments[j].code);
  }

  // Now parse branches from the combined code
  const combinedCode = codeParts.join("\n");
  return parseConditionBranches(combinedCode);
}

// ---------------------------------------------------------------------------
// parseCodeFlowGraph – main entry point
// ---------------------------------------------------------------------------

/**
 * Parse an OmniView fn body string into code flow elements and wires.
 *
 * The fnBody should contain // @pid:xxx annotation comments that define
 * node boundaries. The parser splits on those boundaries, determines the
 * control flow structure (conditions, branches), and returns the equivalent
 * elements and wires that the GraphCompiler would have consumed.
 *
 * @param fnBody  Raw source text (e.g. inside a function body) with // @pid:xxx comments.
 * @returns       Elements and wires for the code flow graph.
 */
export function parseCodeFlowGraph(
  fnBody: string,
): { elements: CodeblockElementData[]; wires: WireData[] } {
  // Defensive: empty fnBody
  if (!fnBody || !fnBody.trim()) {
    return { elements: [], wires: [] };
  }

  const segments = splitByPid(fnBody);
  if (segments.length === 0) return { elements: [], wires: [] };

  const lineDepths = computeLineDepths(fnBody);
  const branchCtx = assignBranchContext(segments, lineDepths);

  // -----------------------------------------------------------------------
  // Build elements with full immutability — no mutations after construction
  // -----------------------------------------------------------------------
  const elements: CodeblockElementData[] = [];
  const elemById = new Map<string, CodeblockElementData>();

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    let props: Record<string, unknown> = {};
    let source = "";

    if (seg.type === "fn") {
      const sigMatch = seg.code.match(/fn\s+(\w+)\s*\(([^)]*)\)/);
      props = {
        name: sigMatch?.[1] || "unnamed",
        params: sigMatch?.[2] || "",
      };
    } else if (seg.type === "condition") {
      // Use combined code from all segments inside the condition so that
      // elif/else keywords in subsequent segments are picked up.
      const branches = collectConditionBranches(segments, i, branchCtx);
      props = { branches };
    } else if (seg.type === "code-block") {
      source = seg.code;
    } else if (seg.type === "return") {
      // The return expression is always on the first line (trailing } may
      // follow on subsequent lines if this is the last segment in a block).
      const retLine = seg.code.split("\n")[0];
      const retMatch = retLine.match(/return\s+(.*?)\s*;?\s*$/);
      source = retMatch?.[1] || "";
    }

    const el: CodeblockElementData = {
      id: seg.pid,
      nodeType: "codeblock",
      tag: seg.type,
      parentId: null,
      props,
      source,
    };

    elements.push(el);
    elemById.set(el.id, el);
  }

  // -----------------------------------------------------------------------
  // Reconstruct wires using branch context
  // -----------------------------------------------------------------------
  const wires: WireData[] = [];
  const wiredFrom = new Set<string>(); // track which ports have been wired

  /**
   * Helper: add a wire if it doesn't already exist (deduplication).
   */
  function wire(fromId: string, fromPort: string, toId: string, toPort: string): void {
    const key = `${fromId}:${fromPort}->${toId}:${toPort}`;
    if (wiredFrom.has(key)) return;
    wiredFrom.add(key);
    wires.push({
      id: uid("wire"),
      fromId,
      fromPort,
      toId,
      toPort,
    });
  }

  // Track the previous segment index per branch chain, for flow-out wiring
  // For top-level segments (inside no condition): previousIndex = -1 initially
  // For condition-embedded segments: previousIndex per (conditionPid, branchIndex)
  let topLevelPrev = -1;
  const branchPrev = new Map<string, number>(); // key: "conditionPid:branchIndex" → seg index

  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    const ctx = branchCtx.get(i);

    if (ctx) {
      // Inside a condition
      const key = `${ctx.conditionPid}:${ctx.branchIndex}`;
      const prev = branchPrev.get(key);

      if (prev === undefined) {
        // First segment in this branch — wire from condition's branch port
        wire(ctx.conditionPid, `br${ctx.branchIndex}`, seg.pid, "flow-in");
      } else {
        // Subsequent segment in the same branch — wire from previous segment
        wire(segments[prev].pid, "flow-out", seg.pid, "flow-in");
      }

      branchPrev.set(key, i);
    } else if (seg.type !== "condition") {
      // Top-level segment (not inside any condition, not a condition itself)
      // Wire linearly
      if (topLevelPrev >= 0) {
        wire(segments[topLevelPrev].pid, "flow-out", seg.pid, "flow-in");
      }
      topLevelPrev = i;
    }
  }

  // For conditions, wire flow-out to the first post-condition segment
  for (let i = 0; i < segments.length; i++) {
    if (segments[i].type !== "condition") continue;

    // Find the first segment after this condition's brace scope
    const condSeg = segments[i];
    const condBaseDepth = lineDepths[condSeg.startLine] ?? 0;

    for (let j = i + 1; j < segments.length; j++) {
      const nextSeg = segments[j];
      const nextDepth = lineDepths[nextSeg.startLine] ?? 0;

      if (nextDepth <= condBaseDepth) {
        // This segment is outside the condition — wire flow-out
        // But only if this segment isn't already wired as a branch successor
        const ctx = branchCtx.get(j);
        if (!ctx || ctx.conditionPid !== condSeg.pid) {
          wire(condSeg.pid, "flow-out", nextSeg.pid, "flow-in");
        }
        break;
      }
    }
  }

  return { elements, wires };
}
