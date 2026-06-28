/** CodeGraph — independent visual programming canvas abstraction.
 *  Each CodeTab owns one CodeGraph. The active tab's graph is rendered by FlowCanvas.
 *
 *  Core types (CodeGraph, CodeTab, CodeTabType, CodeTabVariable) are now in
 *  core/types/ — re-exported from there.  Module-specific types and helpers
 *  remain here. */

import type { ElementData, WireData } from "../../core/types";
import type { CodeGraph, CodeTab } from "../../core/types";
import { uid } from "../../core/utils";

// ── Re-exports from core/types (migrated types) ──
export type { CodeGraph, CodeTab, CodeTabType, CodeTabVariable } from "../../core/types";

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

// ── CodeTab (legacy re-export alias, prefer CodeTabVariable from core/types) ──
/** @deprecated Use CodeTabVariable from core/types */
export type { CodeTabVariable as SignalDefinition_v2 } from "../../core/types";

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
export function newTabId(): string {
  return uid("tab");
}
