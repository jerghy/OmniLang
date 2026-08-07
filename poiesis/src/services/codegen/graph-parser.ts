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
import { formatSyntaxNode, type SyntaxNodeProps } from "./syntax-nodes";
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
  type: "fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call" | "while" | "for-in";
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

type SegmentType = "fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call" | "while" | "for-in";

/** Determine segment type from first non-empty line of code. */
function detectSegmentType(
  lines: string[],
): SegmentType {
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
  // 多语句段 → code-block：首行是完整语句（以 ; 结尾）且后续还有非结构行时，
  // 段内含多条语句——归为 code-block 原样保真。否则单语句节点会吞并后续语句
  // （assign value 合入 "return" 等 → 语义丢失 + 编译-再解析 fixpoint 破坏；
  // 该形态来自 code-block 回退源的原始文本重解析，T3 G1 实证）。
  // 置于 return/单语句检查之前：以 `return x ;` 开头的原始多语句材料同样保真
  // （return 为终态，编译产物无后继行；此处只命中原始回退文本）。
  // 结构尾行（} / } else / else {）除外：条件链挂载的语句段尾部保持单语句分类。
  // 首行以 { 结尾的块头（if/while/for-in）与无分号续行（x = a +\n b;）不命中。
  if (
    /;\s*$/.test(trimmed) &&
    lines.slice(1).some((l) => l.trim() && !(/^\}\s*else/.test(l.trim()) || /^else\s*(if\b|\{)/.test(l.trim()) || l.trim() === "}"))
  )
    return "code-block";
  if (trimmed.startsWith("return ") || trimmed === "return;" || trimmed === "return")
    return "return";
  // OmniLang 风格循环段头（无括号）：`while <expr> {` / `for <ident> in <expr> {`
  if (/^while\s+/.test(trimmed)) return "while";
  if (/^for\s+[a-zA-Z_]\w*\s+in\s+/.test(trimmed)) return "for-in";
  // Loop headers stay code-blocks (they look like call prefixes: `for (`).
  // Mirrors TYPE_TO_SEGMENT: for/while → code-block.
  // C 风格防御规则（不匹配 OmniLang 语法），必须位于 OmniLang 风格规则之后。
  if (/^(for|while)\s*\(/.test(trimmed)) return "code-block";
  if (/^let\s+(mut\s+)?[a-zA-Z_]\w*/.test(trimmed)) return "let";
  if (trimmed.startsWith("emit ")) return "call";
  // Tokenizer-spaced text ("TodoStore . add ( ) ;") must still match —
  // allow optional whitespace around the ".".
  if (/^[A-Z]\w*\s*\.\s*[a-zA-Z_]\w*\s*\(/.test(trimmed)) return "store-call";
  if (/^[a-zA-Z_]\w*(?:\s*\.\s*[a-zA-Z_]\w*)?\s*\(/.test(trimmed)) return "call";
  if (/^[a-zA-Z_]\w*(?:\s*\.\s*[a-zA-Z_]\w*)?\s*(?:=|\+=|-=|\*=|\/=|%=)\s*/.test(trimmed)) return "assign";
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
      currentCond = compactSpacedMembers(match[1].trim());
    } else if ((match = trimmed.match(elifRe))) {
      // Save previous if/elif
      branches.push({
        id: `br${branchIdx++}`,
        type: currentType,
        condition: currentCond,
      });
      currentType = "elif";
      currentCond = compactSpacedMembers(match[1].trim());
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

      // 2d. 段内残余 `} else {` / `} else if` 过渡（段首过渡已由 2a/2b 处理，用 isElifElseCode 排除防重复推进）。
      // else/else if 行常与前段收尾 `}` 同段挂载（发射侧 `}\nelse {` 跨段），且 2c 消费前段过渡时
      // 本段尾部可能还挂着下一分支过渡（`}\nelse {` 在 return 段尾）——若 2d 挂在 else-if 链上会被
      // 2c 短路漏检，下一分支并入当前分支（分支体错位/丢失，E2E 缺陷 2 的末分支 wire 根因）。
      if (
        !isElifElseCode &&
        (/\}\s*else\s*\{/.test(seg.code) || /\}\s*else\s+if\b/.test(seg.code))
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
    // 3. Push new block-header nodes (condition / loop) onto the stack.
    //    - condition：else/elif 延续（isBranchContinuation）不压栈；
    //    - loop（while/for-in）：单分支（branchCount 1）、无 elif 语义，一律压栈——
    //      即使它作为 else 分支体首段消费了 2c 的 pendingTransition
    //      （`} else {` 尾行过渡），它仍是块头，body 必须挂 br0
    //      （否则 loop-in-else 的 body 段被续接为 flow-out，分支体错位）。
    // ------------------------------------------------------------------
    if (
      seg.type === "while" ||
      seg.type === "for-in" ||
      (seg.type === "condition" && !isBranchContinuation)
    ) {
      const branches =
        seg.type === "condition"
          ? parseConditionBranches(seg.code)
          : [{ id: "br0", type: "if" as const, condition: "" }];
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
      // Note: source text from the omni tokenizer may carry spaces between
      // tokens (e.g. "fn newAction ( )") — params are trimmed.
      const sigMatch = seg.code.match(/fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*(\w+))?/);
      props = {
        name: sigMatch?.[1] || "unnamed",
        params: (sigMatch?.[2] || "").trim(),
        returnType: (sigMatch?.[3] || "").trim(),
      };
    } else if (seg.type === "condition") {
      // Use combined code from all segments inside the condition so that
      // elif/else keywords in subsequent segments are picked up.
      const branches = collectConditionBranches(segments, i, branchCtx);
      props = { branches };
    } else if (seg.type === "code-block") {
      // 原样保留段文本（含手写首行缩进——graph-parser.test.ts 实证 "  // 步骤一"）。
      // 首行 tokenizer 空格伪影（"\n " 归一化）由发射侧首行 trimStart 消化（见
      // graph-compiler emitNode code-block 分支），保证 parse(compile(g)) fixpoint。
      // 注意：不再在此挂载 parseOps 结构化 operations——CodeCard 编辑器时代遗留，
      // 当前无任何消费方（canvas 为语法节点模型），且会把回退原始文本改写为
      // ops 形态（"return ;" 误解析为 return(";") → 发射 "return ;;"），
      // 破坏 code-block 原样保真与 fixpoint（T3 G1 实证）。
      source = seg.code;
    } else if (
      seg.type === "let" ||
      seg.type === "assign" ||
      seg.type === "store-call" ||
      seg.type === "call" ||
      seg.type === "while" ||
      seg.type === "for-in"
    ) {
      const parsed = parseSyntaxProps(seg.type, seg.code);
      props = parsed;
      // Only rewrite the source when props parsed successfully; on failure
      // (empty props) keep the original text so nothing is lost or replaced
      // with placeholder defaults. WASM validation reports the loud error.
      source = Object.keys(parsed).length > 0 ? formatSyntaxNode(seg.type, parsed as SyntaxNodeProps) : seg.code;
    } else if (seg.type === "return") {
      // The return expression is always on the first line (trailing } may
      // follow on subsequent lines if this is the last segment in a block).
      const retLine = seg.code.split("\n")[0];
      const retMatch = retLine.match(/return\s+(.*?)\s*;?\s*$/);
      source = retMatch ? compactSpacedMembers(retMatch[1]) : "";
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
    } else {
      // 顶层段线形重建（含 condition 段）：前驱 flow-out → 本段 flow-in。
      // condition 段必须同样入链——此前 `seg.type !== "condition"` 跳过 condition，
      // 中链 condition 永远无 flow-in wire（重载后成孤岛），emit 从 fn 链不可达，
      // 下次保存条件结构整体丢失（E2E 缺陷 2 的入链 wire 根因）。
      // condition 的 flow-out 后继由下方续接循环处理；此处 topLevelPrev 也推进到
      // condition，后续顶层段经线形通道同样指向 condition.flow-out（与续接循环同一
      // wire，wiredFrom 按 from:fromPort->to:toPort 去重）。
      if (topLevelPrev >= 0) {
        wire(segments[topLevelPrev].pid, "flow-out", seg.pid, "flow-in");
      }
      topLevelPrev = i;
    }
  }

  // 块头段（condition / while / for-in）flow-out 续接：
  // 作用域之后首个段接到块头 flow-out（与 condition 对称——loop 的 body 经 br0
  // 进入，循环结束经 flow-out 跳出作用域继续线形链）。
  for (let i = 0; i < segments.length; i++) {
    const segType = segments[i].type;
    if (segType !== "condition" && segType !== "while" && segType !== "for-in") continue;

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

// ---------------------------------------------------------------------------
// parseSyntaxProps – extract syntax node props from code text
// ---------------------------------------------------------------------------

/** 引号感知的 member 空格还原：tokenizer 空格化的 "TodoStore . items" → "TodoStore.items"
 *  （round-trip 保存不退化——Task 12 E2E 实测：磁盘 `TodoStore.items` 重读后变成带空格
 *  表达式，再保存写回坏代码）。字符串字面量内（"a . b"）原样保留。
 *  多级链（"TodoStore . items . length"）单遍收敛：ident 后贪婪吞并所有 " . ident" 段。 */
function compactSpacedMembers(s: string): string {
  let out = "";
  let i = 0;
  let quote: string | null = null;
  while (i < s.length) {
    const ch = s[i]!;
    if (quote) {
      out += ch;
      if (ch === "\\" && i + 1 < s.length) { out += s[i + 1]!; i += 2; continue; }
      if (ch === quote) quote = null;
      i++;
      continue;
    }
    if (ch === "'" || ch === '"') { quote = ch; out += ch; i++; continue; }
    // 多级 member 链：ident（. ident）+——吞并所有空格点段
    const m = s.slice(i).match(/^([A-Za-z_]\w*)(?:\s*\.\s*[A-Za-z_]\w*)+/);
    if (m) {
      out += m[1]! + m[0]!.slice(m[1]!.length).replace(/\s*\.\s*/g, ".");
      i += m[0].length;
      continue;
    }
    out += ch;
    i++;
  }
  return out;
}

/**
 * Extract props for a syntax node from its code text.
 * Inverse of formatSyntaxNode (round-trip stable): every emitted line
 * parses back into the same props, modulo trailing semicolons.
 * Multi-line statements are joined with spaces first (mirrors the omni
 * tokenizer's whitespace normalization) so continuation lines are never
 * truncated or silently dropped.
 */
function parseSyntaxProps(
  type: "let" | "assign" | "store-call" | "call" | "while" | "for-in",
  code: string,
): Record<string, unknown> {
  // 语句段代码可能携带结构尾行：发射侧 `}\nelse if ... {` 把条件块收尾 `}` 与下一分支头
  // 挂在语句段尾部（同 return 段"表达式只在首行"注释的处理模式）——props 解析只应看到
  // 语句本身，截到首个结构行（行首 `}` / `} else` / `else if` / `else {`）为止。
  // 多行语句（续行）不受影响：续行不以结构行开头，join 归一化照旧。
  const lines = code.split("\n");
  const stmtEnd = lines.findIndex((l) => {
    const t = l.trim();
    return t === "}" || /^\}\s*else/.test(t) || /^else\s*(if\b|\{)/.test(t);
  });
  const stmtCode = stmtEnd === -1 ? code : lines.slice(0, stmtEnd).join("\n");
  const t = stmtCode.split("\n").map((l) => l.trim()).filter(Boolean).join(" ").trim();

  switch (type) {
    case "let": {
      // The value group is optional: `let total;` (no initializer, emitted when
      // the value is cleared) must still round-trip without losing the name.
      const m = t.match(/^let\s+(mut\s+)?([a-zA-Z_]\w*)(?::\s*([\w<>, ]+?))?\s*(?:=\s*(.*?))?\s*;?\s*$/);
      if (!m) return { name: "x", mutable: false };
      return { name: m[2], mutable: !!m[1], varType: m[3] || undefined, value: m[4] === undefined ? undefined : compactSpacedMembers(m[4]) };
    }
    case "assign": {
      // Allow optional whitespace around the "." — source from the omni
      // tokenizer may be spaced ("newSignal2 = newSignal2 + 1 ;").
      const m = t.match(/^([a-zA-Z_]\w*(?:\s*\.\s*[a-zA-Z_]\w*)?)\s*(=|\+=|-=|\*=|\/=|%=)\s*(.*?);?\s*$/);
      if (!m) return {};
      return { target: compactSpacedMembers(m[1].trim()), operator: m[2], value: compactSpacedMembers(m[3].trim()) };
    }
    case "store-call": {
      const m = t.match(/^([A-Z]\w*)\s*\.\s*([a-zA-Z_]\w*)\s*\((.*?)\)\s*;?\s*$/);
      if (!m) return {};
      return { storeName: m[1], action: m[2], args: compactSpacedMembers(m[3].trim()) };
    }
    case "call": {
      const m = t.match(/^(emit\s+)?([a-zA-Z_]\w*(?:\s*\.\s*[a-zA-Z_]\w*)?)\s*\((.*?)\)\s*;?\s*$/);
      if (!m) return { fnName: "fn", args: "" };
      return { emit: !!m[1], fnName: compactSpacedMembers(m[2]), args: compactSpacedMembers(m[3].trim()) };
    }
    case "while": {
      // 段头形如 `while n < 10 {`（OmniLang 风格，无括号），提取条件表达式。
      const m = t.match(/^while\s+(.*?)\s*\{?\s*$/);
      return m ? { condition: compactSpacedMembers(m[1].trim()) } : {};
    }
    case "for-in": {
      // 段头形如 `for item in items {`，提取循环变量与可迭代表达式。
      const m = t.match(/^for\s+([a-zA-Z_]\w*)\s+in\s+(.*?)\s*\{?\s*$/);
      return m ? { varName: m[1], iterable: compactSpacedMembers(m[2].trim()) } : {};
    }
    default:
      return {};
  }
}

// ---------------------------------------------------------------------------
// mergeCodeFlowGraphs – 合并多文件的 codeFlowGraphs（fn 名 → 图）
// ---------------------------------------------------------------------------

/** 合并多文件 codeFlowGraphs（fn 名 → 图），同名后者覆盖（跨文件同名 fn 为既有限制）。 */
export function mergeCodeFlowGraphs(
  graphs: Array<Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> | undefined>,
): Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> {
  const out: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> = {};
  for (const g of graphs) {
    if (!g) continue;
    for (const [key, value] of Object.entries(g)) out[key] = value;
  }
  return out;
}
