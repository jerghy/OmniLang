// Connection validators — v2 stub.

export interface ValidationIssue {
  severity: "error" | "warning";
  code: string;
  message: string;
}

export function canConnect(_from: unknown, _to: unknown): boolean {
  return true;
}

export function validateConnection(
  _fromEl: unknown,
  _fromPort: unknown,
  _toEl: unknown,
  _toPort: unknown,
  _wires: unknown[],
  _fromPorts: unknown[],
  _toPorts: unknown[],
): ValidationIssue[] {
  return [];
}

export function getAllPorts(_element: unknown): unknown[] {
  return [];
}
