/** OmniDriver type definitions.
 *  Playwright-like API for programmatic editor control.
 *  Exposed as `window.__OMNI__` for AI-driven automation. */

import type { ElementData, WireData } from "../core/types";
import type { EventMeta } from "../components/panels/ComponentPalette";

// ── Re-export store types ────────────────────────────

export type { ElementData, WireData, EventMeta };

// ── Port descriptor ──────────────────────────────────

export interface PortDescriptor {
  name: string;
  direction: "in" | "out";
  portType?: "flow" | "data";
  type: string;
  label?: string;
}

// ── Element query ────────────────────────────────────

export interface ElementQuery {
  tag?: string;
  nodeType?: "view" | "codeblock" | "group";
  props?: Record<string, unknown>;
  text?: string;
}

// ── Code block options ───────────────────────────────

export interface CodeBlockOptions {
  parentStore?: string;
  source?: string;
  ports?: PortDescriptor[];
  typeAnnotation?: string;
  isFolded?: boolean;
}

// ── Compile result ───────────────────────────────────

export interface CompileResult {
  success: boolean;
  errors: Array<{ code: string; message: string; line: number; severity: string }>;
  output?: string;
}

// ── Editor snapshot (full state dump) ────────────────

export interface EditorSnapshot {
  mode: "design" | "code";
  elements: ElementData[];
  wires: WireData[];
  selectedElementIds: string[];
  projectDir: string | null;
  activeFilePath: string | null;
  compileErrors: Array<{ code: string; message: string; line: number }>;
  previewUrl: string | null;
}

// ── Component protocol entry (for AI discovery) ─────

export interface ComponentInfo {
  tag: string;
  category: string;
  description: string;
  props: Record<string, { type: string; default?: unknown; values?: string[]; editor: string }>;
  events: Record<string, EventMeta>;
  styleProperties: string[];
  hasChildren: boolean;
}

// ── Event handler creation ───────────────────────────

export interface EventHandlerOptions {
  /** Human-readable handler name (auto-generated if omitted) */
  handlerName?: string;
  /** Handler source code (auto-generated if omitted) */
  source?: string;
  /** Mark handler as folded by default */
  folded?: boolean;
}

// ── Validation report (canvas health check) ────────────

export interface ValidationReport {
  /** View elements without a parent when multiple root elements exist */
  orphanElements: string[];
  /** View elements with defined events that have no wire connected */
  unwiredEvents: Array<{
    elementId: string;
    tag: string;
    eventName: string;
    eventPortId: string;
  }>;
  /** Wires connecting ports of incompatible types */
  typeMismatches: Array<{
    wireId: string;
    fromPort: string;
    fromType: string;
    toPort: string;
    toType: string;
  }>;
  /** Code blocks that have no inbound data wire */
  unreachableBlocks: string[];
  /** Whether the canvas passes all health checks */
  isValid: boolean;
}

// ── State diff (change tracking between snapshots) ────

export interface StateDiff {
  /** Elements that were added */
  added: import("../core/types").ElementData[];
  /** IDs of elements that were removed */
  removed: string[];
  /** Elements that were modified, with changed properties */
  modified: Array<{
    id: string;
    changes: Record<string, { from: unknown; to: unknown }>;
  }>;
  /** Wires that were added */
  addedWires: import("../core/types").WireData[];
  /** IDs of wires that were removed */
  removedWires: string[];
}

// ── v2: Function parameter ────────────────────────────

export interface FnParam {
  name: string;
  type: string;
  default?: unknown;
}

// ── v2: Event binding ─────────────────────────────────

export interface ParamBinding {
  sourceType: "self" | "literal" | "event" | "global";
  value: string;
  expectedType: string;
}

export interface EventBindingOptions {
  params?: Record<string, ParamBinding>;
}

export interface EventBindingRecord {
  elementId: string;
  tag: string;
  eventName: string;
  fnName: string;
  tabId: string;
  paramBindings: Record<string, ParamBinding>;
}

// ── v2: Code tab info ─────────────────────────────────

export interface TabInfo {
  id: string;
  type: "event" | "logic";
  label: string;
  icon: string;
  variableCount: number;
  elementCount: number;
}

// ── v2: Function validation ───────────────────────────

export interface FunctionValidationReport {
  valid: boolean;
  returnType: string;
  errors: string[];
  reachableReturnPaths: number;
}

export interface FunctionSignature {
  fnName: string;
  params: FnParam[];
  returnType: string;
  bodyNodeCount: number;
}

// ── v2: Tab export format ─────────────────────────────

export interface TabExportData {
  version: string;
  exportedAt: string;
  tab: {
    type: "event" | "logic";
    label: string;
    icon: string;
    variables: Array<{ name: string; type: string; defaultValue?: unknown; description?: string }>;
    elements: import("../core/types").ElementData[];
    wires: import("../core/types").WireData[];
  };
}
