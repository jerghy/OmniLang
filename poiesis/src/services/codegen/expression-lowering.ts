// editor/client/src/omni-gen/expression-lowering.ts
// Recursive descent builder for code graph elements to OmniLang expressions.
// Maps code graph node tags (literal, add, setVariable, etc.) to OmniLang expression strings.
// Uses reverse wire adjacency to walk backward from consumer ports building expressions.

import type { ElementData, WireData } from "../../core/types";
import type { LoweredCode } from "./types";

/**
 * Build a reverse wire adjacency map.
 * For each (toId:toPort), returns the list of (fromId, fromPort) that feed into it.
 */
function buildReverseWires(
  wires: WireData[],
): Map<string, Array<{ fromId: string; fromPort: string }>> {
  const m = new Map<string, Array<{ fromId: string; fromPort: string }>>();
  for (const w of wires) {
    const key = `${w.toId}:${w.toPort}`;
    let entries = m.get(key);
    if (!entries) {
      entries = [];
      m.set(key, entries);
    }
    entries.push({ fromId: w.fromId, fromPort: w.fromPort });
  }
  return m;
}

/**
 * Lower code graph elements and wires into OmniLang expressions and function bodies.
 *
 * Algorithm:
 * 1. Build reverse wire adjacency map for O(1) port resolution
 * 2. From consumer ports (setVariable.value, return.value), walk backward building
 *    expression strings recursively with cycle detection
 * 3. Cache results in expressions Map to avoid recomputation
 * 4. For customFn: BFS from fnId collecting setVariable/return statements
 */
