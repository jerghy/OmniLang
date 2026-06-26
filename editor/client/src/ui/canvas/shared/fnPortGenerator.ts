/** Auto-generate input/output ports from function source code.
 *  Used by RunCodeBlock and fn-call nodes to dynamically create ports
 *  matching function parameters and return types.
 *
 *  Supports two formats:
 *  1. OmniLang action syntax:  `action name(param: Type) { ... }`
 *  2. OmniLang fn syntax:      `fn name(param: Type) -> Type { ... }`
 */

import type { PortData } from "../../../core/types";

export interface FnSignature {
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType: string;
  kind: "action" | "fn";
}

/** Parse function name, params, and return type from OmniLang source code. */
export function parseFnSignature(source: string): FnSignature | null {
  const trimmed = source.trim();

  // Try action:  action name(event: Type) { ... }
  const actionMatch = trimmed.match(
    /^action\s+(\w+)\s*\(([^)]*)\)/,
  );
  if (actionMatch) {
    const name = actionMatch[1];
    const paramsStr = actionMatch[2]?.trim() || "";
    const params = parseParams(paramsStr);
    return { name, params, returnType: "void", kind: "action" };
  }

  // Try fn:  fn name(param: Type) -> Type { ... }
  const fnMatch = trimmed.match(
    /^fn\s+(\w+)\s*\(([^)]*)\)\s*->\s*(\w+)/,
  );
  if (fnMatch) {
    const name = fnMatch[1];
    const paramsStr = fnMatch[2]?.trim() || "";
    const returnType = fnMatch[3] || "any";
    const params = parseParams(paramsStr);
    return { name, params, returnType, kind: "fn" };
  }

  // Try just a name fallback
  const nameMatch = trimmed.match(/^(\w+)/);
  if (nameMatch) {
    return { name: nameMatch[1], params: [], returnType: "any", kind: "action" };
  }

  return null;
}

function parseParams(paramsStr: string): Array<{ name: string; type: string }> {
  if (!paramsStr.trim()) return [];
  const result: Array<{ name: string; type: string }> = [];
  for (const part of paramsStr.split(",")) {
    const [pName, pType] = part.split(":").map((s) => s.trim());
    if (pName) {
      result.push({ name: pName, type: pType || "any" });
    }
  }
  return result;
}

/** Generate input ports from function parameters. */
export function generateInputPorts(params: Array<{ name: string; type: string }>): PortData[] {
  return params.map((p) => ({
    name: p.name,
    direction: "in" as const,
    portType: "data" as const,
    type: p.type,
    label: `${p.name}:${p.type}`,
  }));
}

/** Generate output ports from function return type. */
export function generateOutputPorts(returnType: string, kind: "action" | "fn" = "action"): PortData[] {
  const ports: PortData[] = [];

  // Flow output (action nodes drive execution flow)
  if (kind === "action") {
    ports.push({
      name: "flowOut",
      direction: "out",
      portType: "flow",
      type: "void",
      label: "→",
    });
  }

  // Data output
  if (returnType !== "void") {
    ports.push({
      name: "result",
      direction: "out",
      portType: "data",
      type: returnType,
      label: `→ ${returnType}`,
    });
  }

  return ports;
}

/** Generate all ports for a RunCodeBlock node from its source code. */
export function generateRunCodeBlockPorts(source: string): PortData[] {
  const sig = parseFnSignature(source);
  if (!sig) return [];

  const ports: PortData[] = [];

  // Flow input
  ports.push({
    name: "flowIn",
    direction: "in",
    portType: "flow",
    type: "void",
    label: "→",
  });

  // Data inputs from params
  for (const p of generateInputPorts(sig.params)) {
    ports.push(p);
  }

  // Outputs
  for (const p of generateOutputPorts(sig.returnType, sig.kind)) {
    ports.push(p);
  }

  return ports;
}
