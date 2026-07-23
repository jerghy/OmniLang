// Code canvas node flow types

/** The four code canvas node types */
export type FlowNodeType = "function-header" | "skeleton" | "code-block" | "return";

/** Control flow skeleton variants */
export type SkeletonType = "if" | "else" | "for" | "while";

export interface FlowPortConfig {
  direction: "in" | "out";
  position: "top" | "bottom" | "left" | "right";
  portType: "flow" | "data";
  label?: string;
}

/** Per-node-type flow port configuration */
const FLOW_NODE_PORTS: Record<FlowNodeType, FlowPortConfig[]> = {
  "function-header": [
    { direction: "out", position: "bottom", portType: "flow", label: "entry" },
    { direction: "out", position: "right", portType: "data", label: "params" },
  ],
  "skeleton": [
    { direction: "in", position: "top", portType: "flow" },
    { direction: "out", position: "bottom", portType: "flow", label: "true" },
    { direction: "out", position: "bottom", portType: "flow", label: "false" },
    { direction: "in", position: "left", portType: "data", label: "condition" },
  ],
  "code-block": [
    { direction: "in", position: "top", portType: "flow" },
    { direction: "out", position: "bottom", portType: "flow" },
    { direction: "in", position: "left", portType: "data" },
    { direction: "out", position: "right", portType: "data" },
  ],
  "return": [
    { direction: "in", position: "top", portType: "flow" },
    { direction: "in", position: "left", portType: "data", label: "value" },
  ],
};

/** Node display colors */
const FLOW_NODE_COLORS: Record<FlowNodeType, { bg: string; border: string }> = {
  "function-header": { bg: "var(--ol-color-primary-subtle)", border: "var(--ol-color-primary)" },
  "skeleton":        { bg: "var(--ol-color-warning-subtle)", border: "var(--ol-color-warning)" },
  "code-block":      { bg: "#F8FAFC", border: "var(--ol-text-muted)" },
  "return":          { bg: "var(--ol-color-success-subtle)", border: "var(--ol-color-success)" },
};