export function lowerCode(
  elements: ElementData[],
  wires: WireData[],
): LoweredCode {
  const elMap = new Map(elements.map((e) => [e.id, e]));
  const revWires = buildReverseWires(wires);

  const expressions = new Map<string, string>();
  const functions: LoweredCode["functions"] = [];

  // ─── Port resolution helpers ──────────────────────────────────

  /**
   * Resolve the expression for a node's input port.
   * Looks up the reverse wire map and recursively builds the source node's expression.
   * Threads the `visiting` set through to enable cycle detection.
   */
  function inp(nodeId: string, port: string, visiting: Set<string>): string {
    const key = `${nodeId}:${port}`;
    const srcs = revWires.get(key);
    if (!srcs || srcs.length === 0) return "";
    // Use the first wire when multiple wires feed the same port.
    // Multiple-output-to-single-input is a user wiring error; the editor UI
    // should prevent it, but we silently pick the first rather than erroring.
    return buildExpr(srcs[0].fromId, visiting);
  }

  /**
   * Resolve a port expression, trying alternative port names.
   * Threads the `visiting` set for cycle detection.
   */
  function resolvePort(
    nodeId: string,
    primary: string,
    fallback: string,
    visiting: Set<string>,
  ): string {
    return inp(nodeId, primary, visiting) || inp(nodeId, fallback, visiting) || "";
  }

  // ─── Recursive expression builder ────────────────────────────────

  /**
   * Recursively build an OmniLang expression string for a code graph node.
   *
   * @param nodeId  - The element ID to build an expression for
   * @param visiting - Set of node IDs currently in the recursion stack (cycle detection)
   */
  function buildExpr(nodeId: string, visiting: Set<string>): string {
    // Cycle detection --- if this node is already being visited, we have a loop
    if (visiting.has(nodeId)) return "/* cycle */";

    // Cache hit --- expression already computed
    if (expressions.has(nodeId)) return expressions.get(nodeId)!;

    const el = elMap.get(nodeId);
    if (!el) return "";

    // Copy the visiting set so siblings don't interfere with each other's cycle detection
    visiting = new Set(visiting);
    visiting.add(nodeId);

    let result: string;

    switch (el.tag) {
      // ── Terminal / leaf nodes ──────────────────────────────────

      case "literal": {
        const v = el.props.value;
        if (typeof v === "string") {
          result = `"${v}"`;
        } else {
          result = String(v ?? "");
        }
        break;
      }

      case "signal":
      case "getVariable":
        result = (el.props.name as string) || "";
        break;

      // ── Binary arithmetic ops ──────────────────────────────────

      case "add":
        result = `${resolvePort(nodeId, "a", "left", visiting)} + ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      case "sub":
        result = `${resolvePort(nodeId, "a", "left", visiting)} - ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      case "mul":
        result = `${resolvePort(nodeId, "a", "left", visiting)} * ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      case "div":
        result = `${resolvePort(nodeId, "a", "left", visiting)} / ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      // ── Comparison ─────────────────────────────────────────────

      case "compare": {
        const op = (el.props.op as string) || (el.config ? (el.config as Record<string, unknown>).op as string : undefined) || "==";
        result = `${resolvePort(nodeId, "a", "left", visiting)} ${op} ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;
      }

      // ── Logical ops ────────────────────────────────────────────

      case "and":
        result = `${resolvePort(nodeId, "a", "left", visiting)} && ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      case "or":
        result = `${resolvePort(nodeId, "a", "left", visiting)} || ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      // ── Unary ops ──────────────────────────────────────────────

      case "not":
        result = `!${resolvePort(nodeId, "a", "left", visiting)}`;
        break;

      // ── String concat ──────────────────────────────────────────

      case "concat":
        result = `${resolvePort(nodeId, "a", "left", visiting)} + ${resolvePort(nodeId, "b", "right", visiting)}`;
        break;

      // ── Consumer / statement nodes ─────────────────────────────

      case "setVariable":
        result = `${el.props.name as string || ""} = ${inp(nodeId, "value", visiting)};`;
        break;

      case "return":
        result = `return ${inp(nodeId, "value", visiting)};`;
        break;

      // ── External source node ───────────────────────────────────

      case "fn":
        result = (el.props.source as string) || "";
        break;

      // ── Function container (body collected separately) ─────────

      case "customFn":
        result = "";
        break;

      // ── Fallback ───────────────────────────────────────────────

      default:
        result = `/* unknown: ${el.tag} */`;
    }

    expressions.set(nodeId, result);
    return result;
  }

  // ─── Function body collection ────────────────────────────────────

  /**
   * BFS collect setVariable/return statements reachable from a starting node
   * via "body" and "flow" wires.
   */
  function collectStatements(
    nodeId: string,
    collected: string[],
    visited: Set<string>,
  ): void {
    if (visited.has(nodeId)) return;
    visited.add(nodeId);

    const el = elMap.get(nodeId);
    if (!el) return;

    // Collect this node if it's a statement
    if (el.tag === "setVariable" || el.tag === "return") {
      const expr = buildExpr(nodeId, new Set());
      collected.push(expr);
    }

    // Follow "flow" wires to the next statement in the chain
    for (const w of wires) {
      if (w.fromId === nodeId && w.fromPort === "flow") {
        collectStatements(w.toId, collected, visited);
      }
    }
  }

  /**
   * Build a function entry from a customFn element by collecting
   * its body statements.
   */
  function buildFnBody(fnEl: ElementData): void {
    const fnName = (fnEl.props.name as string) || "fn";
    const params = (fnEl.props.params as Array<{ name: string; type: string }>) || [];
    const returnType = (fnEl.props.returnType as string) || "void";

    const bodyParts: string[] = [];
    const visited = new Set<string>();

    // Follow wires from the customFn's "body" port to the first statement
    for (const w of wires) {
      if (w.fromId === fnEl.id && w.fromPort === "body") {
        collectStatements(w.toId, bodyParts, visited);
      }
    }

    functions.push({
      name: fnName,
      params,
      returnType,
      body: bodyParts.join("\n"),
    });
  }

  // ─── Main pipeline ───────────────────────────────────────────────

  // Phase 1: Build expressions for consumer elements.
  // Intermediate nodes (add, literal, signal, etc.) are built on-demand
  // via buildExpr recursion from these consumer nodes.
  for (const el of elements) {
    if (el.nodeType !== "codeblock") continue;
    if (el.tag === "setVariable" || el.tag === "return" || el.tag === "fn") {
      buildExpr(el.id, new Set());
    }
  }

  // Phase 2: Build function bodies from customFn elements.
  // This must run after Phase 1 so that body-statement expressions are cached.
  for (const el of elements) {
    if (el.nodeType === "codeblock" && el.tag === "customFn") {
      buildFnBody(el);
    }
  }

  return {
    expressions,
    functions,
    actions: [],
    getters: [],
  };
}
