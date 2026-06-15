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

/** Tab type: "event" = event handler functions (appear in design event binding),
 *  "logic" = general logic code (not shown in event binding dropdown). */
export type CodeTabType = "event" | "logic";

export interface CodeTab {
  /** Unique tab identifier (e.g. "main", "tab-xxxx") */
  id: string;
  type: CodeTabType;
  label: string;
  icon: string;
  graph: CodeGraph;

  /** Variables (signals/stores) defined in this tab's scope */
  variables: SignalDefinition[];
}

// ── Custom Function Metadata ──

/** Metadata for a customFn node in the graph.
 *  Stored in the node's config field. */
export interface CustomFunctionMeta {
  fnName: string;
  fnParams: FnParam[];
  fnReturnType: string;
  description: string;
}

// ── Event Binding (Design → Code bridge) ──

/** An event binding: maps a component event to a custom function in a code tab. */
export interface EventBinding {
  /** Which event type (e.g. "onPress", "onChange") */
  eventType: string;
  /** Which code tab contains the target function */
  tabId: string;
  /** Name of the custom function to call */
  fnName: string;
  /** Parameter bindings: fnParamName → data source */
  paramBindings: Record<string, ParamBinding>;
}

/** A single parameter binding: maps a function parameter to a data source. */
export interface ParamBinding {
  /** Data source type */
  sourceType: "self" | "literal" | "event" | "global";
  /** For "self": signal name in component scope (e.g. "title")
   *  For "literal": the literal value string
   *  For "event": the event payload field (e.g. "event.value")
   *  For "global": global signal name (e.g. "app.theme") */
  value: string;
  /** Expected type of this parameter (from function signature) */
  expectedType: string;
}

// ── Clipboard ──

export interface ClipboardData {
  elements: ElementData[];
  wires: WireData[];
  /** Which tab the data was copied from */
  sourceTabId: string;
}

// ── Helpers ──

/** Create an empty code graph. */
export function emptyGraph(): CodeGraph {
  return { elements: [], wires: [] };
}

/** Create the default "Main" logic tab. */
export function createMainTab(): CodeTab {
  return {
    id: "main",
    type: "logic",
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
