// poiesis/src/services/codegen/graph-parser.ts
// Parse an OmniView fn body string → code flow graph (nodes + wires).
// Reverse of GraphCompiler. Uses // @pid:xxx and // @<type> annotations to
// identify nodes.  When // @<type> annotations are present, auto-generated
// PIDs are created so that hand-written .omni source is immediately usable
// in the canvas.
//
// Handles proper branch wiring for condition nodes via brace-depth analysis.

import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { uid } from "../../core/utils/id";
import { parseOps } from "./code-op-parser";
import {
  isAnyAnnotation,
  parsePidAnnotation,
  parseTypeAnnotation,
  emitPidAnnotation,
} from "./annotation-registry";

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

/**
 * Segment type override map for // @<type> annotations.
 * Maps annotation node types to segment types.
 * @elif and @else are mapped to "code-block" so they are handled as
 * branch continuations rather than new condition nodes.
 */
const TYPE_TO_SEGMENT: Record<string, "fn" | "code-block" | "condition" | "return"> = {
  if: "condition",
  elif: "code-block",
  else: "code-block",
  "code-block": "code-block",
  return: "return",
  for: "code-block",
  while: "code-block",
};

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
// splitByPid – split fn body into segments at // @pid:xxx or // @<type>
// comment boundaries
// ---------------------------------------------------------------------------

/**
 * Split fn body into segments delimited by // @pid:xxx or // @<type> comments.
 * Each segment starts at an annotation comment and ends before the next one.
 * For // @<type> annotations, a PID is auto-generated via uid().
 *
 * Returns segments and a pidMap that maps "nodeType:lineNumber" → generated pid.
 * The pidMap lets callers (e.g. normalizeAnnotations) replace // @<type> lines
 * with // @pid:xxx in saved source.
 */
