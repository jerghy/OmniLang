/** CodeGraph — independent visual programming canvas abstraction.
 *  Each CodeTab owns one CodeGraph. The active tab's graph is rendered by FlowCanvas. */

import type { ElementData, WireData } from "../store/editor";

// ── CodeGraph ──

/** A single visual programming canvas (elements + wires). */
export interface CodeGraph {
  elements: ElementData[];
  wires: WireData[];
}

// ── Signal Definition ──

/** A named variable (signal or store) within a tab's scope. */
export interface SignalDefinition {
  name: string;
  type: string;
  defaultValue?: unknown;
  description?: string;
}

// ── Function Parameter ──

/** A parameter on a function tab. */
export interface FnParam {
  name: string;
  type: string;
  default?: unknown;
}

// ── CodeTab ──

export type CodeTabType = "freeform" | "component" | "function";

export interface CodeTab {
  /** Unique tab identifier (e.g. "main", "comp:MyButton", "fn:validate") */
  id: string;
  type: CodeTabType;
  label: string;
  icon: string;
  graph: CodeGraph;

  /** For type="component": which component definition this tab belongs to */
  componentId?: string;

  /** For type="function": function signature metadata */
  fnName?: string;
  fnParams?: FnParam[];
  fnReturnType?: string;

  /** Variables (signals/stores) defined in this tab's scope */
  variables: SignalDefinition[];
}

// ── Clipboard ──

export interface ClipboardData {
  elements: ElementData[];
  wires: WireData[];
  /** Which tab the data was copied from (for cross-tab awareness) */
  sourceTabId: string;
}

// ── Helpers ──

/** Create an empty code graph. */
export function emptyGraph(): CodeGraph {
  return { elements: [], wires: [] };
}

/** Create the default "Main" freeform tab. */
export function createMainTab(): CodeTab {
  return {
    id: "main",
    type: "freeform",
    label: "Main",
    icon: "📄",
    graph: emptyGraph(),
    variables: [],
  };
}

/** Generate a unique tab id. */
let _tabUid = 0;
export function newTabId(): string {
  return `tab-${Date.now().toString(36)}-${(_tabUid++).toString(36)}`;
}
