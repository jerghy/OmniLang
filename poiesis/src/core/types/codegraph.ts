// core/types/codegraph.ts
// Code graph types for the visual programming canvas.

import type { ElementData } from "./element";
import type { WireData } from "./wire";

export type CodeTabType = "logic" | "event";

export interface CodeGraph {
  elements: ElementData[];
  wires: WireData[];
}

export interface CodeTabVariable {
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface CodeTab {
  id: string;
  type: CodeTabType;
  label: string;
  icon: string;
  graph: CodeGraph;
  variables: CodeTabVariable[];
}

export interface ChainBlock {
  functionName: string;
  nodeLines: string[];
  wireLines: string[];
}
