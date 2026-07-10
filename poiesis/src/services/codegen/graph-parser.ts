// poiesis/src/services/codegen/graph-parser.ts
// Parse an OmniView fn body string → code flow graph (nodes + wires).
// Reverse of GraphCompiler. Uses // @pid:xxx annotations to identify nodes.

import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { uid } from "../../core/utils/id";

interface ParsedSegment {
  pid: string;
  startLine: number;
  endLine: number;
  code: string;     // raw code for this segment (without pid comment)
  type: "fn" | "code-block" | "condition" | "return";
  // For condition segments, store branch info
  branches?: Array<{ id: string; type: "if" | "elif" | "else"; condition: string }>;
  parentPid?: string; // enclosing block's pid (for wire reconstruction)
}

/**
 * Split fn body into segments delimited by // @pid:xxx comments.
 * Each segment starts at a @pid comment and ends before the next @pid comment.
 */
function splitByPid(fnBody: string): ParsedSegment[] {
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
      // Save previous segment
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

/** Determine segment type from first non-empty line of code */
function detectSegmentType(lines: string[]): "fn" | "code-block" | "condition" | "return" {
  const firstLine = lines.find((l) => l.trim()) || "";
  const trimmed = firstLine.trim();

  if (trimmed.startsWith("fn ")) return "fn";
  if (trimmed.startsWith("if ") || trimmed.startsWith("else if ") || trimmed === "else" || trimmed.startsWith("else {")) return "condition";
  if (trimmed.startsWith("return ") || trimmed === "return;" || trimmed === "return") return "return";
  return "code-block";
}

/** Parse condition segment to extract branches */
function parseConditionBranches(code: string): Array<{ id: string; type: "if" | "elif" | "else"; condition: string }> {
  const branches: Array<{ id: string; type: "if" | "elif" | "else"; condition: string }> = [];
  const lines = code.split("\n");

  let currentType: "if" | "elif" | "else" = "if";
  let currentCond = "";
  let branchIdx = 0;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("if ")) {
      currentType = "if";
      currentCond = trimmed.slice(3).replace(/\s*\{$/, "").trim();
    } else if (trimmed.startsWith("else if ")) {
      // Save previous if/elif
      branches.push({ id: `br${branchIdx++}`, type: currentType, condition: currentCond });
      currentType = "elif";
      currentCond = trimmed.slice(8).replace(/\s*\{$/, "").trim();
    } else if (trimmed === "else {" || trimmed === "else") {
      // Save previous if/elif
      branches.push({ id: `br${branchIdx++}`, type: currentType, condition: currentCond });
      currentType = "else";
      currentCond = "";
    }
  }
  // Save last branch
  branches.push({ id: `br${branchIdx}`, type: currentType, condition: currentCond });

  return branches;
}

/**
 * Parse an OmniView fn body string into code flow elements and wires.
 *
 * @param fnBody        Raw fn body source (inside the braces), with // @pid:xxx comments
 * @param componentName Name of the enclosing component (for element ID generation)
 * @returns             Elements and wires for the code flow graph
 */
export function parseCodeFlowGraph(
  fnBody: string,
  componentName: string,
): { elements: CodeblockElementData[]; wires: WireData[] } {
  const segments = splitByPid(fnBody);
  if (segments.length === 0) return { elements: [], wires: [] };

  const elements: CodeblockElementData[] = [];
  const wires: WireData[] = [];

  // Build elements from segments
  for (let i = 0; i < segments.length; i++) {
    const seg = segments[i];
    const el: CodeblockElementData = {
      id: seg.pid,
      nodeType: "codeblock",
      tag: seg.type,
      parentId: null,
      props: {},
      source: "",
    };

    if (seg.type === "fn") {
      // Extract fn name and params from code
      const sigMatch = seg.code.match(/fn\s+(\w+)\s*\(([^)]*)\)/);
      el.props = {
        name: sigMatch?.[1] || "unnamed",
        params: sigMatch?.[2] || "",
      };
      el.source = "";
    } else if (seg.type === "condition") {
      const branches = parseConditionBranches(seg.code);
      el.props = { branches };
      el.source = "";
    } else if (seg.type === "code-block") {
      el.source = seg.code;
    } else if (seg.type === "return") {
      const retMatch = seg.code.match(/return\s+(.+);?/);
      el.source = retMatch?.[1] || "";
    }

    elements.push(el);

    // Wire to next segment (linear chain by default — branches handled below)
    if (i < segments.length - 1) {
      const next = segments[i + 1];
      wires.push({
        id: uid("wire"),
        fromId: seg.pid,
        fromPort: seg.type === "condition" ? "br0" : "flow-out",
        toId: next.pid,
        toPort: "flow-in",
      });
    }
  }

  return { elements, wires };
}