function splitByPid(
  fnBody: string,
): { segments: ParsedSegment[]; pidMap: Map<string, string> } {
  const empty = { segments: [], pidMap: new Map<string, string>() };
  if (!fnBody || !fnBody.trim()) return empty;

  const segments: ParsedSegment[] = [];
  const pidMap = new Map<string, string>();
  const lines = fnBody.split("\n");

  let currentPid = "";
  let currentStart = 0;
  let currentLines: string[] = [];
  /** Non-null when the most recent annotation was a // @<type> annotation. */
  let lastTypeAnnotation: string | null = null;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (isAnyAnnotation(line)) {
      // Save previous segment
      if (currentPid && currentLines.length > 0) {
        segments.push({
          pid: currentPid,
          startLine: currentStart,
          endLine: i - 1,
          code: currentLines.join("\n").trimEnd(),
          type: lastTypeAnnotation
            ? (TYPE_TO_SEGMENT[lastTypeAnnotation] ?? detectSegmentType(currentLines))
            : detectSegmentType(currentLines),
        });
      }

      // Determine pid for the new segment
      const pidResult = parsePidAnnotation(line);
      const typeResult = parseTypeAnnotation(line);

      if (pidResult) {
        currentPid = pidResult.pid;
        lastTypeAnnotation = null;
      } else if (typeResult) {
        // Auto-generate PID for // @<type> annotations
        currentPid = uid(typeResult.nodeType);
        pidMap.set(`${typeResult.nodeType}:${i}`, currentPid);
        lastTypeAnnotation = typeResult.nodeType;
      } else {
        // Unknown annotation — treat as part of code
        currentLines.push(line);
        continue;
      }

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
      type: lastTypeAnnotation
        ? (TYPE_TO_SEGMENT[lastTypeAnnotation] ?? detectSegmentType(currentLines))
        : detectSegmentType(currentLines),
    });
  }

  return { segments, pidMap };
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

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // Annotation lines don't contribute to brace depth
    if (!isAnyAnnotation(line)) {
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

    // Detect if this segment is an elif/else continuation.
    // Handles both `} else {` (legacy @pid style) and bare `else {` / `else if`
    // (new @type style where the closing `}` is in the preceding segment).
    const isElifElseCode =
      /^\}\s*else\s*\{/.test(trimmed) ||
      /^\}\s*else\s+if\b/.test(trimmed) ||
      /^else\s*\{/.test(trimmed) ||
      /^else\s+if\b/.test(trimmed);

    // ------------------------------------------------------------------
    // 1. Close conditions whose brace scope ended (depth returned to their base)
    //    EXCEPT if this segment is an elif/else of the top condition — in that
    //    case the depth match is expected (the annotation sits outside braces).
    // ------------------------------------------------------------------
    while (
      conditionStack.length > 0 &&
      startDepth <= conditionStack[conditionStack.length - 1].baseDepth
    ) {
      // For @type-style elif/else, the annotation line is at the same depth as
      // the condition's base (the `}` closed before the annotation).  Don't pop
      // in that case — the elif/else is a branch continuation.
      const top = conditionStack[conditionStack.length - 1];
      if (
        isElifElseCode &&
        startDepth === top.baseDepth
      ) {
        break;
      }
      conditionStack.pop();
    }

    // ------------------------------------------------------------------
    // 2. Determine if this segment is a branch continuation of an open condition
    // ------------------------------------------------------------------
    let isBranchContinuation = false;

    if (conditionStack.length > 0) {
      const cond = conditionStack[conditionStack.length - 1];

      // 2a. @pid-style: segment starts with `} else {` or `} else if`
      if (/^\}\s*else\s*\{/.test(trimmed) || /^\}\s*else\s+if\b/.test(trimmed)) {
        cond.currentBranch++;
        cond.pendingTransition = false;
        isBranchContinuation = true;
      }
      // 2b. @type-style elif/else (no leading `}`)
      else if (/^else\s*\{/.test(trimmed) || /^else\s+if\b/.test(trimmed)) {
        cond.currentBranch++;
        cond.pendingTransition = false;
        isBranchContinuation = true;
      }
      // 2c. Previous segment indicated a pending transition
      else if (cond.pendingTransition) {
        cond.currentBranch++;
        cond.pendingTransition = false;
      }
      // 2d. This segment's code contains `} else {` or `} else if` inside (not at start)
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
 * node boundaries.  // @<type> annotations (e.g. // @if, // @return) are
 * also supported — auto-generated PIDs are assigned via uid().
 *
 * @param fnBody  Raw source text (e.g. inside a function body).
 * @returns       Elements, wires, and a pidMap mapping "nodeType:lineNumber"
 *                to auto-generated pid (for use by normalizeAnnotations).
 */
export function parseCodeFlowGraph(
  fnBody: string,
): { elements: CodeblockElementData[]; wires: WireData[]; pidMap: Map<string, string> } {
  // Defensive: empty fnBody
  if (!fnBody || !fnBody.trim()) {
    return { elements: [], wires: [], pidMap: new Map() };
  }

  const { segments, pidMap } = splitByPid(fnBody);
  if (segments.length === 0) return { elements: [], wires: [], pidMap };

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
      // Capture fn name, params, and optional return type (-> Type).
      // The fn body may contain the opening brace after the signature.
      const sigMatch = seg.code.match(/fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*(\w+))?/);
      props = {
        name: sigMatch?.[1] || "unnamed",
        params: sigMatch?.[2] || "",
        returnType: sigMatch?.[3] || "",
      };
    } else if (seg.type === "condition") {
      // Use combined code from all segments inside the condition so that
      // elif/else keywords in subsequent segments are picked up.
      const branches = collectConditionBranches(segments, i, branchCtx);
      props = { branches };
    } else if (seg.type === "code-block") {
      source = seg.code;
      // Try parsing source into structured operations
      let operations;
      try {
        operations = parseOps(source);
      } catch {
        operations = undefined; // keep as raw source only
      }
      // Store operations for later use
      (seg as any)._operations = operations;
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
      ...((seg as any)._operations && (seg as any)._operations.length > 0 ? { operations: (seg as any)._operations } : {}),
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
        // This segment is at or outside the condition's base depth.
        // Wire flow-out only if it isn't still part of the same condition
        // (e.g. an @elif/@else segment at the same brace depth).
        const ctx = branchCtx.get(j);
        if (!ctx || ctx.conditionPid !== condSeg.pid) {
          wire(condSeg.pid, "flow-out", nextSeg.pid, "flow-in");
          break;
        }
        // Still inside the same condition (elif/else at same depth) — keep looking
      }
    }
  }

  return { elements, wires, pidMap };
}

// ---------------------------------------------------------------------------
// normalizeAnnotations – convert // @<type> to // @pid:xxx in source
// ---------------------------------------------------------------------------

/**
 * Convert // @<type> annotations to // @pid:xxx in fn body source.
 * Called during save to replace manual type annotations with editor-generated
 * PIDs so the source is stable for round-trip editing.
 *
 * The pidMap should be the one returned by parseCodeFlowGraph, which maps
 * "nodeType:lineNumber" → auto-generated pid for each // @<type> line.
 *
 * @param fnBody  Raw fn body source with // @<type> or // @pid annotations.
 * @param pidMap  Map from "nodeType:lineNumber" to pid, as returned by
 *                parseCodeFlowGraph.
 * @returns       fnBody with // @<type> replaced by // @pid:xxx.
 */
export function normalizeAnnotations(
  fnBody: string,
  pidMap: Map<string, string>,
): string {
  const lines = fnBody.split("\n");
  const result: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const typeResult = parseTypeAnnotation(line);
    if (typeResult) {
      const key = `${typeResult.nodeType}:${i}`;
      const pid = pidMap.get(key);
      if (pid) {
        result.push(emitPidAnnotation(pid));
        continue;
      }
    }
    result.push(line);
  }

  return result.join("\n");
}
