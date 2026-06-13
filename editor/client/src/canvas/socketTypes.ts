/** Socket type compatibility rules for OmniLang wire canvas.
 *  Rete.js Socket has no built-in `isCompatibleWith` — we add it here
 *  and validate in FlowCanvas's `connectioncreate` pipe. */

/** Type compatibility matrix:
 *  - `any` is compatible with everything (both directions)
 *  - Numeric types (i32, f64) are mutually compatible
 *  - `bool` is only compatible with `bool` and `any`
 *  - `String` is only compatible with `String` and `any`
 *  - `void` is only compatible with `void` and `any` (event trigger, no data)
 *  - `MouseEvent` is only compatible with `MouseEvent` and `any` */
const TYPE_COMPAT: Record<string, string[]> = {
  any:        ["any", "i32", "f64", "bool", "String", "void", "MouseEvent"],
  i32:        ["i32", "f64", "any"],
  f64:        ["f64", "i32", "any"],
  bool:       ["bool", "any"],
  String:     ["String", "any"],
  void:       ["void", "any"],
  MouseEvent: ["MouseEvent", "any"],
};

/** Check whether a connection from `sourceType` to `targetType` is valid. */
export function isTypeCompatible(sourceType: string, targetType: string): boolean {
  if (sourceType === targetType) return true;
  return (TYPE_COMPAT[sourceType] || []).includes(targetType);
}

/** Human-readable error message for incompatible types. */
export function typeMismatchMessage(sourceType: string, targetType: string): string {
  return `类型不兼容：输出 "${sourceType}" 不能连接到输入 "${targetType}"`;
}

/** Color map for socket types (shared across canvas + popup). */
export const TYPE_COLORS: Record<string, string> = {
  any: "#6B7280",
  i32: "#2563EB",
  f64: "#059669",
  bool: "#D97706",
  String: "#DB2777",
  void: "#9CA3AF",
  MouseEvent: "#DC2626",
};

/** Event port color — used to distinguish event triggers from data ports. */
export const EVENT_PORT_COLOR = "#DC2626";
