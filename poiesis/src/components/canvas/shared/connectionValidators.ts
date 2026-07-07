import type { ElementData, PortData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

export interface ValidationIssue {
  severity: "error" | "warning";
  code: string;
  message: string;
}

export function canConnect(from: ElementData, to: ElementData): boolean {
  const fromPorts = ("ports" in from ? from.ports : []) ?? [];
  const toPorts = ("ports" in to ? to.ports : []) ?? [];
  return fromPorts.some((p: PortData) => p.direction === "out") && toPorts.some((p: PortData) => p.direction === "in");
}

export function validateConnection(
  fromEl: ElementData,
  fromPort: string,
  toEl: ElementData,
  toPort: string,
  wires: WireData[],
  fromPorts: PortData[],
  toPorts: PortData[],
): ValidationIssue[] {
  const issues: ValidationIssue[] = [];

  // Check ports exist
  const fromP = fromPorts.find((p) => p.name === fromPort);
  const toP = toPorts.find((p) => p.name === toPort);
  if (!fromP) {
    issues.push({
      severity: "error",
      code: "PORT_NOT_FOUND",
      message: `Port '${fromPort}' not found on ${fromEl.id}`,
    });
  }
  if (!toP) {
    issues.push({
      severity: "error",
      code: "PORT_NOT_FOUND",
      message: `Port '${toPort}' not found on ${toEl.id}`,
    });
  }
  if (!fromP || !toP) return issues;

  // Check directions
  if (fromP.direction !== "out") {
    issues.push({
      severity: "error",
      code: "WRONG_DIRECTION",
      message: `Port '${fromPort}' is not an output port`,
    });
  }
  if (toP.direction !== "in") {
    issues.push({
      severity: "error",
      code: "WRONG_DIRECTION",
      message: `Port '${toPort}' is not an input port`,
    });
  }

  // Check duplicates
  const isDuplicate = wires.some(
    (w) =>
      w.fromId === fromEl.id &&
      w.fromPort === fromPort &&
      w.toId === toEl.id &&
      w.toPort === toPort,
  );
  if (isDuplicate) {
    issues.push({
      severity: "error",
      code: "DUPLICATE",
      message: `Wire already exists: ${fromEl.id}.${fromPort} → ${toEl.id}.${toPort}`,
    });
  }

  // Check port type compatibility
  if (fromP.portType === "data" && toP.portType === "data" && fromP.type !== toP.type) {
    issues.push({
      severity: "warning",
      code: "TYPE_MISMATCH",
      message: `Port type mismatch: '${fromP.type}' → '${toP.type}'`,
    });
  }

  return issues;
}

export function getAllPorts(element: ElementData): PortData[] {
  return ("ports" in element ? element.ports : []) ?? [];
}
